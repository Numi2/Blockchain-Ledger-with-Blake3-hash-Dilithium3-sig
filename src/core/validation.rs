use crate::types::{Transaction, Hash, Address, TxIn, TxOut};
use crate::core::state::utxo::{UTXOSet, UTXOError};
use thiserror::Error;

/// Transaction validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid transaction format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Insufficient funds")]
    InsufficientFunds,
    
    #[error("Double spend detected")]
    DoubleSpend,
    
    #[error("Invalid UTXO reference")]
    InvalidUTXO,
    
    #[error("Transaction too large")]
    TooLarge,
    
    #[error("Gas price too low")]
    GasPriceTooLow,
    
    #[error("Nonce mismatch")]
    NonceMismatch,
    
    #[error("Invalid recipient")]
    InvalidRecipient,
    
    #[error("UTXO error: {0}")]
    UTXOError(#[from] UTXOError),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Transaction validator
pub struct TransactionValidator {
    /// Minimum gas price
    min_gas_price: u64,
    
    /// Maximum transaction size in bytes
    max_tx_size: usize,
    
    /// Current block height
    current_height: u64,
}

impl TransactionValidator {
    /// Create a new transaction validator
    pub fn new(min_gas_price: u64, max_tx_size: usize, current_height: u64) -> Self {
        Self {
            min_gas_price,
            max_tx_size,
            current_height,
        }
    }
    
    /// Validate a transaction without state context
    pub fn validate_transaction_format(&self, tx: &Transaction) -> ValidationResult<()> {
        // Check transaction size
        let tx_size = bincode::serialized_size(tx)
            .map_err(|e| ValidationError::Other(format!("Serialization error: {}", e)))?;
            
        if tx_size > self.max_tx_size as u64 {
            return Err(ValidationError::TooLarge);
        }
        
        // Check gas price
        if tx.gas_price < self.min_gas_price {
            return Err(ValidationError::GasPriceTooLow);
        }
        
        // Check inputs and outputs are not empty
        if tx.inputs.is_empty() {
            return Err(ValidationError::InvalidFormat("No inputs".to_string()));
        }
        
        if tx.outputs.is_empty() && tx.to.is_none() && tx.data.is_empty() {
            return Err(ValidationError::InvalidFormat("No outputs or recipient".to_string()));
        }
        
        // Check that the total output value is greater than zero
        let total_output = tx.outputs.iter().map(|o| o.value).sum::<u64>();
        if total_output == 0 && tx.value == 0 {
            return Err(ValidationError::InvalidFormat("Zero value transaction".to_string()));
        }
        
        // For contract creation (tx.to is None), data must not be empty
        if tx.to.is_none() && tx.data.is_empty() {
            return Err(ValidationError::InvalidFormat("Contract creation requires data".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate a transaction against the UTXO set
    pub fn validate_transaction_state(
        &self,
        tx: &Transaction,
        utxo_set: &UTXOSet,
        check_signature: bool,
    ) -> ValidationResult<u64> {
        // Validate format first
        self.validate_transaction_format(tx)?;
        
        // Check each input UTXO exists and is not spent
        let mut total_input = 0;
        
        for input in &tx.inputs {
            // Get UTXO
            let utxo = utxo_set.get_utxo(&input.utxo_id)
                .ok_or(ValidationError::InvalidUTXO)?;
            
            // Check if already spent
            if utxo.spent {
                return Err(ValidationError::DoubleSpend);
            }
            
            // Check ownership (sender must own all inputs)
            if utxo.owner != tx.from {
                return Err(ValidationError::InvalidSignature);
            }
            
            // Add to total input
            total_input += utxo.value;
        }
        
        // Check signature if required
        if check_signature {
            self.validate_signature(tx)?;
        }
        
        // Calculate total output
        let total_output = tx.outputs.iter().map(|o| o.value).sum::<u64>() + tx.value;
        
        // Check inputs >= outputs + fees
        let fee = tx.gas_limit * tx.gas_price;
        if total_input < total_output + fee {
            return Err(ValidationError::InsufficientFunds);
        }
        
        // Return the transaction fee
        Ok(fee)
    }
    
    /// Validate transaction signature
    pub fn validate_signature(&self, tx: &Transaction) -> ValidationResult<()> {
        // In a real implementation, check the signature based on transaction type
        // For Dilithium, verify using the quantum-resistant algorithm
        // For Ed25519, use standard elliptic curve verification
        
        // For now, we'll assume the signature is valid
        Ok(())
    }
    
    /// Update the current block height
    pub fn set_height(&mut self, height: u64) {
        self.current_height = height;
    }
    
    /// Set minimum gas price
    pub fn set_min_gas_price(&mut self, price: u64) {
        self.min_gas_price = price;
    }
    
    /// Set maximum transaction size
    pub fn set_max_tx_size(&mut self, size: usize) {
        self.max_tx_size = size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    
    // Generate a random transaction
    fn create_test_transaction(inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Transaction {
        let mut rng = ChaCha20Rng::seed_from_u64(12345);
        
        Transaction {
            from: [rng.gen(); 20],
            to: Some([rng.gen(); 20]),
            value: 100,
            gas_limit: 21000,
            gas_price: 1_000_000_000, // 1 gwei
            nonce: 0,
            data: Vec::new(),
            inputs,
            outputs,
        }
    }
    
    #[test]
    fn test_transaction_format_validation() {
        let validator = TransactionValidator::new(1_000_000_000, 1024 * 1024, 0);
        
        // Valid transaction
        let tx = create_test_transaction(
            vec![TxIn { utxo_id: [1u8; 32] }],
            vec![TxOut { value: 100, recipient: [2u8; 20] }],
        );
        
        assert!(validator.validate_transaction_format(&tx).is_ok());
        
        // Transaction with no inputs
        let invalid_tx = create_test_transaction(
            vec![],
            vec![TxOut { value: 100, recipient: [2u8; 20] }],
        );
        
        assert!(matches!(
            validator.validate_transaction_format(&invalid_tx),
            Err(ValidationError::InvalidFormat(_))
        ));
        
        // Transaction with no outputs
        let invalid_tx = create_test_transaction(
            vec![TxIn { utxo_id: [1u8; 32] }],
            vec![],
        );
        
        // This should fail because there's no recipient or outputs
        assert!(matches!(
            validator.validate_transaction_format(&invalid_tx),
            Err(ValidationError::InvalidFormat(_))
        ));
        
        // Transaction with gas price too low
        let mut low_gas_tx = tx.clone();
        low_gas_tx.gas_price = 1;
        
        assert!(matches!(
            validator.validate_transaction_format(&low_gas_tx),
            Err(ValidationError::GasPriceTooLow)
        ));
    }
} 