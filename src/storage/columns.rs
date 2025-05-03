use std::sync::Arc;
use rocksdb::{ColumnFamilyDescriptor, Options};

/// Database column families for storing different types of data
#[derive(Debug, Clone, Copy)]
pub enum Column {
    /// Default column (used for metadata)
    Default = 0,
    
    /// Block headers indexed by hash
    BlockHeaders = 1,
    
    /// Block bodies indexed by hash
    BlockBodies = 2,
    
    /// Transactions indexed by hash
    Transactions = 3,
    
    /// Transaction receipts indexed by transaction hash
    Receipts = 4,
    
    /// Chain state (account states, etc.)
    State = 5,
    
    /// Block hash to slot mapping
    BlockHashToSlot = 6,
    
    /// Slot to block hash mapping
    SlotToBlockHash = 7,
    
    /// Finalized block data
    FinalizedBlocks = 8,
    
    /// Validator data
    Validators = 9,
    
    /// Wallet data (encrypted)
    Wallets = 10,
    
    /// STARK proofs
    StarkProofs = 11,
}

impl Column {
    /// Get all columns as a slice
    pub fn all() -> &'static [Column] {
        &[
            Column::Default,
            Column::BlockHeaders,
            Column::BlockBodies,
            Column::Transactions,
            Column::Receipts,
            Column::State,
            Column::BlockHashToSlot,
            Column::SlotToBlockHash,
            Column::FinalizedBlocks,
            Column::Validators,
            Column::Wallets,
            Column::StarkProofs,
        ]
    }
    
    /// Get column family name
    pub fn name(&self) -> &'static str {
        match self {
            Column::Default => "default",
            Column::BlockHeaders => "block_headers",
            Column::BlockBodies => "block_bodies",
            Column::Transactions => "transactions",
            Column::Receipts => "receipts",
            Column::State => "state",
            Column::BlockHashToSlot => "block_hash_to_slot",
            Column::SlotToBlockHash => "slot_to_block_hash",
            Column::FinalizedBlocks => "finalized_blocks",
            Column::Validators => "validators",
            Column::Wallets => "wallets",
            Column::StarkProofs => "stark_proofs",
        }
    }
    
    /// Create column family descriptors with optimized options
    pub fn descriptors() -> Vec<ColumnFamilyDescriptor> {
        let mut descriptors = Vec::with_capacity(Self::all().len());
        
        for column in Self::all() {
            let mut options = Options::default();
            
            // Configure options based on column type
            match column {
                Column::Default => {
                    options.set_max_write_buffer_number(4);
                    options.set_write_buffer_size(64 * 1024 * 1024); // 64MB
                },
                Column::BlockHeaders | Column::BlockBodies => {
                    // Optimize for read-heavy but occasional writes
                    options.set_max_write_buffer_number(4);
                    options.set_write_buffer_size(64 * 1024 * 1024); // 64MB
                    options.optimize_for_point_lookup(1024); // 1KB block cache
                },
                Column::Transactions | Column::Receipts => {
                    // Optimize for high read/write throughput
                    options.set_max_write_buffer_number(6);
                    options.set_write_buffer_size(128 * 1024 * 1024); // 128MB
                },
                Column::State => {
                    // State has frequent updates, optimize accordingly
                    options.set_max_write_buffer_number(6);
                    options.set_write_buffer_size(256 * 1024 * 1024); // 256MB
                    options.set_max_background_jobs(4);
                },
                _ => {
                    // Default configuration for other columns
                    options.set_max_write_buffer_number(2);
                    options.set_write_buffer_size(32 * 1024 * 1024); // 32MB
                },
            }
            
            descriptors.push(ColumnFamilyDescriptor::new(column.name(), options));
        }
        
        descriptors
    }
}

/// Column family handles
pub struct ColumnFamilyHandles {
    /// Handles for each column family
    handles: Vec<Arc<rocksdb::ColumnFamily>>,
}

impl ColumnFamilyHandles {
    /// Create from raw handles
    pub fn new(handles: Vec<Arc<rocksdb::ColumnFamily>>) -> Self {
        Self { handles }
    }
    
    /// Get a column family handle by column
    pub fn get(&self, column: Column) -> Arc<rocksdb::ColumnFamily> {
        self.handles[column as usize].clone()
    }
} 