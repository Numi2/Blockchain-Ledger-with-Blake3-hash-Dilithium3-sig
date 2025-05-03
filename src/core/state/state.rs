use crate::types::{Address, Hash, Transaction, ExecutionStatus, Slot};
use crate::core::state::utxo::{Utxo, TxInput, TxOutput, UtxoTransaction};
use crate::core::state::merkle::{MerkleTree};
use std::collections::HashMap;
use crate::core::block::{Block, Transaction as BlockTransaction};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Represents the UTXO set and manages state transitions
pub struct UtxoSet {
    utxos: HashMap<(Hash, u32), Utxo>,
    merkle_tree: MerkleTree,
}

impl UtxoSet {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            merkle_tree: MerkleTree::new(),
        }
    }

    /// Apply a transaction to the current state. Returns true if successful.
    pub fn apply_transaction(&mut self, tx: &UtxoTransaction) -> bool {
        // Validate all inputs exist and sum >= sum of outputs
        let mut input_sum = 0u64;
        for inp in &tx.inputs {
            if let Some(utxo) = self.utxos.remove(&(inp.utxo_ref.tx_hash, inp.utxo_ref.output_index)) {
                input_sum += utxo.value;
                // In a real implementation, would verify signatures here
            } else {
                return false; // Double spend or invalid input
            }
        }
        
        let output_sum: u64 = tx.outputs.iter().map(|o| o.value).sum();
        if input_sum < output_sum {
            return false; // Insufficient input value
        }
        
        // Add outputs as new UTXOs
        for (i, out) in tx.outputs.iter().enumerate() {
            let tx_hash = tx.hash(); // This is not implemented but would be in the real tx
            
            let utxo = Utxo {
                tx_hash,
                output_index: i as u32,
                value: out.value,
                address: out.address,
                locktime: out.locktime,
            };
            
            let key = (tx_hash, i as u32);
            self.utxos.insert(key, utxo);
        }
        
        // In a real implementation, would update the Merkle tree here
        
        true
    }

    /// Return state root (Merkle root over the UTXO set).
    pub fn state_root(&self) -> Hash {
        // In a real implementation, would compute actual Merkle root
        // For now, return empty hash
        [0u8; 32]
    }
}

/// AccountState represents the state of an account in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Account balance
    pub balance: u64,
    
    /// Account nonce (for preventing replay attacks)
    pub nonce: u64,
    
    /// Account code (for smart contracts)
    pub code: Vec<u8>,
    
    /// Account storage (key-value store for contracts)
    pub storage: HashMap<Hash, Hash>,
}

impl Account {
    /// Create a new empty account
    pub fn new() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            code: Vec::new(),
            storage: HashMap::new(),
        }
    }

    /// Create a new account with an initial balance
    pub fn with_balance(balance: u64) -> Self {
        let mut account = Self::new();
        account.balance = balance;
        account
    }

    /// Compute the hash of this account state
    pub fn hash(&self) -> Hash {
        let mut hasher = Hasher::new();
        
        // Hash account balance and nonce
        hasher.update(&self.balance.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        
        // Hash the code
        hasher.update(&(self.code.len() as u64).to_le_bytes());
        hasher.update(&self.code);
        
        // Hash the storage - first sort keys for determinism
        let mut storage_keys: Vec<_> = self.storage.keys().collect();
        storage_keys.sort();
        
        for key in storage_keys {
            hasher.update(key);
            hasher.update(self.storage.get(key).unwrap());
        }
        
        // Get the hash result
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
}

/// WorldState represents the entire state of the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Map of addresses to account states
    pub accounts: HashMap<Address, Account>,
    
    /// Current slot
    pub slot: Slot,
    
    /// Current block hash
    pub latest_block_hash: Hash,
}

impl WorldState {
    /// Create a new empty world state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            slot: 0,
            latest_block_hash: [0u8; 32],
        }
    }

    /// Get an account state (creates new if doesn't exist)
    pub fn get_or_create_account(&mut self, address: &Address) -> &mut Account {
        if !self.accounts.contains_key(address) {
            self.accounts.insert(*address, Account::new());
        }
        self.accounts.get_mut(address).unwrap()
    }

    /// Apply a transaction to the world state
    pub fn apply_transaction(&mut self, tx: &Transaction) -> ExecutionStatus {
        // First, check the sender has enough balance and valid nonce
        let sender = tx.from;
        let sender_account = self.get_or_create_account(&sender);
        
        // Check nonce
        if sender_account.nonce != tx.nonce {
            return ExecutionStatus::InvalidOperation;
        }
        
        // Check balance (including gas fees)
        let total_cost = tx.value + tx.gas_limit * tx.gas_price;
        if sender_account.balance < total_cost {
            return ExecutionStatus::OutOfGas;
        }
        
        // Deduct gas fees (these are charged regardless of transaction success)
        sender_account.balance -= tx.gas_limit * tx.gas_price;
        
        // Increment sender's nonce
        sender_account.nonce += 1;
        
        // Deduct value from sender
        if sender_account.balance < tx.value {
            return ExecutionStatus::OutOfGas;
        }
        sender_account.balance -= tx.value;
        
        // If there's a recipient, add value to their account
        if let Some(to) = tx.to {
            let recipient_account = self.get_or_create_account(&to);
            recipient_account.balance += tx.value;
        }
        
        // TODO: Handle contract creation and execution
        // For now, we'll just return success
        ExecutionStatus::Success
    }

    /// Apply a block to the world state
    pub fn apply_block(&mut self, block: &Block) -> ExecutionStatus {
        // Update slot and latest block hash
        self.slot = block.header.slot;
        self.latest_block_hash = block.hash();
        
        // Apply all transactions
        for tx in &block.body.transactions {
            let status = self.apply_transaction(tx);
            if status != ExecutionStatus::Success {
                return status;
            }
        }
        
        ExecutionStatus::Success
    }

    /// Compute the state root hash
    pub fn state_root(&self) -> Hash {
        let mut hasher = Hasher::new();
        
        // Sort addresses for deterministic hashing
        let mut addresses: Vec<_> = self.accounts.keys().collect();
        addresses.sort();
        
        // Hash each account state
        for addr in addresses {
            hasher.update(addr);
            let account = &self.accounts[addr];
            hasher.update(&account.hash());
        }
        
        // Also include the current slot and latest block hash
        hasher.update(&self.slot.to_le_bytes());
        hasher.update(&self.latest_block_hash);
        
        // Get the hash result
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
}

/// StateRoot represents the root hash of the world state tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRoot(pub Hash);

/// WorldStateSnapshot represents a snapshot of the world state at a specific point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    /// Root hash of the world state
    pub root: StateRoot,
    
    /// Block height of this snapshot
    pub height: u64,
    
    /// Timestamp of this snapshot
    pub timestamp: u64,
}

/// StateChange represents a change to the world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChange {
    /// Create a new account
    CreateAccount {
        /// Account address
        address: Address,
        
        /// Initial balance
        balance: u64,
    },
    
    /// Delete an account
    DeleteAccount {
        /// Account address
        address: Address,
    },
    
    /// Update account balance
    UpdateBalance {
        /// Account address
        address: Address,
        
        /// Amount to add (positive) or subtract (negative)
        amount: i64,
    },
    
    /// Update account code
    UpdateCode {
        /// Account address
        address: Address,
        
        /// New code
        code: Vec<u8>,
    },
    
    /// Update account storage
    UpdateStorage {
        /// Account address
        address: Address,
        
        /// Storage key
        key: Hash,
        
        /// Storage value
        value: Hash,
    },
}

/// StateDiff represents a set of changes to the world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// List of changes
    pub changes: Vec<StateChange>,
    
    /// Previous state root
    pub previous_root: StateRoot,
    
    /// New state root after applying changes
    pub new_root: StateRoot,
}

/// StateTransition represents the transition from one state to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Transaction that caused this transition
    pub transaction: Transaction,
    
    /// State changes
    pub diff: StateDiff,
}

/// StateTree manages the blockchain state
pub struct StateTree {
    /// Current state root
    pub root: StateRoot,
    
    /// History of state snapshots
    snapshots: Vec<WorldStateSnapshot>,
}

impl StateTree {
    /// Create a new empty state tree
    pub fn new() -> Self {
        Self {
            root: StateRoot([0u8; 32]),
            snapshots: Vec::new(),
        }
    }
    
    /// Apply a state transition
    pub fn apply_transition(&mut self, transition: &StateTransition) -> bool {
        // Verify previous root matches current root
        if transition.diff.previous_root != self.root {
            return false;
        }
        
        // In a real implementation, we would actually apply the changes
        // For now, we just update the root
        self.root = transition.diff.new_root;
        
        // Save a snapshot
        self.snapshots.push(WorldStateSnapshot {
            root: self.root,
            height: self.snapshots.len() as u64,
            timestamp: current_timestamp(),
        });
        
        true
    }
    
    /// Get a state snapshot by height
    pub fn get_snapshot(&self, height: u64) -> Option<&WorldStateSnapshot> {
        self.snapshots.get(height as usize)
    }
    
    /// Get the latest state snapshot
    pub fn latest_snapshot(&self) -> Option<&WorldStateSnapshot> {
        self.snapshots.last()
    }
}

/// Calculate a state root hash from a set of key-value pairs
pub fn calculate_state_root(state_pairs: &[(Vec<u8>, Vec<u8>)]) -> Hash {
    // In a real implementation, this would build a Merkle Patricia Trie
    // For now, we'll just hash the concatenation of all key-value pairs
    
    let mut hasher = Hasher::new();
    
    for (key, value) in state_pairs {
        hasher.update(key);
        hasher.update(value);
    }
    
    let hash = hasher.finalize();
    let mut root = [0u8; 32];
    root.copy_from_slice(hash.as_bytes());
    
    root
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
