use crate::core::block::Block;
use crate::types::Hash;
use std::collections::{HashMap, HashSet};

/// SimpleChain represents a chain of blocks
struct SimpleChain {
    /// The blocks in this chain (by hash)
    blocks: HashMap<Hash, Block>,
    
    /// Map of child blocks for each parent hash
    children: HashMap<Hash, Vec<Hash>>,
    
    /// The head (tip) of this chain
    head: Hash,
    
    /// Total number of blocks in this chain
    length: usize,
}

/// ForkChoice implements the fork choice rule for selecting the canonical chain
pub struct ForkChoice {
    /// All known blocks by hash
    blocks: HashMap<Hash, Block>,
    
    /// Map of parent hash to child hashes
    children: HashMap<Hash, Vec<Hash>>,
    
    /// The current head of the canonical chain
    head: Hash,
    
    /// The finalized block hash (no reorgs past this point)
    finalized_hash: Hash,
    
    /// Set of blocks that have been finalized
    finalized_blocks: HashSet<Hash>,
}

impl ForkChoice {
    /// Create a new fork choice rule
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            children: HashMap::new(),
            head: [0u8; 32], // Genesis hash
            finalized_hash: [0u8; 32], // Genesis hash
            finalized_blocks: HashSet::new(),
        }
    }

    /// Add a block to the fork choice
    pub fn add_block(&mut self, block: Block) -> Result<(), &'static str> {
        let block_hash = block.hash();
        let parent_hash = block.header.parent_hash;
        
        // Check if we already know this block
        if self.blocks.contains_key(&block_hash) {
            return Ok(());
        }
        
        // Check if the parent exists (except for genesis)
        if parent_hash != [0u8; 32] && !self.blocks.contains_key(&parent_hash) {
            return Err("Parent block unknown");
        }
        
        // Add block to our maps
        self.blocks.insert(block_hash, block);
        
        // Update children map
        self.children
            .entry(parent_hash)
            .or_insert_with(Vec::new)
            .push(block_hash);
        
        // Run fork choice to update head
        self.update_head();
        
        Ok(())
    }

    /// Mark a block as finalized
    pub fn set_finalized(&mut self, block_hash: Hash) -> Result<(), &'static str> {
        // Check if the block exists
        if !self.blocks.contains_key(&block_hash) {
            return Err("Block not found");
        }
        
        // Mark the block as finalized
        self.finalized_hash = block_hash;
        self.finalized_blocks.insert(block_hash);
        
        // Mark all ancestors as finalized as well
        let mut current_hash = block_hash;
        while current_hash != [0u8; 32] {
            let parent_hash = self.blocks.get(&current_hash).unwrap().header.parent_hash;
            self.finalized_blocks.insert(parent_hash);
            current_hash = parent_hash;
        }
        
        // Run fork choice to update head
        self.update_head();
        
        Ok(())
    }

    /// Update the chain head based on fork choice rule
    fn update_head(&mut self) {
        // Start from the finalized block
        let mut current = self.finalized_hash;
        
        // Keep choosing the "best" child until we reach a leaf
        loop {
            let children = match self.children.get(&current) {
                Some(c) => c,
                None => break, // No children, we're at a leaf
            };
            
            if children.is_empty() {
                break;
            }
            
            // For simplicity, our fork choice rule is "highest slot wins"
            // In a full implementation, this would be LMD GHOST or similar
            let best_child = children
                .iter()
                .max_by_key(|&&child| self.blocks.get(&child).unwrap().header.slot)
                .unwrap();
            
            current = *best_child;
        }
        
        // Update the head
        self.head = current;
    }

    /// Get the current head of the chain
    pub fn get_head(&self) -> Hash {
        self.head
    }

    /// Get a block by hash
    pub fn get_block(&self, hash: &Hash) -> Option<&Block> {
        self.blocks.get(hash)
    }

    /// Check if a block is finalized
    pub fn is_finalized(&self, hash: &Hash) -> bool {
        self.finalized_blocks.contains(hash)
    }

    /// Get the chain from the genesis to the current head
    pub fn get_chain(&self) -> Vec<Hash> {
        let mut chain = Vec::new();
        let mut current = self.head;
        
        // Walk backwards from head to genesis
        while current != [0u8; 32] {
            chain.push(current);
            let parent = self.blocks.get(&current).unwrap().header.parent_hash;
            current = parent;
        }
        
        // Reverse to get genesis -> head order
        chain.reverse();
        chain
    }
} 