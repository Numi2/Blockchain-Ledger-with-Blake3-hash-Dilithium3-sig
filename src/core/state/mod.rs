pub mod utxo;
pub mod merkle;
mod state;

pub use state::WorldState;
pub use utxo::{UTXOSet, UTXO, UTXORef, UTXOError, UTXOResult, MerkleProof};

/// The core state interface that all state implementations must provide
pub trait State {
    /// Apply a transaction to the state
    fn apply_transaction(&mut self, tx: &crate::types::Transaction) -> Result<(), String>;
    
    /// Get the state root hash
    fn state_root(&self) -> crate::types::Hash;
    
    /// Commit changes to state
    fn commit(&mut self) -> Result<(), String>;
    
    /// Reset the state to a previous root
    fn reset(&mut self, root: &crate::types::Hash) -> Result<(), String>;
    
    /// Generate a proof for a key in the state
    fn generate_proof(&self, key: &[u8]) -> Option<Vec<u8>>;
    
    /// Verify a proof for a key and value
    fn verify_proof(&self, key: &[u8], value: &[u8], proof: &[u8]) -> bool;
}

/// Implementation of State trait using the UTXO model
pub struct UTXOState {
    utxo_set: UTXOSet,
    current_height: u64,
}

impl UTXOState {
    /// Create a new UTXO-based state
    pub fn new() -> Self {
        Self {
            utxo_set: UTXOSet::new(),
            current_height: 0,
        }
    }
    
    /// Set the current block height
    pub fn set_height(&mut self, height: u64) {
        self.current_height = height;
        self.utxo_set.set_height(height);
    }
    
    /// Get the current block height
    pub fn height(&self) -> u64 {
        self.current_height
    }
    
    /// Get the underlying UTXO set
    pub fn utxo_set(&self) -> &UTXOSet {
        &self.utxo_set
    }
    
    /// Get a mutable reference to the underlying UTXO set
    pub fn utxo_set_mut(&mut self) -> &mut UTXOSet {
        &mut self.utxo_set
    }
}

impl State for UTXOState {
    fn apply_transaction(&mut self, tx: &crate::types::Transaction) -> Result<(), String> {
        self.utxo_set.apply_transaction(tx)
            .map_err(|e| format!("UTXO error: {:?}", e))
    }
    
    fn state_root(&self) -> crate::types::Hash {
        self.utxo_set.merkle_root()
    }
    
    fn commit(&mut self) -> Result<(), String> {
        // UTXO model doesn't need explicit commits as it's applied immediately
        Ok(())
    }
    
    fn reset(&mut self, root: &crate::types::Hash) -> Result<(), String> {
        // In a real implementation, we would restore the UTXO set to the state with the given root
        Err("Reset not implemented for UTXO state".to_string())
    }
    
    fn generate_proof(&self, key: &[u8]) -> Option<Vec<u8>> {
        // Convert key to UTXO ID (assuming key is a serialized UTXO ID)
        if key.len() != 32 {
            return None;
        }
        
        let mut id = [0u8; 32];
        id.copy_from_slice(key);
        
        // Generate and serialize the proof
        self.utxo_set.generate_proof(&id)
            .map(|proof| bincode::serialize(&proof).ok())
            .flatten()
    }
    
    fn verify_proof(&self, key: &[u8], value: &[u8], proof_bytes: &[u8]) -> bool {
        // Deserialize the proof
        let proof: MerkleProof = match bincode::deserialize(proof_bytes) {
            Ok(p) => p,
            Err(_) => return false,
        };
        
        // Verify the proof
        proof.verify()
    }
}

/// Account stores an individual account state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    /// The account nonce (transaction count)
    pub nonce: u64,
    
    /// The account balance
    pub balance: u64,
    
    /// The account storage (key-value map)
    pub storage: HashMap<Hash, Hash>,
    
    /// The account code (for contracts)
    pub code: Vec<u8>,
}

impl Account {
    /// Create a new empty account
    pub fn new() -> Self {
        Self {
            nonce: 0,
            balance: 0,
            storage: HashMap::new(),
            code: Vec::new(),
        }
    }
    
    /// Create a new account with the given balance
    pub fn with_balance(balance: u64) -> Self {
        Self {
            nonce: 0,
            balance,
            storage: HashMap::new(),
            code: Vec::new(),
        }
    }
    
    /// Increment the account nonce
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
    
    /// Update the account balance
    pub fn update_balance(&mut self, amount: i64) -> bool {
        if amount < 0 && self.balance < amount.unsigned_abs() {
            return false; // Insufficient funds
        }
        
        if amount < 0 {
            self.balance -= amount.unsigned_abs();
        } else {
            self.balance += amount as u64;
        }
        
        true
    }
    
    /// Set a storage key
    pub fn set_storage(&mut self, key: Hash, value: Hash) {
        self.storage.insert(key, value);
    }
    
    /// Get a storage value
    pub fn get_storage(&self, key: &Hash) -> Option<&Hash> {
        self.storage.get(key)
    }
    
    /// Set account code
    pub fn set_code(&mut self, code: Vec<u8>) {
        self.code = code;
    }
    
    /// Check if this is a contract account
    pub fn is_contract(&self) -> bool {
        !self.code.is_empty()
    }
}

/// WorldState represents the entire blockchain state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldState {
    /// Accounts by address
    pub accounts: HashMap<Address, Account>,
    
    /// State version/block height
    pub version: u64,
}

impl WorldState {
    /// Create a new empty world state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            version: 0,
        }
    }
    
    /// Get an account by address
    pub fn get_account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }
    
    /// Get a mutable account, creating it if it doesn't exist
    pub fn get_or_create_account(&mut self, address: &Address) -> &mut Account {
        self.accounts.entry(*address).or_insert_with(Account::new)
    }
    
    /// Delete an account
    pub fn delete_account(&mut self, address: &Address) -> Option<Account> {
        self.accounts.remove(address)
    }
    
    /// Calculate the root hash of the world state
    pub fn state_root(&self) -> Hash {
        // For now we'll use a simple hash of the serialized state
        // In production, this would be a Merkle Patricia Trie root
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        
        // Use BLAKE3 for hashing
        let hash = blake3::hash(&bytes);
        
        // Convert to a fixed-size array
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(hash.as_bytes());
        
        state_root
    }
}

