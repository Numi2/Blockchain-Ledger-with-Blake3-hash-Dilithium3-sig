use crate::types::{Address, Hash, Signature, Slot, Transaction, ValidatorIndex};
use crate::core::state::WorldState;
use crate::ssz;
use blake3;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Block header containing block metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Previous block hash
    pub prev_hash: Hash,
    
    /// Block height
    pub height: u64,
    
    /// Block slot
    pub slot: Slot,
    
    /// Block timestamp (seconds since Unix epoch)
    pub timestamp: u64,
    
    /// Block state root hash (Merkle root of state)
    pub state_root: Hash,
    
    /// Transactions root hash (Merkle root of transactions)
    pub transactions_root: Hash,
    
    /// Block producer address
    pub producer: Address,
    
    /// In PoW: nonce used to find valid hash
    /// In PoS: not used (can be 0)
    pub nonce: u64,
    
    /// Difficulty target (for PoW only)
    pub difficulty: u64,
}

/// Block body containing transactions and attestations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockBody {
    /// List of transactions
    pub transactions: Vec<Transaction>,
    
    /// List of attestations (for PoS)
    pub attestations: Vec<Attestation>,
    
    /// Any additional metadata
    pub metadata: Vec<u8>,
}

/// Attestation (for PoS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// Validator index
    pub validator_index: ValidatorIndex,
    
    /// Attested block hash
    pub block_hash: Hash,
    
    /// Source epoch
    pub source_epoch: u64,
    
    /// Target epoch
    pub target_epoch: u64,
    
    /// Signature
    pub signature: Signature,
}

/// Complete block (header + body)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    
    /// Block body
    pub body: BlockBody,
    
    /// Block signature (for PoS)
    pub signature: Option<Signature>,
}

impl Block {
    /// Create a new block
    pub fn new(
        prev_hash: Hash,
        height: u64,
        slot: Slot,
        state_root: Hash,
        transactions: Vec<Transaction>,
        producer: Address,
        difficulty: u64,
    ) -> Self {
        // Compute transaction root
        let transactions_root = compute_transactions_root(&transactions);
        
        // Create header
        let header = BlockHeader {
            prev_hash,
            height,
            slot,
            timestamp: current_timestamp(),
            state_root,
            transactions_root,
            producer,
            nonce: 0,
            difficulty,
        };
        
        // Create body
        let body = BlockBody {
            transactions,
            attestations: Vec::new(),
            metadata: Vec::new(),
        };
        
        Self {
            header,
            body,
            signature: None,
        }
    }
    
    /// Calculate the block hash
    pub fn hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        
        // Hash the header fields
        hasher.update(&self.header.prev_hash);
        hasher.update(&self.header.height.to_le_bytes());
        hasher.update(&self.header.slot.to_le_bytes());
        hasher.update(&self.header.timestamp.to_le_bytes());
        hasher.update(&self.header.state_root);
        hasher.update(&self.header.transactions_root);
        hasher.update(&self.header.producer);
        hasher.update(&self.header.nonce.to_le_bytes());
        hasher.update(&self.header.difficulty.to_le_bytes());
        
        // For PoS blocks, include the signature if present
        if let Some(sig) = &self.signature {
            hasher.update(sig);
        }
        
        // Compute final hash
        let hash_result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(hash_result.as_bytes());
        
        hash
    }
    
    /// Sign the block (for PoS)
    pub fn sign(&mut self, signature: Signature) {
        self.signature = Some(signature);
    }
    
    /// Calculate block size in bytes
    pub fn size(&self) -> usize {
        // Serialize to get accurate size
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        bytes.len()
    }
    
    /// Add an attestation to the block
    pub fn add_attestation(&mut self, attestation: Attestation) {
        self.body.attestations.push(attestation);
    }
    
    /// For PoW: set the nonce and recalculate the hash
    pub fn set_pow_nonce(&mut self, nonce: u64) {
        self.header.nonce = nonce;
    }
    
    /// Serialize the block for storage or transmission
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        ssz::serialize(self).map_err(|e| e.to_string())
    }
    
    /// Deserialize a block from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        ssz::deserialize(data).map_err(|e| e.to_string())
    }
    
    /// Validate the basic structure of the block
    pub fn validate_structure(&self) -> Result<(), String> {
        // Check that the transactions root matches
        let expected_tx_root = compute_transactions_root(&self.body.transactions);
        if expected_tx_root != self.header.transactions_root {
            return Err("Invalid transactions root".to_string());
        }
        
        // More validations as needed...
        
        Ok(())
    }
}

/// Block production manages the creation of new blocks
pub struct BlockProduction {
    /// Current state
    world_state: WorldState,
    
    /// Pending transactions
    pending_transactions: Vec<Transaction>,
    
    /// Maximum transactions per block
    max_transactions_per_block: usize,
}

impl BlockProduction {
    /// Create a new block production instance
    pub fn new(world_state: WorldState) -> Self {
        Self {
            world_state,
            pending_transactions: Vec::new(),
            max_transactions_per_block: 1000, // Default
        }
    }
    
    /// Set the maximum transactions per block
    pub fn set_max_transactions_per_block(&mut self, max: usize) {
        self.max_transactions_per_block = max;
    }
    
    /// Add a transaction to the pending pool
    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.pending_transactions.push(transaction);
    }
    
    /// Create a new block with pending transactions
    pub fn produce_block(
        &mut self,
        prev_hash: Hash,
        height: u64,
        slot: Slot,
        producer: Address,
        difficulty: u64,
    ) -> Block {
        // Sort transactions by gas price (for priority)
        self.pending_transactions
            .sort_by(|a, b| b.gas_price.cmp(&a.gas_price));
        
        // Take up to max_transactions_per_block
        let mut block_transactions = Vec::new();
        
        // Try to fit as many transactions as possible
        for tx in self.pending_transactions.drain(..) {
            // In a real implementation, we would apply transactions to a temporary state
            // to ensure they're valid and fit within gas limits
            block_transactions.push(tx);
            
            if block_transactions.len() >= self.max_transactions_per_block {
                break;
            }
        }
        
        // Compute state root
        let state_root = self.world_state.state_root();
        
        // Create block
        Block::new(
            prev_hash,
            height,
            slot,
            state_root,
            block_transactions,
            producer,
            difficulty,
        )
    }
}

/// Calculate the Merkle root of transactions
fn compute_transactions_root(transactions: &[Transaction]) -> Hash {
    // For simplicity, hash all transactions together
    // In a real implementation, we'd build a Merkle tree
    let mut hasher = blake3::Hasher::new();
    
    for tx in transactions {
        let tx_hash = tx.hash();
        hasher.update(&tx_hash);
    }
    
    let result = hasher.finalize();
    let mut root = [0u8; 32];
    root.copy_from_slice(result.as_bytes());
    
    root
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
} 