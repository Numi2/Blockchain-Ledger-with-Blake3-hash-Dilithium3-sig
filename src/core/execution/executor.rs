use crate::core::block::Transaction;
use crate::core::execution::{ExecutionContext, ExecutionResult, VM, VMError};
use crate::core::state::WorldState;
use crate::types::{Address, ExecutionStatus, Hash};

/// TransactionExecutor handles the execution of transactions against the world state
pub struct TransactionExecutor {
    /// The current world state
    pub state: WorldState,
}

impl TransactionExecutor {
    /// Create a new transaction executor with the given state
    pub fn new(state: WorldState) -> Self {
        Self { state }
    }
    
    /// Execute a transaction against the current state
    pub fn execute_transaction(&mut self, tx: &Transaction) -> (ExecutionStatus, u64) {
        // First apply basic transaction validation
        let sender = tx.from;
        let sender_account = self.state.get_or_create_account(&sender);
        
        // Check nonce
        if sender_account.nonce != tx.nonce {
            return (ExecutionStatus::InvalidInput, 0);
        }
        
        // Check balance for value + gas cost
        let gas_cost = tx.gas_limit * tx.gas_price;
        let total_cost = tx.value + gas_cost;
        if sender_account.balance < total_cost {
            return (ExecutionStatus::OutOfGas, 0);
        }
        
        // Increment nonce and deduct gas cost from sender (will refund unused later)
        sender_account.nonce += 1;
        sender_account.balance -= gas_cost;
        
        // Determine if this is a contract creation or a contract call
        let (status, gas_used) = if tx.to.is_none() {
            // Contract creation
            self.execute_contract_creation(tx)
        } else {
            // Contract call
            self.execute_contract_call(tx)
        };
        
        // Process refunds and final balance transfers
        let refund_amount = if status == ExecutionStatus::Success {
            // Refund unused gas
            let unused_gas = tx.gas_limit - gas_used;
            let refund = unused_gas * tx.gas_price;
            
            // Transfer value to recipient if the call was successful
            if let Some(to) = tx.to {
                let recipient = self.state.get_or_create_account(&to);
                recipient.balance += tx.value;
            }
            
            refund
        } else {
            // Refund all gas on failure, but keep the tx fee
            gas_cost
        };
        
        // Apply refund to sender
        let sender_account = self.state.get_or_create_account(&sender);
        sender_account.balance += refund_amount;
        
        (status, gas_used)
    }
    
    /// Execute a contract creation transaction
    fn execute_contract_creation(&mut self, tx: &Transaction) -> (ExecutionStatus, u64) {
        // Create a new address for the contract
        // In a real implementation, this would be derived from sender and nonce
        let contract_address = [0u8; 20]; // Placeholder
        
        // Set up execution context
        let context = ExecutionContext {
            address: contract_address,
            caller: tx.from,
            value: tx.value,
            input: Vec::new(), // No input for creation
            gas_price: tx.gas_price,
        };
        
        // Initialize VM with the transaction data as code
        let mut vm = VM::new(tx.data.clone(), tx.gas_limit, context);
        
        // Execute the contract creation code
        let result = vm.execute();
        
        if result.success {
            // Create the contract account
            let contract = self.state.get_or_create_account(&contract_address);
            contract.code = result.output.clone();
            
            (ExecutionStatus::Success, result.gas_used)
        } else {
            // Contract creation failed
            match result.error {
                Some(VMError::OutOfGas) => (ExecutionStatus::OutOfGas, tx.gas_limit),
                _ => (ExecutionStatus::Failure, result.gas_used),
            }
        }
    }
    
    /// Execute a contract call transaction
    fn execute_contract_call(&mut self, tx: &Transaction) -> (ExecutionStatus, u64) {
        // Get the recipient address
        let recipient = tx.to.unwrap();
        
        // Get the recipient account
        let recipient_account = self.state.get_or_create_account(&recipient);
        
        // Set up execution context
        let context = ExecutionContext {
            address: recipient,
            caller: tx.from,
            value: tx.value,
            input: tx.data.clone(),
            gas_price: tx.gas_price,
        };
        
        // Initialize VM with the contract code
        let mut vm = VM::new(recipient_account.code.clone(), tx.gas_limit, context);
        
        // Execute the contract code
        let result = vm.execute();
        
        if result.success {
            (ExecutionStatus::Success, result.gas_used)
        } else {
            // Contract call failed
            match result.error {
                Some(VMError::OutOfGas) => (ExecutionStatus::OutOfGas, tx.gas_limit),
                _ => (ExecutionStatus::Failure, result.gas_used),
            }
        }
    }
} 