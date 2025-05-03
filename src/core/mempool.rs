use crate::types::{Address, Hash, Transaction};
use crate::core::state::WorldState;
use crate::core::state::Account;
use std::collections::{HashMap, HashSet, BTreeMap};
use std::time::{Duration, Instant};

/// Transaction verification result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxVerificationResult {
    /// Transaction is valid
    Valid,
    
    /// Transaction is invalid
    Invalid(String),
    
    /// Transaction depends on another transaction
    Dependency(Vec<Hash>),
}

/// Transaction priority information
#[derive(Debug, Clone)]
struct TxPriority {
    /// Gas price
    gas_price: u64,
    
    /// Transaction timestamp
    timestamp: Instant,
    
    /// Transaction hash
    hash: Hash,
}

/// MemPool manages pending transactions
pub struct MemPool {
    /// Pending transactions by hash
    pending_txs: HashMap<Hash, Transaction>,
    
    /// Transaction priorities ordered by gas price (highest first)
    priorities: BTreeMap<u64, Vec<TxPriority>>,
    
    /// Transactions by sender (to track nonces)
    txs_by_sender: HashMap<Address, BTreeMap<u64, Hash>>,
    
    /// Maximum number of transactions
    max_transactions: usize,
    
    /// Current world state (for validation)
    world_state: WorldState,
    
    /// Set of transactions recently removed
    removed_txs: HashSet<Hash>,
    
    /// Transaction timeout (how long to keep them)
    tx_timeout: Duration,
}

impl MemPool {
    /// Create a new mempool
    pub fn new(world_state: WorldState, max_transactions: usize) -> Self {
        Self {
            pending_txs: HashMap::new(),
            priorities: BTreeMap::new(),
            txs_by_sender: HashMap::new(),
            max_transactions,
            world_state,
            removed_txs: HashSet::new(),
            tx_timeout: Duration::from_secs(3600), // 1 hour default
        }
    }
    
    /// Add a transaction to the mempool
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        let tx_hash = tx.hash();
        
        // Check if already processed or removed
        if self.pending_txs.contains_key(&tx_hash) || self.removed_txs.contains(&tx_hash) {
            return Err("Transaction already in mempool or recently removed".to_string());
        }
        
        // Verify transaction
        match self.verify_transaction(&tx) {
            TxVerificationResult::Valid => {},
            TxVerificationResult::Invalid(reason) => {
                return Err(format!("Invalid transaction: {}", reason));
            },
            TxVerificationResult::Dependency(deps) => {
                return Err(format!("Transaction depends on {} other transactions", deps.len()));
            }
        }
        
        // Add to pending transactions
        self.pending_txs.insert(tx_hash, tx.clone());
        
        // Add to priorities
        let priority = TxPriority {
            gas_price: tx.gas_price,
            timestamp: Instant::now(),
            hash: tx_hash,
        };
        
        self.priorities
            .entry(tx.gas_price)
            .or_insert_with(Vec::new)
            .push(priority);
        
        // Add to sender map (for nonce tracking)
        self.txs_by_sender
            .entry(tx.from)
            .or_insert_with(BTreeMap::new)
            .insert(tx.nonce, tx_hash);
        
        // Clean up if we're over capacity
        if self.pending_txs.len() > self.max_transactions {
            self.remove_lowest_priority_tx();
        }
        
        Ok(())
    }
    
    /// Remove lowest priority transaction
    fn remove_lowest_priority_tx(&mut self) {
        // Find lowest gas price entry
        if let Some((gas_price, txs)) = self.priorities.iter_mut().next() {
            let gas_price = *gas_price;
            
            // Find oldest transaction with this gas price
            if let Some(oldest_idx) = txs
                .iter()
                .enumerate()
                .min_by_key(|(_, tx)| tx.timestamp)
                .map(|(idx, _)| idx)
            {
                let tx_to_remove = txs.remove(oldest_idx);
                
                // If this was the last transaction with this gas price, remove the entry
                if txs.is_empty() {
                    self.priorities.remove(&gas_price);
                }
                
                // Remove from pending_txs
                if let Some(tx) = self.pending_txs.remove(&tx_to_remove.hash) {
                    // Remove from txs_by_sender
                    if let Some(txs_map) = self.txs_by_sender.get_mut(&tx.from) {
                        txs_map.remove(&tx.nonce);
                        
                        // If this was the last transaction for this sender, remove the entry
                        if txs_map.is_empty() {
                            self.txs_by_sender.remove(&tx.from);
                        }
                    }
                    
                    // Add to removed_txs
                    self.removed_txs.insert(tx_to_remove.hash);
                }
            }
        }
    }
    
    /// Get transactions for a new block (sorted by priority)
    pub fn get_transactions(&self, max_count: usize) -> Vec<Transaction> {
        let mut result = Vec::with_capacity(max_count);
        let mut used_senders = HashMap::<Address, u64>::new();
        
        // Iterate from highest to lowest gas price
        for (_gas_price, txs) in self.priorities.iter().rev() {
            for tx_priority in txs {
                if result.len() >= max_count {
                    break;
                }
                
                if let Some(tx) = self.pending_txs.get(&tx_priority.hash) {
                    // Check if we already used a transaction from this sender
                    if let Some(nonce) = used_senders.get(&tx.from) {
                        // Only include if the nonce is sequential
                        if tx.nonce != *nonce + 1 {
                            continue;
                        }
                    } else {
                        // Check if this is the lowest nonce for this sender
                        if let Some(sender_txs) = self.txs_by_sender.get(&tx.from) {
                            let lowest_nonce = *sender_txs.keys().next().unwrap_or(&0);
                            if tx.nonce != lowest_nonce {
                                continue;
                            }
                        }
                    }
                    
                    // Add transaction
                    result.push(tx.clone());
                    
                    // Update used_senders
                    used_senders.insert(tx.from, tx.nonce);
                }
            }
        }
        
        result
    }
    
    /// Remove transactions from the mempool
    pub fn remove_transactions(&mut self, tx_hashes: &[Hash]) {
        for hash in tx_hashes {
            if let Some(tx) = self.pending_txs.remove(hash) {
                // Remove from txs_by_sender
                if let Some(txs_map) = self.txs_by_sender.get_mut(&tx.from) {
                    txs_map.remove(&tx.nonce);
                    
                    // If this was the last transaction for this sender, remove the entry
                    if txs_map.is_empty() {
                        self.txs_by_sender.remove(&tx.from);
                    }
                }
                
                // Add to removed_txs
                self.removed_txs.insert(*hash);
            }
        }
        
        // Rebuild priorities (not ideal, but simple)
        self.rebuild_priorities();
    }
    
    /// Rebuild the priorities index
    fn rebuild_priorities(&mut self) {
        self.priorities.clear();
        
        for (hash, tx) in &self.pending_txs {
            let priority = TxPriority {
                gas_price: tx.gas_price,
                timestamp: Instant::now(), // We lose the original timestamp, but it's acceptable
                hash: *hash,
            };
            
            self.priorities
                .entry(tx.gas_price)
                .or_insert_with(Vec::new)
                .push(priority);
        }
    }
    
    /// Prune expired transactions
    pub fn prune_expired(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        
        // Find expired transactions
        for (gas_price, txs) in &mut self.priorities {
            let mut expired_indices = Vec::new();
            
            for (idx, tx_priority) in txs.iter().enumerate() {
                if now.duration_since(tx_priority.timestamp) > self.tx_timeout {
                    expired_indices.push(idx);
                    to_remove.push(tx_priority.hash);
                }
            }
            
            // Remove expired transactions (in reverse order to maintain indices)
            for idx in expired_indices.iter().rev() {
                txs.remove(*idx);
            }
        }
        
        // Clean up empty priority buckets
        self.priorities.retain(|_, txs| !txs.is_empty());
        
        // Remove from other collections
        for hash in to_remove {
            if let Some(tx) = self.pending_txs.remove(&hash) {
                // Remove from txs_by_sender
                if let Some(txs_map) = self.txs_by_sender.get_mut(&tx.from) {
                    txs_map.remove(&tx.nonce);
                    
                    // If this was the last transaction for this sender, remove the entry
                    if txs_map.is_empty() {
                        self.txs_by_sender.remove(&tx.from);
                    }
                }
            }
        }
    }
    
    /// Update the world state (e.g., after a new block)
    pub fn update_world_state(&mut self, world_state: WorldState) {
        self.world_state = world_state;
        
        // Re-verify all transactions
        let mut invalid_txs = Vec::new();
        
        for (hash, tx) in &self.pending_txs {
            match self.verify_transaction(tx) {
                TxVerificationResult::Valid => {},
                _ => {
                    invalid_txs.push(*hash);
                }
            }
        }
        
        // Remove invalid transactions
        self.remove_transactions(&invalid_txs);
    }
    
    /// Verify a transaction against the current state
    fn verify_transaction(&self, tx: &Transaction) -> TxVerificationResult {
        // Check if sender exists
        let sender_account = match self.world_state.get_account(&tx.from) {
            Some(account) => account,
            None => {
                // In some blockchain models, non-existent accounts are valid with 0 balance
                return if tx.value > 0 || tx.gas_limit * tx.gas_price > 0 {
                    TxVerificationResult::Invalid("Sender account does not exist or has insufficient funds".to_string())
                } else {
                    TxVerificationResult::Valid
                }
            }
        };
        
        // Check nonce
        let expected_nonce = sender_account.nonce;
        
        if tx.nonce < expected_nonce {
            return TxVerificationResult::Invalid(format!(
                "Transaction nonce {} is lower than current nonce {}",
                tx.nonce, expected_nonce
            ));
        }
        
        if tx.nonce > expected_nonce {
            // This transaction depends on others
            // Find the hashes of the transactions it depends on
            let mut dependencies = Vec::new();
            
            if let Some(sender_txs) = self.txs_by_sender.get(&tx.from) {
                for nonce in expected_nonce..tx.nonce {
                    if let Some(hash) = sender_txs.get(&nonce) {
                        dependencies.push(*hash);
                    } else {
                        // Missing a dependency
                        return TxVerificationResult::Invalid(format!(
                            "Missing transaction with nonce {} for sender",
                            nonce
                        ));
                    }
                }
            } else {
                // No pending transactions from this sender
                return TxVerificationResult::Invalid(format!(
                    "Transaction nonce {} is higher than current nonce {} with no pending transactions",
                    tx.nonce, expected_nonce
                ));
            }
            
            return TxVerificationResult::Dependency(dependencies);
        }
        
        // Check balance
        let total_cost = tx.value + tx.gas_limit * tx.gas_price;
        
        if sender_account.balance < total_cost {
            return TxVerificationResult::Invalid(format!(
                "Insufficient funds: have {}, need {}",
                sender_account.balance, total_cost
            ));
        }
        
        // Additional checks (e.g., signature verification) would go here
        
        TxVerificationResult::Valid
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, hash: &Hash) -> Option<&Transaction> {
        self.pending_txs.get(hash)
    }
    
    /// Get pending transaction count
    pub fn pending_count(&self) -> usize {
        self.pending_txs.len()
    }
    
    /// Set transaction timeout
    pub fn set_transaction_timeout(&mut self, timeout: Duration) {
        self.tx_timeout = timeout;
    }
} 