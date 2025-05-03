use crate::types::{Address, Hash};
use crate::storage::{StorageResult, StorageError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Contract state access trait
pub trait StateAccess {
    /// Get a value from contract storage
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    
    /// Set a value in contract storage
    fn set(&mut self, key: &[u8], value: &[u8]) -> StorageResult<()>;
    
    /// Delete a value from contract storage
    fn delete(&mut self, key: &[u8]) -> StorageResult<()>;
    
    /// Check if a key exists in contract storage
    fn contains(&self, key: &[u8]) -> bool;
}

/// In-memory contract state
pub struct ContractState {
    /// Storage mapping (key -> value)
    storage: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl ContractState {
    /// Create a new contract state
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
        }
    }
}

impl StateAccess for ContractState {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let storage = self.storage.read().unwrap();
        storage.get(key).cloned()
    }
    
    fn set(&mut self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.insert(key.to_vec(), value.to_vec());
        Ok(())
    }
    
    fn delete(&mut self, key: &[u8]) -> StorageResult<()> {
        let mut storage = self.storage.write().unwrap();
        storage.remove(key);
        Ok(())
    }
    
    fn contains(&self, key: &[u8]) -> bool {
        let storage = self.storage.read().unwrap();
        storage.contains_key(key)
    }
}

/// Persistent contract state using the blockchain storage
pub struct PersistentContractState {
    /// Contract address
    address: Address,
    
    /// RocksDB storage adapter
    db: Arc<crate::storage::Database>,
    
    /// Cache for faster access
    cache: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl PersistentContractState {
    /// Create a new persistent contract state for a contract
    pub fn new(address: Address, db: Arc<crate::storage::Database>) -> Self {
        Self {
            address,
            db,
            cache: RwLock::new(HashMap::new()),
        }
    }
    
    /// Generate a storage key for the contract
    fn storage_key(&self, key: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.address.len() + 1 + key.len());
        result.extend_from_slice(&self.address);
        result.push(b':');
        result.extend_from_slice(key);
        result
    }
    
    /// Flush cache to storage
    pub fn flush(&self) -> StorageResult<()> {
        let cache = self.cache.read().unwrap();
        let mut batch = self.db.batch();
        
        for (key, value) in cache.iter() {
            let storage_key = self.storage_key(key);
            batch.put(crate::storage::Column::ContractStorage, &storage_key, value);
        }
        
        batch.write()?;
        Ok(())
    }
}

impl StateAccess for PersistentContractState {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(value) = cache.get(key) {
                return Some(value.clone());
            }
        }
        
        // Check database
        let storage_key = self.storage_key(key);
        match self.db.get(crate::storage::Column::ContractStorage, &storage_key) {
            Ok(Some(value)) => {
                // Update cache
                let mut cache = self.cache.write().unwrap();
                cache.insert(key.to_vec(), value.clone());
                Some(value)
            }
            _ => None,
        }
    }
    
    fn set(&mut self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        // Update cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(key.to_vec(), value.to_vec());
        }
        
        // Update database
        let storage_key = self.storage_key(key);
        self.db.put(crate::storage::Column::ContractStorage, &storage_key, value)?;
        
        Ok(())
    }
    
    fn delete(&mut self, key: &[u8]) -> StorageResult<()> {
        // Update cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(key);
        }
        
        // Update database
        let storage_key = self.storage_key(key);
        self.db.delete(crate::storage::Column::ContractStorage, &storage_key)?;
        
        Ok(())
    }
    
    fn contains(&self, key: &[u8]) -> bool {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if cache.contains_key(key) {
                return true;
            }
        }
        
        // Check database
        let storage_key = self.storage_key(key);
        match self.db.exists(crate::storage::Column::ContractStorage, &storage_key) {
            Ok(exists) => exists,
            _ => false,
        }
    }
}

/// Contract code storage
pub struct ContractCodeStorage {
    /// RocksDB storage adapter
    db: Arc<crate::storage::Database>,
}

impl ContractCodeStorage {
    /// Create a new contract code storage
    pub fn new(db: Arc<crate::storage::Database>) -> Self {
        Self { db }
    }
    
    /// Store contract code
    pub fn store_code(&self, address: &Address, code: &[u8]) -> StorageResult<()> {
        self.db.put(crate::storage::Column::ContractCode, address, code)
    }
    
    /// Get contract code
    pub fn get_code(&self, address: &Address) -> StorageResult<Option<Vec<u8>>> {
        self.db.get(crate::storage::Column::ContractCode, address)
    }
    
    /// Delete contract code
    pub fn delete_code(&self, address: &Address) -> StorageResult<()> {
        self.db.delete(crate::storage::Column::ContractCode, address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_in_memory_state() {
        let mut state = ContractState::new();
        
        // Initially empty
        assert_eq!(state.get(b"test"), None);
        assert!(!state.contains(b"test"));
        
        // Set a value
        state.set(b"test", b"value").unwrap();
        
        // Now we should find it
        assert_eq!(state.get(b"test"), Some(b"value".to_vec()));
        assert!(state.contains(b"test"));
        
        // Delete the value
        state.delete(b"test").unwrap();
        
        // Should be gone
        assert_eq!(state.get(b"test"), None);
        assert!(!state.contains(b"test"));
    }
} 