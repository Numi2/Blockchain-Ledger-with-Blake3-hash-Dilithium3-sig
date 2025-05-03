use crate::core::block::{Block, BlockBody, BlockHeader};
use crate::storage::{Column, Database, StorageError, StorageResult};
use crate::types::{Hash, Slot};
use crate::ssz::{serialize, deserialize};

/// BlockStorage provides functionality for storing and retrieving blocks
pub struct BlockStorage {
    db: Database,
}

impl BlockStorage {
    /// Create a new BlockStorage with the given database
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    /// Store a block
    pub fn store_block(&self, block: &Block) -> StorageResult<()> {
        let block_hash = block.hash();
        let slot = block.header.slot;
        
        // Store components separately for efficient retrieval
        self.store_block_header(&block.header)?;
        self.store_block_body(&block.body, &block_hash)?;
        
        // Store slot to block hash mapping
        self.db.put(Column::SlotToBlockHash, &slot.to_le_bytes(), &block_hash)?;
        
        // Store block hash to slot mapping
        self.db.put(Column::BlockHashToSlot, &block_hash, &slot.to_le_bytes())?;
        
        Ok(())
    }
    
    /// Store a block header
    pub fn store_block_header(&self, header: &BlockHeader) -> StorageResult<()> {
        let header_hash = header.hash();
        let header_bytes = serialize(header)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(Column::BlockHeaders, &header_hash, &header_bytes)
    }
    
    /// Store a block body
    pub fn store_block_body(&self, body: &BlockBody, block_hash: &Hash) -> StorageResult<()> {
        let body_bytes = serialize(body)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        self.db.put(Column::BlockBodies, block_hash, &body_bytes)
    }
    
    /// Mark a block as finalized
    pub fn mark_finalized(&self, block_hash: &Hash, slot: Slot) -> StorageResult<()> {
        // Store finalized block hash for slot
        self.db.put(Column::FinalizedBlocks, &slot.to_le_bytes(), block_hash)?;
        
        // Also store latest finalized slot in default column
        self.db.put(Column::Default, b"latest_finalized_slot", &slot.to_le_bytes())?;
        
        Ok(())
    }
    
    /// Get a block by hash
    pub fn get_block(&self, hash: &Hash) -> StorageResult<Option<Block>> {
        // Get header and body separately
        let header = match self.get_block_header(hash)? {
            Some(header) => header,
            None => return Ok(None),
        };
        
        let body = match self.get_block_body(hash)? {
            Some(body) => body,
            None => return Ok(None),
        };
        
        Ok(Some(Block::new(header, body)))
    }
    
    /// Get a block header by hash
    pub fn get_block_header(&self, hash: &Hash) -> StorageResult<Option<BlockHeader>> {
        let header_bytes = match self.db.get(Column::BlockHeaders, hash)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        let header = deserialize(&header_bytes)
            .map_err(|e| StorageError::Serialization(e))?;
        
        Ok(Some(header))
    }
    
    /// Get a block body by hash
    pub fn get_block_body(&self, hash: &Hash) -> StorageResult<Option<BlockBody>> {
        let body_bytes = match self.db.get(Column::BlockBodies, hash)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        let body = deserialize(&body_bytes)
            .map_err(|e| StorageError::Serialization(e))?;
        
        Ok(Some(body))
    }
    
    /// Get a block by slot
    pub fn get_block_by_slot(&self, slot: Slot) -> StorageResult<Option<Block>> {
        let hash_bytes = match self.db.get(Column::SlotToBlockHash, &slot.to_le_bytes())? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        if hash_bytes.len() != 32 {
            return Err(StorageError::InvalidData("Invalid block hash length".to_string()));
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);
        
        self.get_block(&hash)
    }
    
    /// Get the latest finalized block
    pub fn get_latest_finalized_block(&self) -> StorageResult<Option<Block>> {
        let slot_bytes = match self.db.get(Column::Default, b"latest_finalized_slot")? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        if slot_bytes.len() != 8 {
            return Err(StorageError::InvalidData("Invalid slot length".to_string()));
        }
        
        let mut slot_arr = [0u8; 8];
        slot_arr.copy_from_slice(&slot_bytes);
        let slot = u64::from_le_bytes(slot_arr);
        
        let block_hash = match self.db.get(Column::FinalizedBlocks, &slot_bytes)? {
            Some(bytes) => {
                if bytes.len() != 32 {
                    return Err(StorageError::InvalidData("Invalid block hash length".to_string()));
                }
                
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&bytes);
                hash
            },
            None => return Ok(None),
        };
        
        self.get_block(&block_hash)
    }
    
    /// Get the slot of the latest finalized block
    pub fn get_latest_finalized_slot(&self) -> StorageResult<Option<Slot>> {
        let slot_bytes = match self.db.get(Column::Default, b"latest_finalized_slot")? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        if slot_bytes.len() != 8 {
            return Err(StorageError::InvalidData("Invalid slot length".to_string()));
        }
        
        let mut slot_arr = [0u8; 8];
        slot_arr.copy_from_slice(&slot_bytes);
        let slot = u64::from_le_bytes(slot_arr);
        
        Ok(Some(slot))
    }
    
    /// Check if a block exists
    pub fn block_exists(&self, hash: &Hash) -> StorageResult<bool> {
        self.db.exists(Column::BlockHeaders, hash)
    }
    
    /// Delete a block (both header and body)
    pub fn delete_block(&self, hash: &Hash) -> StorageResult<()> {
        // Get the slot first to remove mappings
        let slot_bytes = match self.db.get(Column::BlockHashToSlot, hash)? {
            Some(bytes) => bytes,
            None => return Ok(()), // Block doesn't exist, nothing to delete
        };
        
        // Delete header, body, and mappings
        let mut batch = self.db.batch();
        batch.delete(Column::BlockHeaders, hash)
             .delete(Column::BlockBodies, hash)
             .delete(Column::BlockHashToSlot, hash)
             .delete(Column::SlotToBlockHash, &slot_bytes);
        
        batch.write()
    }
    
    /// Get all blocks in a slot range
    pub fn get_blocks_by_slot_range(&self, start_slot: Slot, end_slot: Slot) -> StorageResult<Vec<Block>> {
        let mut blocks = Vec::new();
        
        for slot in start_slot..=end_slot {
            if let Some(block) = self.get_block_by_slot(slot)? {
                blocks.push(block);
            }
        }
        
        Ok(blocks)
    }
    
    /// Get blocks after a given hash, up to a maximum number
    pub fn get_blocks_after(&self, hash: &Hash, max_count: usize) -> StorageResult<Vec<Block>> {
        let mut blocks = Vec::new();
        let mut current_hash = *hash;
        
        // Find the slot for this hash
        let slot_bytes = match self.db.get(Column::BlockHashToSlot, &current_hash)? {
            Some(bytes) => bytes,
            None => return Ok(blocks), // Hash not found, return empty
        };
        
        let mut slot_arr = [0u8; 8];
        slot_arr.copy_from_slice(&slot_bytes);
        let mut slot = u64::from_le_bytes(slot_arr);
        
        // Iterate through subsequent slots
        while blocks.len() < max_count {
            slot += 1;
            
            if let Some(block) = self.get_block_by_slot(slot)? {
                blocks.push(block);
            } else {
                break; // No more blocks
            }
        }
        
        Ok(blocks)
    }
} 