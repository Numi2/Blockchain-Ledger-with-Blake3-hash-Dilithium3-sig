use crate::core::state::{Account, WorldState};
use crate::storage::{Column, Database, StorageError, StorageResult};
use crate::types::{Address, Hash};
use crate::ssz::{serialize, deserialize};
use std::collections::HashMap;

/// StateStorage provides functionality for storing and retrieving blockchain state
pub struct StateStorage {
    db: Database,
}

impl StateStorage {
    /// Create a new StateStorage with the given database
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    /// Store the entire world state
    pub fn store_world_state(&self, state: &WorldState) -> StorageResult<()> {
        let state_root = state.state_root();
        let state_bytes = serialize(state)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        // Store full state with its root hash as key
        self.db.put(Column::State, &state_root, &state_bytes)?;
        
        // Also store the current world state hash in the default column
        self.db.put(Column::Default, b"current_state_root", &state_root)?;
        
        // Store individual accounts for direct access
        for (address, account) in &state.accounts {
            self.store_account(address, account)?;
        }
        
        Ok(())
    }
    
    /// Store an individual account
    pub fn store_account(&self, address: &Address, account: &Account) -> StorageResult<()> {
        let account_bytes = serialize(account)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        
        // Use a prefix to separate account data from other state data
        let mut key = Vec::with_capacity(1 + address.len());
        key.push(b'a'); // 'a' for account
        key.extend_from_slice(address);
        
        self.db.put(Column::State, &key, &account_bytes)?;
        
        // Also store account storage
        for (storage_key, storage_value) in &account.storage {
            self.store_account_storage(address, storage_key, storage_value)?;
        }
        
        Ok(())
    }
    
    /// Store account storage value
    pub fn store_account_storage(&self, address: &Address, key: &Hash, value: &Hash) -> StorageResult<()> {
        // Create composite key: 's' + address + storage_key
        let mut composite_key = Vec::with_capacity(1 + address.len() + key.len());
        composite_key.push(b's'); // 's' for storage
        composite_key.extend_from_slice(address);
        composite_key.extend_from_slice(key);
        
        self.db.put(Column::State, &composite_key, value)?;
        
        Ok(())
    }
    
    /// Get the current world state
    pub fn get_current_world_state(&self) -> StorageResult<Option<WorldState>> {
        // Get the current state root
        let root_bytes = match self.db.get(Column::Default, b"current_state_root")? {
            Some(bytes) => bytes,
            None => return Ok(None), // No current state
        };
        
        if root_bytes.len() != 32 {
            return Err(StorageError::InvalidData("Invalid state root length".to_string()));
        }
        
        let mut root = [0u8; 32];
        root.copy_from_slice(&root_bytes);
        
        // Get the state by root
        self.get_world_state_by_root(&root)
    }
    
    /// Get a world state by its root hash
    pub fn get_world_state_by_root(&self, root: &Hash) -> StorageResult<Option<WorldState>> {
        let state_bytes = match self.db.get(Column::State, root)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        let state = deserialize(&state_bytes)
            .map_err(|e| StorageError::Serialization(e))?;
        
        Ok(Some(state))
    }
    
    /// Get an account by address
    pub fn get_account(&self, address: &Address) -> StorageResult<Option<Account>> {
        // Create key: 'a' + address
        let mut key = Vec::with_capacity(1 + address.len());
        key.push(b'a'); // 'a' for account
        key.extend_from_slice(address);
        
        let account_bytes = match self.db.get(Column::State, &key)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        let account = deserialize(&account_bytes)
            .map_err(|e| StorageError::Serialization(e))?;
        
        Ok(Some(account))
    }
    
    /// Get account storage value
    pub fn get_account_storage(&self, address: &Address, key: &Hash) -> StorageResult<Option<Hash>> {
        // Create composite key: 's' + address + storage_key
        let mut composite_key = Vec::with_capacity(1 + address.len() + key.len());
        composite_key.push(b's'); // 's' for storage
        composite_key.extend_from_slice(address);
        composite_key.extend_from_slice(key);
        
        let value_bytes = match self.db.get(Column::State, &composite_key)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        
        if value_bytes.len() != 32 {
            return Err(StorageError::InvalidData("Invalid storage value length".to_string()));
        }
        
        let mut value = [0u8; 32];
        value.copy_from_slice(&value_bytes);
        
        Ok(Some(value))
    }
    
    /// Get all account storage for an address
    pub fn get_all_account_storage(&self, address: &Address) -> StorageResult<HashMap<Hash, Hash>> {
        let mut storage = HashMap::new();
        
        // Create prefix: 's' + address
        let mut prefix = Vec::with_capacity(1 + address.len());
        prefix.push(b's'); // 's' for storage
        prefix.extend_from_slice(address);
        
        // Iterate over all keys with this prefix
        let iter = self.db.iter(Column::State)?;
        
        for (key_bytes, value_bytes) in iter {
            let key = key_bytes.to_vec();
            
            // Check if this key has our prefix
            if key.len() > prefix.len() && key.starts_with(&prefix) {
                // Extract the storage key (after the prefix)
                let storage_key_bytes = &key[prefix.len()..];
                if storage_key_bytes.len() != 32 {
                    continue; // Skip invalid keys
                }
                
                // Extract the storage value
                if value_bytes.len() != 32 {
                    continue; // Skip invalid values
                }
                
                let mut storage_key = [0u8; 32];
                storage_key.copy_from_slice(storage_key_bytes);
                
                let mut storage_value = [0u8; 32];
                storage_value.copy_from_slice(&value_bytes);
                
                storage.insert(storage_key, storage_value);
            }
        }
        
        Ok(storage)
    }
    
    /// Delete an account
    pub fn delete_account(&self, address: &Address) -> StorageResult<()> {
        // Get the account first to find its storage
        let storage = match self.get_all_account_storage(address)? {
            storage => storage,
        };
        
        // Start a batch delete
        let mut batch = self.db.batch();
        
        // Delete the account
        let mut account_key = Vec::with_capacity(1 + address.len());
        account_key.push(b'a');
        account_key.extend_from_slice(address);
        batch.delete(Column::State, &account_key);
        
        // Delete all storage entries
        for storage_key in storage.keys() {
            let mut composite_key = Vec::with_capacity(1 + address.len() + storage_key.len());
            composite_key.push(b's');
            composite_key.extend_from_slice(address);
            composite_key.extend_from_slice(storage_key);
            
            batch.delete(Column::State, &composite_key);
        }
        
        batch.write()
    }
    
    /// Check if an account exists
    pub fn account_exists(&self, address: &Address) -> StorageResult<bool> {
        let mut key = Vec::with_capacity(1 + address.len());
        key.push(b'a');
        key.extend_from_slice(address);
        
        self.db.exists(Column::State, &key)
    }
    
    /// Get account balance
    pub fn get_balance(&self, address: &Address) -> StorageResult<u64> {
        match self.get_account(address)? {
            Some(account) => Ok(account.balance),
            None => Ok(0), // Non-existent accounts have zero balance
        }
    }
    
    /// Get account nonce
    pub fn get_nonce(&self, address: &Address) -> StorageResult<u64> {
        match self.get_account(address)? {
            Some(account) => Ok(account.nonce),
            None => Ok(0), // Non-existent accounts have zero nonce
        }
    }
    
    /// Get account code
    pub fn get_code(&self, address: &Address) -> StorageResult<Vec<u8>> {
        match self.get_account(address)? {
            Some(account) => Ok(account.code.clone()),
            None => Ok(Vec::new()), // Non-existent accounts have empty code
        }
    }
} 