use crate::types::{Address, Hash};
use blake3;
use serde::{Serialize, Deserialize};

/// A blockchain transaction
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Transaction {
    /// Sender address
    pub from: Address,
    
    /// Recipient address
    pub to: Option<Address>,
    
    /// Transaction nonce (prevents replay attacks)
    pub nonce: u64,
    
    /// Transaction value (amount of tokens)
    pub value: u64,
    
    /// Gas limit for the transaction
    pub gas_limit: u64,
    
    /// Gas price in smallest token unit
    pub gas_price: u64,
    
    /// Transaction data (for contract interactions)
    pub data: Vec<u8>,
}

impl Transaction {
    /// Calculate the transaction hash
    pub fn hash(&self) -> Hash {
        // Serialize and hash the transaction
        let tx_bytes = serde_json::to_vec(self).unwrap_or_default();
        
        // Use BLAKE3 for hashing
        let hash = blake3::hash(&tx_bytes);
        
        // Convert to a fixed-size array
        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(hash.as_bytes());
        
        tx_hash
    }
    
    /// Calculate the gas cost of the transaction
    pub fn gas_cost(&self) -> u64 {
        self.gas_limit * self.gas_price
    }
    
    /// Create a new transaction
    pub fn new(
        from: Address,
        to: Option<Address>,
        value: u64,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
        data: Vec<u8>,
    ) -> Self {
        Self {
            from,
            to,
            nonce,
            value,
            gas_limit,
            gas_price,
            data,
        }
    }
    
    /// Check if this is a contract creation transaction
    pub fn is_contract_creation(&self) -> bool {
        self.to.is_none() && !self.data.is_empty()
    }
} 