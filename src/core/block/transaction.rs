use crate::types::{Address, Hash, Signature};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Transaction represents a state transition in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender's address
    pub from: Address,
    
    /// Receiver's address (None for contract creation)
    pub to: Option<Address>,
    
    /// Transaction nonce (to prevent replay attacks)
    pub nonce: u64,
    
    /// Transaction data/payload
    pub data: Vec<u8>,
    
    /// Gas limit for execution
    pub gas_limit: u64,
    
    /// Gas price the sender is willing to pay
    pub gas_price: u64,
    
    /// Value to transfer
    pub value: u64,
    
    /// Signature of the transaction
    pub signature: Signature,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        from: Address,
        to: Option<Address>,
        nonce: u64,
        data: Vec<u8>,
        gas_limit: u64,
        gas_price: u64,
        value: u64,
        signature: Signature,
    ) -> Self {
        Self {
            from,
            to,
            nonce,
            data,
            gas_limit,
            gas_price,
            value,
            signature,
        }
    }

    /// Compute the hash of this transaction using BLAKE3
    pub fn hash(&self) -> Hash {
        let mut hasher = Hasher::new();
        
        // Hash all transaction fields
        hasher.update(&self.from);
        
        // Hash optional to address
        if let Some(to) = self.to {
            hasher.update(&[1u8]); // Flag indicating presence
            hasher.update(&to);
        } else {
            hasher.update(&[0u8]); // Flag indicating absence
        }
        
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.data);
        hasher.update(&self.gas_limit.to_le_bytes());
        hasher.update(&self.gas_price.to_le_bytes());
        hasher.update(&self.value.to_le_bytes());
        
        // Note: We typically don't hash the signature as it's added after tx creation
        
        // Get the hash result
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
} 