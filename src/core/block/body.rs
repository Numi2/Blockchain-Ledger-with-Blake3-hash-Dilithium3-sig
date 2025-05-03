use crate::core::block::Transaction;
use crate::types::Hash;
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// BlockBody contains the transactions and other content of a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockBody {
    /// List of transactions included in this block
    pub transactions: Vec<Transaction>,
}

impl BlockBody {
    /// Creates a new block body with the given transactions
    pub fn new(transactions: Vec<Transaction>) -> Self {
        Self { transactions }
    }

    /// Compute the body root hash using a simple Merkle tree approach with BLAKE3
    pub fn root_hash(&self) -> Hash {
        if self.transactions.is_empty() {
            // Return a zero hash for empty transaction list
            return [0u8; 32];
        }

        // First get the hash of each transaction
        let tx_hashes: Vec<Hash> = self.transactions.iter()
            .map(|tx| tx.hash())
            .collect();

        // Create a simple Merkle tree of the transaction hashes
        self.compute_merkle_root(&tx_hashes)
    }

    /// Compute a simple Merkle root from a list of hashes
    fn compute_merkle_root(&self, hashes: &[Hash]) -> Hash {
        // Base case: single hash is the root
        if hashes.len() == 1 {
            return hashes[0];
        }

        // Create a new level in the tree by pairing adjacent hashes
        let mut next_level = Vec::with_capacity((hashes.len() + 1) / 2);
        
        for chunk in hashes.chunks(2) {
            let mut hasher = Hasher::new();
            
            // Add the first hash
            hasher.update(&chunk[0]);
            
            // If there's a second hash, add it; otherwise duplicate the first
            if chunk.len() > 1 {
                hasher.update(&chunk[1]);
            } else {
                hasher.update(&chunk[0]); // Duplicate the hash if odd number
            }
            
            // Create the parent hash
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(result.as_bytes());
            next_level.push(hash);
        }
        
        // Recursively compute the Merkle root of the next level
        self.compute_merkle_root(&next_level)
    }
}

/// Block represents a complete block in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: super::header::BlockHeader,
    
    /// Block body
    pub body: BlockBody,
}

impl Block {
    /// Creates a new block
    pub fn new(header: super::header::BlockHeader, body: BlockBody) -> Self {
        Self { header, body }
    }

    /// Gets the hash of this block (which is the hash of its header)
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }
} 