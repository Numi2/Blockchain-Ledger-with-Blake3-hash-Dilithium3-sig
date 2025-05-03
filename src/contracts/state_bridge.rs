use crate::core::state::{UTXOState, State, UTXO, UTXORef, UTXOError};
use crate::contracts::state::{StateAccess, StateError};
use crate::types::{Address, Hash, Transaction, TxIn, TxOut};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;

/// UTXO state bridge error
#[derive(Error, Debug)]
pub enum StateBridgeError {
    #[error("UTXO error: {0}")]
    UTXOError(#[from] UTXOError),
    
    #[error("State error: {0}")]
    StateError(#[from] StateError),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type for state bridge operations
pub type StateBridgeResult<T> = Result<T, StateBridgeError>;

/// Bridge between WASM contract state and UTXO state
pub struct UTXOStateBridge {
    /// The underlying UTXO state
    utxo_state: Arc<RwLock<UTXOState>>,
    
    /// Contract address
    contract_address: Address,
    
    /// Caller address
    caller: Address,
    
    /// Contract state cache
    state_cache: Mutex<HashMap<Vec<u8>, Vec<u8>>>,
    
    /// UTXOs created during execution
    created_utxos: Mutex<Vec<UTXO>>,
    
    /// UTXOs spent during execution
    spent_utxos: Mutex<Vec<Hash>>,
}

impl UTXOStateBridge {
    /// Create a new UTXO state bridge
    pub fn new(
        utxo_state: Arc<RwLock<UTXOState>>,
        contract_address: Address,
        caller: Address,
    ) -> Self {
        Self {
            utxo_state,
            contract_address,
            caller,
            state_cache: Mutex::new(HashMap::new()),
            created_utxos: Mutex::new(Vec::new()),
            spent_utxos: Mutex::new(Vec::new()),
        }
    }
    
    /// Get UTXO by its ID
    pub fn get_utxo(&self, id: &Hash) -> StateBridgeResult<Option<UTXO>> {
        let state = self.utxo_state.read().unwrap();
        Ok(state.utxo_set().get_utxo(id).cloned())
    }
    
    /// Get UTXOs for an address
    pub fn get_utxos_for_address(&self, address: &Address) -> StateBridgeResult<Vec<UTXO>> {
        let state = self.utxo_state.read().unwrap();
        let utxos: Vec<UTXO> = state.utxo_set()
            .get_utxos_for_address(address)
            .iter()
            .map(|&utxo| utxo.clone())
            .collect();
        
        Ok(utxos)
    }
    
    /// Get balance for an address
    pub fn get_balance(&self, address: &Address) -> StateBridgeResult<u64> {
        let state = self.utxo_state.read().unwrap();
        Ok(state.utxo_set().get_balance(address))
    }
    
    /// Spend a UTXO
    pub fn spend_utxo(&self, id: &Hash) -> StateBridgeResult<()> {
        // First check if the UTXO exists and is owned by the caller
        let state = self.utxo_state.read().unwrap();
        let utxo = state.utxo_set().get_utxo(id)
            .ok_or(UTXOError::NotFound)?;
        
        // Only allow spending caller's UTXOs
        if utxo.owner != self.caller {
            return Err(StateBridgeError::PermissionDenied(
                "Can only spend UTXOs owned by the caller".to_string()
            ));
        }
        
        // Add to spent UTXOs list
        let mut spent = self.spent_utxos.lock().unwrap();
        spent.push(*id);
        
        Ok(())
    }
    
    /// Create a new UTXO
    pub fn create_utxo(
        &self,
        value: u64,
        owner: Address,
    ) -> StateBridgeResult<Hash> {
        // Check if contract has enough balance
        let total_spent: u64 = {
            let created = self.created_utxos.lock().unwrap();
            created.iter().map(|utxo| utxo.value).sum()
        };
        
        // Get the contract balance
        let contract_balance = self.get_balance(&self.contract_address)?;
        
        if total_spent + value > contract_balance {
            return Err(StateBridgeError::InvalidOperation(
                "Insufficient contract balance".to_string()
            ));
        }
        
        // Create a fake transaction hash for this UTXO
        let mut tx_hash = [0u8; 32];
        tx_hash[0..20].copy_from_slice(&self.contract_address);
        
        // Create a new UTXO
        let state = self.utxo_state.read().unwrap();
        let height = state.height();
        
        let utxo_ref = UTXORef {
            tx_hash,
            index: 0, // In real implementation, use a proper index
        };
        
        let id = utxo_ref.to_id();
        
        let utxo = UTXO {
            id,
            tx_hash,
            index: 0,
            value,
            owner,
            created_at: height,
            spent: false,
            spent_at: None,
        };
        
        // Add to created UTXOs list
        let mut created = self.created_utxos.lock().unwrap();
        created.push(utxo);
        
        Ok(id)
    }
    
    /// Commit all changes to the UTXO state
    pub fn commit(&self) -> StateBridgeResult<()> {
        let mut state = self.utxo_state.write().unwrap();
        
        // Spend all UTXOs first
        let spent = self.spent_utxos.lock().unwrap();
        for id in &*spent {
            state.utxo_set_mut().spend_utxo(id)?;
        }
        
        // Then add all created UTXOs
        let created = self.created_utxos.lock().unwrap();
        for utxo in &*created {
            let tx_hash = utxo.tx_hash;
            let index = utxo.index;
            let value = utxo.value;
            let owner = utxo.owner;
            
            state.utxo_set_mut().add_utxo(tx_hash, index, value, owner);
        }
        
        Ok(())
    }
    
    /// Create a transaction from the bridge operations
    pub fn create_transaction(&self) -> StateBridgeResult<Option<Transaction>> {
        let spent = self.spent_utxos.lock().unwrap();
        let created = self.created_utxos.lock().unwrap();
        
        if spent.is_empty() && created.is_empty() {
            return Ok(None);
        }
        
        // Create transaction inputs
        let inputs: Vec<TxIn> = spent.iter()
            .map(|id| TxIn { utxo_id: *id })
            .collect();
        
        // Create transaction outputs
        let outputs: Vec<TxOut> = created.iter()
            .map(|utxo| TxOut {
                value: utxo.value,
                recipient: utxo.owner,
            })
            .collect();
        
        // Create the transaction
        let tx = Transaction {
            from: self.caller,
            to: Some(self.contract_address),
            value: 0, // Value is already in outputs
            gas_limit: 0, // Not relevant for this transaction
            gas_price: 0, // Not relevant for this transaction
            nonce: 0, // Would come from the actual state
            data: Vec::new(), // No contract call data needed
            inputs,
            outputs,
        };
        
        Ok(Some(tx))
    }
}

impl StateAccess for UTXOStateBridge {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // First check cache
        let cache = self.state_cache.lock().unwrap();
        if let Some(value) = cache.get(key) {
            return Some(value.clone());
        }
        
        // If not in cache, try to interpret key as a UTXO ID or other state request
        None
    }
    
    fn set(&self, key: &[u8], value: &[u8]) -> Result<(), StateError> {
        // Store in cache
        let mut cache = self.state_cache.lock().unwrap();
        cache.insert(key.to_vec(), value.to_vec());
        Ok(())
    }
    
    fn remove(&self, key: &[u8]) -> Result<(), StateError> {
        // Remove from cache
        let mut cache = self.state_cache.lock().unwrap();
        cache.remove(key);
        Ok(())
    }
    
    fn contains(&self, key: &[u8]) -> bool {
        // Check cache
        let cache = self.state_cache.lock().unwrap();
        cache.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::core::state::UTXOState;
    
    #[test]
    fn test_basic_state_bridge() {
        // Create UTXO state
        let utxo_state = Arc::new(RwLock::new(UTXOState::new()));
        
        // Create addresses
        let contract_address = [1u8; 20];
        let caller_address = [2u8; 20];
        
        // Create state bridge
        let bridge = UTXOStateBridge::new(
            utxo_state.clone(),
            contract_address,
            caller_address,
        );
        
        // Test setting and getting state
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        
        bridge.set(&key, &value).unwrap();
        assert_eq!(bridge.get(&key), Some(value));
        
        // Test removing state
        bridge.remove(&key).unwrap();
        assert_eq!(bridge.get(&key), None);
    }
} 