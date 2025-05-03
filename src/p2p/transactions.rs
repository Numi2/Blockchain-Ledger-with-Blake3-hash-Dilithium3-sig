use crate::types::Transaction;
use crate::core::validation::{TransactionValidator, ValidationResult};
use libp2p::{
    gossipsub::{Gossipsub, GossipsubEvent, GossipsubMessage, MessageAuthenticity, MessageId, ValidationMode},
    identity::Keypair,
    PeerId,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Transaction pool errors
#[derive(Error, Debug)]
pub enum TxPoolError {
    #[error("Transaction already exists")]
    AlreadyExists,
    
    #[error("Transaction pool full")]
    PoolFull,
    
    #[error("Invalid transaction: {0}")]
    InvalidTx(String),
    
    #[error("Rate limit exceeded")]
    RateLimited,
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for transaction pool operations
pub type TxPoolResult<T> = Result<T, TxPoolError>;

/// Transaction propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxPropagationConfig {
    /// Maximum transactions in the pending pool
    pub max_pool_size: usize,
    
    /// Maximum transaction size in bytes
    pub max_tx_size: usize,
    
    /// Rate limit in transactions per minute per peer
    pub rate_limit_tx_per_min: u32,
    
    /// Rate limit in bytes per minute per peer
    pub rate_limit_bytes_per_min: u64,
    
    /// Timeout for seen transactions cache in seconds
    pub seen_tx_timeout_secs: u64,
    
    /// Number of peers to propagate each transaction to
    pub fanout: usize,
    
    /// Maximum retry attempts for propagation
    pub max_retries: u32,
    
    /// Gossipsub topic for transaction propagation
    pub gossip_topic: String,
}

impl Default for TxPropagationConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 5000,
            max_tx_size: 100 * 1024, // 100 KB
            rate_limit_tx_per_min: 100,
            rate_limit_bytes_per_min: 1024 * 1024, // 1 MB
            seen_tx_timeout_secs: 300, // 5 minutes
            fanout: 3,
            max_retries: 3,
            gossip_topic: "world-ledger/transactions/1".to_string(),
        }
    }
}

/// Peer rate limit statistics
#[derive(Debug, Clone)]
struct PeerRateLimits {
    /// Transactions received in current window
    tx_count: u32,
    
    /// Bytes received in current window
    bytes_count: u64,
    
    /// Window start time
    window_start: Instant,
}

impl PeerRateLimits {
    /// Create new rate limit stats
    fn new() -> Self {
        Self {
            tx_count: 0,
            bytes_count: 0,
            window_start: Instant::now(),
        }
    }
    
    /// Reset the window
    fn reset(&mut self) {
        self.tx_count = 0;
        self.bytes_count = 0;
        self.window_start = Instant::now();
    }
    
    /// Check if window should be reset (after 1 minute)
    fn check_window_reset(&mut self) {
        if self.window_start.elapsed() >= Duration::from_secs(60) {
            self.reset();
        }
    }
    
    /// Add a transaction to the rate limit
    fn add_tx(&mut self, size: usize) {
        self.check_window_reset();
        self.tx_count += 1;
        self.bytes_count += size as u64;
    }
    
    /// Check if peer is rate limited
    fn is_rate_limited(&mut self, config: &TxPropagationConfig, tx_size: usize) -> bool {
        self.check_window_reset();
        
        // Check transaction count limit
        if self.tx_count >= config.rate_limit_tx_per_min {
            return true;
        }
        
        // Check bytes limit
        if self.bytes_count + (tx_size as u64) > config.rate_limit_bytes_per_min {
            return true;
        }
        
        false
    }
}

/// Transaction propagation manager
pub struct TransactionPropagator {
    /// Configuration
    config: RwLock<TxPropagationConfig>,
    
    /// Pending transactions to be propagated
    pending_txs: Mutex<HashMap<[u8; 32], Transaction>>,
    
    /// Seen transaction hashes with timestamp
    seen_txs: Mutex<HashMap<[u8; 32], Instant>>,
    
    /// Peer rate limit tracking
    peer_limits: Mutex<HashMap<PeerId, PeerRateLimits>>,
    
    /// Transaction validator
    validator: Arc<TransactionValidator>,
    
    /// Gossipsub instance
    gossipsub: Arc<RwLock<Gossipsub>>,
    
    /// Propagation work queue sender
    work_tx: Mutex<Option<Sender<([u8; 32], u32)>>>,
    
    /// Peers to propagate to (best peers first)
    peers: RwLock<Vec<PeerId>>,
}

impl TransactionPropagator {
    /// Create a new transaction propagator
    pub fn new(
        config: TxPropagationConfig,
        validator: Arc<TransactionValidator>,
        gossipsub: Arc<RwLock<Gossipsub>>,
    ) -> Self {
        Self {
            config: RwLock::new(config),
            pending_txs: Mutex::new(HashMap::new()),
            seen_txs: Mutex::new(HashMap::new()),
            peer_limits: Mutex::new(HashMap::new()),
            validator,
            gossipsub,
            work_tx: Mutex::new(None),
            peers: RwLock::new(Vec::new()),
        }
    }
    
    /// Start the propagation worker
    pub async fn start(&self) {
        // Create channel for work queue
        let (tx, rx) = mpsc::channel(100);
        
        // Store sender
        let mut work_tx = self.work_tx.lock().unwrap();
        *work_tx = Some(tx);
        
        // Start worker
        let gossipsub = self.gossipsub.clone();
        let config = self.config();
        let pending_txs = self.pending_txs.clone();
        let peers = self.peers.clone();
        
        tokio::spawn(async move {
            Self::propagation_worker(rx, gossipsub, config, pending_txs, peers).await;
        });
        
        // Start cleanup task
        let seen_txs = self.seen_txs.clone();
        let config_clone = self.config();
        
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                Self::cleanup_seen_txs(seen_txs.clone(), config_clone.seen_tx_timeout_secs);
            }
        });
    }
    
    /// Stop the propagation worker
    pub fn stop(&self) {
        let mut work_tx = self.work_tx.lock().unwrap();
        *work_tx = None;
    }
    
    /// Get the configuration
    pub fn config(&self) -> TxPropagationConfig {
        let config = self.config.read().unwrap();
        config.clone()
    }
    
    /// Set the configuration
    pub fn set_config(&self, config: TxPropagationConfig) {
        let mut cfg = self.config.write().unwrap();
        *cfg = config;
    }
    
    /// Update the list of peers
    pub fn update_peers(&self, new_peers: Vec<PeerId>) {
        let mut peers = self.peers.write().unwrap();
        *peers = new_peers;
    }
    
    /// Add a transaction to the pool
    pub fn add_transaction(&self, tx: Transaction) -> TxPoolResult<()> {
        // Get tx hash
        let tx_hash = tx.hash();
        
        // Check if we've seen this transaction before
        {
            let seen = self.seen_txs.lock().unwrap();
            if seen.contains_key(&tx_hash) {
                return Err(TxPoolError::AlreadyExists);
            }
        }
        
        // Validate transaction format
        self.validator.validate_transaction_format(&tx)
            .map_err(|e| TxPoolError::InvalidTx(format!("{}", e)))?;
        
        // Compute serialized size
        let tx_size = bincode::serialized_size(&tx)
            .map_err(|e| TxPoolError::SerializationError(e.to_string()))? as usize;
            
        // Check transaction size
        let config = self.config();
        if tx_size > config.max_tx_size {
            return Err(TxPoolError::InvalidTx(format!("Transaction too large: {} bytes", tx_size)));
        }
        
        // Check pool size
        {
            let pending = self.pending_txs.lock().unwrap();
            if pending.len() >= config.max_pool_size {
                return Err(TxPoolError::PoolFull);
            }
        }
        
        // Add to pending transactions
        {
            let mut pending = self.pending_txs.lock().unwrap();
            pending.insert(tx_hash, tx);
        }
        
        // Add to seen transactions
        {
            let mut seen = self.seen_txs.lock().unwrap();
            seen.insert(tx_hash, Instant::now());
        }
        
        // Queue for propagation
        let work_tx = self.work_tx.lock().unwrap();
        if let Some(tx) = &*work_tx {
            let _ = tx.try_send((tx_hash, 0)); // Retry count 0
        }
        
        Ok(())
    }
    
    /// Handle an incoming transaction from the network
    pub fn handle_incoming_tx(&self, peer_id: &PeerId, data: &[u8]) -> TxPoolResult<()> {
        // Deserialize transaction
        let tx: Transaction = bincode::deserialize(data)
            .map_err(|e| TxPoolError::SerializationError(e.to_string()))?;
            
        // Check transaction size
        let config = self.config();
        if data.len() > config.max_tx_size {
            return Err(TxPoolError::InvalidTx(format!("Transaction too large: {} bytes", data.len())));
        }
        
        // Check rate limits
        {
            let mut limits = self.peer_limits.lock().unwrap();
            let peer_limit = limits.entry(*peer_id).or_insert_with(PeerRateLimits::new);
            
            if peer_limit.is_rate_limited(&config, data.len()) {
                return Err(TxPoolError::RateLimited);
            }
            
            // Update rate limit stats
            peer_limit.add_tx(data.len());
        }
        
        // Add transaction (which will validate it)
        self.add_transaction(tx)
    }
    
    /// Handle a gossipsub message
    pub fn handle_gossipsub_message(&self, peer_id: &PeerId, message: &GossipsubMessage) -> TxPoolResult<()> {
        // Check topic
        let config = self.config();
        if message.topic.as_str() != config.gossip_topic {
            return Ok(()); // Ignore messages from other topics
        }
        
        // Handle as incoming transaction
        self.handle_incoming_tx(peer_id, &message.data)
    }
    
    /// Get all pending transactions
    pub fn get_pending_txs(&self) -> Vec<Transaction> {
        let pending = self.pending_txs.lock().unwrap();
        pending.values().cloned().collect()
    }
    
    /// Remove transactions that have been included in a block
    pub fn remove_txs(&self, tx_hashes: &[[u8; 32]]) {
        let mut pending = self.pending_txs.lock().unwrap();
        
        for hash in tx_hashes {
            pending.remove(hash);
        }
    }
    
    /// Propagation worker task
    async fn propagation_worker(
        mut rx: Receiver<([u8; 32], u32)>,
        gossipsub: Arc<RwLock<Gossipsub>>,
        config: TxPropagationConfig,
        pending_txs: Mutex<HashMap<[u8; 32], Transaction>>,
        peers: RwLock<Vec<PeerId>>,
    ) {
        while let Some((tx_hash, retry_count)) = rx.recv().await {
            // Get transaction from pending pool
            let tx = {
                let pending = pending_txs.lock().unwrap();
                match pending.get(&tx_hash) {
                    Some(tx) => tx.clone(),
                    None => continue, // Transaction no longer in pool
                }
            };
            
            // Serialize transaction
            let tx_data = match bincode::serialize(&tx) {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to serialize transaction: {}", e);
                    continue;
                }
            };
            
            // Get gossipsub
            let mut gs = gossipsub.write().unwrap();
            
            // Publish to gossipsub
            match gs.publish(config.gossip_topic.clone(), tx_data) {
                Ok(_) => {
                    debug!("Published transaction to gossipsub: {:?}", tx_hash);
                }
                Err(e) => {
                    // Failed to publish, retry if under max retries
                    error!("Failed to publish transaction: {}", e);
                    
                    if retry_count < config.max_retries {
                        // Re-queue with increased retry count
                        drop(gs); // Release lock before async operation
                        
                        let tx_sender = rx.clone();
                        let tx_hash_clone = tx_hash;
                        let new_retry = retry_count + 1;
                        
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(1)).await;
                            let _ = tx_sender.send((tx_hash_clone, new_retry)).await;
                        });
                    }
                }
            }
        }
    }
    
    /// Clean up old transactions from the seen cache
    fn cleanup_seen_txs(seen_txs: Mutex<HashMap<[u8; 32], Instant>>, timeout_secs: u64) {
        let mut seen = seen_txs.lock().unwrap();
        let timeout = Duration::from_secs(timeout_secs);
        
        // Remove old entries
        seen.retain(|_, time| time.elapsed() < timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TxIn, TxOut};
    use crate::core::validation::TransactionValidator;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    
    // Generate a random transaction
    fn create_test_transaction(inputs: Vec<TxIn>, outputs: Vec<TxOut>) -> Transaction {
        let mut rng = ChaCha20Rng::seed_from_u64(12345);
        
        Transaction {
            from: [rng.gen(); 20],
            to: Some([rng.gen(); 20]),
            value: 100,
            gas_limit: 21000,
            gas_price: 1_000_000_000,
            nonce: 0,
            data: Vec::new(),
            inputs,
            outputs,
        }
    }
    
    #[test]
    fn test_rate_limits() {
        let config = TxPropagationConfig {
            rate_limit_tx_per_min: 5,
            rate_limit_bytes_per_min: 1000,
            ..Default::default()
        };
        
        let mut rate_limit = PeerRateLimits::new();
        
        // Add transactions up to limit
        for _ in 0..5 {
            assert!(!rate_limit.is_rate_limited(&config, 100));
            rate_limit.add_tx(100);
        }
        
        // Should be rate limited now
        assert!(rate_limit.is_rate_limited(&config, 100));
        
        // Reset window
        rate_limit.reset();
        
        // Should work again
        assert!(!rate_limit.is_rate_limited(&config, 100));
        
        // Add large transaction approaching byte limit
        assert!(!rate_limit.is_rate_limited(&config, 900));
        rate_limit.add_tx(900);
        
        // Next transaction exceeding byte limit should be limited
        assert!(rate_limit.is_rate_limited(&config, 200));
        
        // Small transaction still limited
        assert!(rate_limit.is_rate_limited(&config, 50));
    }
} 