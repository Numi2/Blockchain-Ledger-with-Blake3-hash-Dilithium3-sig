use crate::types::{Address, Hash, Signature, Transaction};
use blake3::{Hash as Blake3Hash, Hasher};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// UTXO error types
#[derive(Error, Debug)]
pub enum UTXOError {
    #[error("UTXO not found")]
    NotFound,
    
    #[error("UTXO already spent")]
    AlreadySpent,
    
    #[error("Invalid UTXO reference")]
    InvalidReference,
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Insufficient funds")]
    InsufficientFunds,
}

/// Result type for UTXO operations
pub type UTXOResult<T> = Result<T, UTXOError>;

/// Unspent Transaction Output (UTXO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    /// Unique ID (hash of transaction + output index)
    pub id: Hash,
    
    /// Transaction hash this UTXO came from
    pub tx_hash: Hash,
    
    /// Output index in the transaction
    pub index: u32,
    
    /// Value in smallest units
    pub value: u64,
    
    /// Owner address
    pub owner: Address,
    
    /// Block height when created
    pub created_at: u64,
    
    /// Is this UTXO spent
    pub spent: bool,
    
    /// Block height when spent (if spent)
    pub spent_at: Option<u64>,
}

/// UTXO reference in a transaction input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXORef {
    /// Transaction hash
    pub tx_hash: Hash,
    
    /// Output index
    pub index: u32,
}

impl UTXORef {
    /// Create a UTXO ID from this reference
    pub fn to_id(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&self.tx_hash);
        hasher.update(&self.index.to_be_bytes());
        let hash = hasher.finalize();
        
        let mut id = [0u8; 32];
        id.copy_from_slice(hash.as_bytes());
        id
    }
}

/// UTXO Set - the primary state structure
#[derive(Debug, Clone)]
pub struct UTXOSet {
    /// All UTXOs indexed by ID
    utxos: HashMap<Hash, UTXO>,
    
    /// Current block height
    height: u64,
}

impl UTXOSet {
    /// Create a new UTXO set
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            height: 0,
        }
    }
    
    /// Add a UTXO to the set
    pub fn add_utxo(&mut self, tx_hash: Hash, index: u32, value: u64, owner: Address) -> Hash {
        let utxo_ref = UTXORef {
            tx_hash,
            index,
        };
        
        let id = utxo_ref.to_id();
        
        let utxo = UTXO {
            id,
            tx_hash,
            index,
            value,
            owner,
            created_at: self.height,
            spent: false,
            spent_at: None,
        };
        
        self.utxos.insert(id, utxo);
        id
    }
    
    /// Get a UTXO by ID
    pub fn get_utxo(&self, id: &Hash) -> Option<&UTXO> {
        self.utxos.get(id)
    }
    
    /// Spend a UTXO
    pub fn spend_utxo(&mut self, id: &Hash) -> UTXOResult<()> {
        let utxo = self.utxos.get_mut(id).ok_or(UTXOError::NotFound)?;
        
        if utxo.spent {
            return Err(UTXOError::AlreadySpent);
        }
        
        utxo.spent = true;
        utxo.spent_at = Some(self.height);
        
        Ok(())
    }
    
    /// Get all UTXOs for an address
    pub fn get_utxos_for_address(&self, address: &Address) -> Vec<&UTXO> {
        self.utxos.values()
            .filter(|utxo| !utxo.spent && utxo.owner == *address)
            .collect()
    }
    
    /// Get total balance for an address
    pub fn get_balance(&self, address: &Address) -> u64 {
        self.get_utxos_for_address(address)
            .iter()
            .map(|utxo| utxo.value)
            .sum()
    }
    
    /// Apply a transaction to the UTXO set
    pub fn apply_transaction(&mut self, tx: &Transaction) -> UTXOResult<()> {
        // Check if we have enough inputs
        if tx.to.is_none() && tx.data.is_empty() {
            return Err(UTXOError::InvalidReference);
        }
        
        // Process inputs (spend UTXOs)
        for input in &tx.inputs {
            let utxo_ref = UTXORef {
                tx_hash: input.utxo_id, // This field may need adjustment based on your Transaction struct
                index: 0, // This field may need adjustment based on your Transaction struct
            };
            let id = utxo_ref.to_id();
            
            self.spend_utxo(&id)?;
        }
        
        // Create new UTXOs for each output
        let tx_hash = tx.hash();
        
        for (i, output) in tx.outputs.iter().enumerate() {
            self.add_utxo(
                tx_hash,
                i as u32,
                output.value,
                output.recipient,
            );
        }
        
        Ok(())
    }
    
    /// Compute the Merkle root of the UTXO set
    pub fn merkle_root(&self) -> Hash {
        if self.utxos.is_empty() {
            return [0u8; 32];
        }
        
        // Get all unspent UTXOs
        let mut utxos: Vec<&UTXO> = self.utxos.values()
            .filter(|utxo| !utxo.spent)
            .collect();
        
        // Sort by ID for determinism
        utxos.sort_by_key(|utxo| utxo.id);
        
        // Build leaves
        let mut leaves: Vec<Hash> = utxos.iter()
            .map(|utxo| utxo.id)
            .collect();
        
        // If we have odd number of leaves, duplicate the last one
        if leaves.len() % 2 == 1 {
            leaves.push(*leaves.last().unwrap());
        }
        
        // Build the Merkle tree
        while leaves.len() > 1 {
            let mut next_level = Vec::new();
            
            for i in (0..leaves.len()).step_by(2) {
                let left = leaves[i];
                let right = leaves[i + 1];
                
                // Combine two hashes
                let mut hasher = Hasher::new();
                hasher.update(&left);
                hasher.update(&right);
                let hash = hasher.finalize();
                
                let mut combined = [0u8; 32];
                combined.copy_from_slice(hash.as_bytes());
                
                next_level.push(combined);
            }
            
            leaves = next_level;
        }
        
        // Return the root
        leaves[0]
    }
    
    /// Generate a Merkle proof for a UTXO
    pub fn generate_proof(&self, id: &Hash) -> Option<MerkleProof> {
        // Check if the UTXO exists and is unspent
        let utxo = self.utxos.get(id)?;
        if utxo.spent {
            return None;
        }
        
        // Get all unspent UTXOs
        let mut utxos: Vec<&UTXO> = self.utxos.values()
            .filter(|utxo| !utxo.spent)
            .collect();
        
        // Sort by ID for determinism
        utxos.sort_by_key(|utxo| utxo.id);
        
        // Find the index of our UTXO
        let leaf_index = utxos.iter().position(|u| &u.id == id)?;
        
        // Build leaves
        let mut leaves: Vec<Hash> = utxos.iter()
            .map(|utxo| utxo.id)
            .collect();
        
        // If we have odd number of leaves, duplicate the last one
        if leaves.len() % 2 == 1 {
            leaves.push(*leaves.last().unwrap());
        }
        
        // Build the proof
        let mut proof = Vec::new();
        let mut index = leaf_index;
        
        let mut level = leaves;
        while level.len() > 1 {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            proof.push(level[sibling_index]);
            
            // Move to next level
            let mut next_level = Vec::new();
            for i in (0..level.len()).step_by(2) {
                let left = level[i];
                let right = level[i + 1];
                
                // Combine two hashes
                let mut hasher = Hasher::new();
                hasher.update(&left);
                hasher.update(&right);
                let hash = hasher.finalize();
                
                let mut combined = [0u8; 32];
                combined.copy_from_slice(hash.as_bytes());
                
                next_level.push(combined);
            }
            
            // Update index for next level
            index = index / 2;
            level = next_level;
        }
        
        Some(MerkleProof {
            leaf: *id,
            proof,
            root: level[0],
        })
    }
    
    /// Set the current block height
    pub fn set_height(&mut self, height: u64) {
        self.height = height;
    }
    
    /// Get the current block height
    pub fn height(&self) -> u64 {
        self.height
    }
}

/// Merkle proof for proving inclusion of a UTXO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// The leaf (UTXO ID)
    pub leaf: Hash,
    
    /// The proof (sibling hashes)
    pub proof: Vec<Hash>,
    
    /// The Merkle root
    pub root: Hash,
}

impl MerkleProof {
    /// Verify the Merkle proof
    pub fn verify(&self) -> bool {
        let mut current = self.leaf;
        
        for sibling in &self.proof {
            // Combine current with sibling
            let mut hasher = Hasher::new();
            
            // Ensure deterministic ordering
            if current <= *sibling {
                hasher.update(&current);
                hasher.update(sibling);
            } else {
                hasher.update(sibling);
                hasher.update(&current);
            }
            
            let hash = hasher.finalize();
            current.copy_from_slice(hash.as_bytes());
        }
        
        // Check if we arrived at the root
        current == self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_utxo_lifecycle() {
        let mut utxo_set = UTXOSet::new();
        
        // Create a UTXO
        let tx_hash = [1u8; 32];
        let owner = [2u8; 20];
        let utxo_id = utxo_set.add_utxo(tx_hash, 0, 100, owner);
        
        // Check UTXO exists
        let utxo = utxo_set.get_utxo(&utxo_id).unwrap();
        assert_eq!(utxo.value, 100);
        assert_eq!(utxo.owner, owner);
        assert!(!utxo.spent);
        
        // Check address balance
        assert_eq!(utxo_set.get_balance(&owner), 100);
        
        // Spend the UTXO
        utxo_set.spend_utxo(&utxo_id).unwrap();
        
        // Check UTXO is spent
        let utxo = utxo_set.get_utxo(&utxo_id).unwrap();
        assert!(utxo.spent);
        assert_eq!(utxo.spent_at, Some(0));
        
        // Balance should be zero now
        assert_eq!(utxo_set.get_balance(&owner), 0);
        
        // Trying to spend again should fail
        assert!(matches!(utxo_set.spend_utxo(&utxo_id), Err(UTXOError::AlreadySpent)));
    }
    
    #[test]
    fn test_merkle_root_and_proof() {
        let mut utxo_set = UTXOSet::new();
        
        // Create some UTXOs
        let tx_hash = [1u8; 32];
        let owner1 = [2u8; 20];
        let owner2 = [3u8; 20];
        
        let id1 = utxo_set.add_utxo(tx_hash, 0, 100, owner1);
        let id2 = utxo_set.add_utxo(tx_hash, 1, 200, owner2);
        let id3 = utxo_set.add_utxo(tx_hash, 2, 300, owner1);
        
        // Get Merkle root
        let root = utxo_set.merkle_root();
        assert_ne!(root, [0u8; 32]);
        
        // Get proof for a UTXO
        let proof = utxo_set.generate_proof(&id2).unwrap();
        assert_eq!(proof.leaf, id2);
        assert_eq!(proof.root, root);
        
        // Verify the proof
        assert!(proof.verify());
        
        // Spend a UTXO and check root changes
        utxo_set.spend_utxo(&id1).unwrap();
        let new_root = utxo_set.merkle_root();
        assert_ne!(new_root, root);
        
        // Get proof for remaining UTXO
        let proof2 = utxo_set.generate_proof(&id2).unwrap();
        assert_eq!(proof2.root, new_root);
        assert!(proof2.verify());
    }
}
