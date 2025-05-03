use world_ledger as wl;
use tempfile::TempDir;
use std::sync::Arc;
use std::time::Duration;
use wl::storage::Database;
use wl::core::state::WorldState;
use wl::wallet::AccountManager;
use wl::core::mempool::MemPool;

mod blockchain;
mod consensus;
mod networking;
mod wallet;
mod contracts;

/// Test node configuration
pub struct TestNodeConfig {
    /// Data directory
    pub data_dir: TempDir,
    
    /// Database instance
    pub db: Arc<Database>,
    
    /// World state
    pub world_state: Arc<WorldState>,
    
    /// Account manager
    pub account_manager: Arc<AccountManager>,
    
    /// Mempool
    pub mempool: Arc<MemPool>,
    
    /// Private keys for testing (hex encoded)
    pub private_keys: Vec<String>,
    
    /// Addresses for testing
    pub addresses: Vec<String>,
}

impl TestNodeConfig {
    /// Create a new test node configuration
    pub fn new() -> Self {
        // Create temp directory
        let data_dir = TempDir::new().expect("Failed to create temp dir");
        let path = data_dir.path().to_path_buf();
        
        // Create database
        let db = Arc::new(Database::open(&path.join("db"), None)
            .expect("Failed to create database"));
        
        // Create world state
        let world_state = Arc::new(WorldState::new(db.clone()));
        
        // Create account manager
        let account_manager = Arc::new(AccountManager::new(db.clone())
            .expect("Failed to create account manager"));
            
        // Create mempool
        let mempool = Arc::new(MemPool::new(world_state.clone(), 10000));
        
        // Generate a test account with some balance
        let private_keys = vec![
            "0xd2bf6e81c58cfde9d8a90b8f20a95eca847fde852c168e5fa645bfd14cf1b31b".to_string(),
            "0xe14dc6bc33a65b33108e82b3b0b2ea3ff147b16993ab5ce7829947faa2dd3739".to_string(),
        ];
        
        let addresses = vec![
            "0x7bb62c7d8c215b1ef3e55fc231e77b3ca1b960a3".to_string(),
            "0x6a6b09dff11f254c9513096a72b11c5de37e8392".to_string(),
        ];
        
        Self {
            data_dir,
            db,
            world_state,
            account_manager,
            mempool,
            private_keys,
            addresses,
        }
    }
    
    /// Create a test transaction
    pub fn create_test_transaction(&self, from_idx: usize, to_idx: usize, value: u64) -> wl::types::Transaction {
        use wl::types::{Transaction, Hash};
        
        // Get addresses
        let from_addr_str = &self.addresses[from_idx];
        let to_addr_str = &self.addresses[to_idx];
        
        // Convert from hex to Address
        let mut from = [0u8; 20];
        from.copy_from_slice(&hex::decode(&from_addr_str[2..]).expect("Invalid from address"));
        
        let mut to = [0u8; 20];
        to.copy_from_slice(&hex::decode(&to_addr_str[2..]).expect("Invalid to address"));
        
        // Get nonce
        let nonce = self.world_state.get_account(&from)
            .map(|a| a.nonce)
            .unwrap_or(0);
        
        Transaction {
            from,
            to: Some(to),
            value,
            gas_limit: 21000,
            gas_price: 1_000_000_000, // 1 gwei
            nonce,
            data: Vec::new(),
        }
    }
    
    /// Wait for a condition
    pub fn wait_for<F>(&self, mut condition: F, timeout: Duration) -> bool 
    where 
        F: FnMut() -> bool
    {
        use std::time::Instant;
        
        let start = Instant::now();
        while start.elapsed() < timeout {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }
}

/// Setup test environment
pub fn setup() -> TestNodeConfig {
    TestNodeConfig::new()
}

/// Clean up test environment
pub fn teardown(config: TestNodeConfig) {
    // TempDir will automatically clean up when dropped
    drop(config);
} 