use crate::types::{Hash, Slot};
use crate::core::block::{Block, BlockHeader};
use crate::core::state::WorldState;
use crate::consensus::types::{Consensus, BlockValidationResult};
use crate::p2p::protocol::SyncStatus;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Maximum number of block requests in flight
const MAX_BLOCK_REQUESTS: usize = 64;

/// Block download timeout (seconds)
const BLOCK_REQUEST_TIMEOUT: u64 = 30;

/// State sync batch size
const STATE_SYNC_BATCH_SIZE: usize = 1000;

/// Block request information
#[derive(Debug, Clone)]
struct BlockRequest {
    /// Block hash being requested
    hash: Hash,
    
    /// When the request was sent
    timestamp: Instant,
    
    /// Number of retries
    retries: u32,
}

/// Chain synchronization status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    /// Not synchronizing
    Idle,
    
    /// Downloading block headers
    HeaderSync,
    
    /// Downloading block bodies
    BodySync,
    
    /// Downloading state
    StateSync,
    
    /// Processing downloaded blocks
    Processing,
    
    /// Sync completed
    Complete,
    
    /// Sync failed
    Failed(String),
}

/// Chain synchronization manager
pub struct ChainSync<C: Consensus> {
    /// Current sync state
    state: SyncState,
    
    /// Consensus mechanism
    consensus: Arc<RwLock<C>>,
    
    /// Current world state
    world_state: Arc<RwLock<WorldState>>,
    
    /// Block requests in progress
    block_requests: HashMap<Hash, BlockRequest>,
    
    /// Block headers to download (queue)
    header_queue: VecDeque<Hash>,
    
    /// Block bodies to download (queue)
    body_queue: VecDeque<Hash>,
    
    /// Downloaded blocks pending processing
    pending_blocks: HashMap<Hash, Block>,
    
    /// Processed block hashes
    processed_blocks: HashSet<Hash>,
    
    /// Known headers by hash
    known_headers: HashMap<Hash, BlockHeader>,
    
    /// Headers by height
    headers_by_height: HashMap<u64, Vec<Hash>>,
    
    /// Target sync head
    target_head: Option<BlockHeader>,
    
    /// Local head before sync started
    local_head: Option<BlockHeader>,
    
    /// Sync start time
    start_time: Instant,
    
    /// Header download progress
    header_progress: (usize, usize), // (downloaded, total)
    
    /// Body download progress
    body_progress: (usize, usize), // (downloaded, total)
    
    /// State download progress
    state_progress: (usize, usize), // (downloaded, total)
}

impl<C: Consensus> ChainSync<C> {
    /// Create a new chain sync manager
    pub fn new(consensus: Arc<RwLock<C>>, world_state: Arc<RwLock<WorldState>>) -> Self {
        Self {
            state: SyncState::Idle,
            consensus,
            world_state,
            block_requests: HashMap::new(),
            header_queue: VecDeque::new(),
            body_queue: VecDeque::new(),
            pending_blocks: HashMap::new(),
            processed_blocks: HashSet::new(),
            known_headers: HashMap::new(),
            headers_by_height: HashMap::new(),
            target_head: None,
            local_head: None,
            start_time: Instant::now(),
            header_progress: (0, 0),
            body_progress: (0, 0),
            state_progress: (0, 0),
        }
    }
    
    /// Start chain synchronization
    pub fn start_sync(&mut self, target_head: BlockHeader) -> Result<(), String> {
        // Check if already syncing
        if self.state != SyncState::Idle && self.state != SyncState::Complete {
            return Err("Sync already in progress".to_string());
        }
        
        // Reset sync state
        self.block_requests.clear();
        self.header_queue.clear();
        self.body_queue.clear();
        self.pending_blocks.clear();
        self.known_headers.clear();
        self.headers_by_height.clear();
        
        // Store target and local heads
        self.target_head = Some(target_head.clone());
        
        // Get current consensus status
        let consensus_status = {
            let consensus = self.consensus.read().unwrap();
            consensus.status()
        };
        
        // Get local head from consensus
        let local_head_hash = consensus_status.head_hash;
        let local_height = consensus_status.current_height;
        
        // Initialize sync
        self.state = SyncState::HeaderSync;
        self.start_time = Instant::now();
        
        // Add target head to known headers
        self.known_headers.insert(target_head.prev_hash, target_head.clone());
        
        // Setup header sync
        let mut current_hash = target_head.prev_hash;
        let mut height = target_head.height;
        
        // Estimate total headers to download
        let headers_to_download = target_head.height.saturating_sub(local_height);
        self.header_progress = (0, headers_to_download as usize);
        
        // Queue headers to download (from newest to oldest)
        while height > local_height {
            // Add to download queue
            self.header_queue.push_back(current_hash);
            
            // Move to previous header
            if let Some(header) = self.known_headers.get(&current_hash) {
                current_hash = header.prev_hash;
                height -= 1;
            } else {
                // We don't have this header yet, it will be queued after we download it
                break;
            }
        }
        
        Ok(())
    }
    
    /// Process a downloaded block header
    pub fn process_header(&mut self, header: BlockHeader) -> Result<(), String> {
        let hash = header.prev_hash; // Using prev_hash as the block hash
        
        // Check if we were waiting for this header
        if let Some(request) = self.block_requests.remove(&hash) {
            // Add to known headers
            self.known_headers.insert(hash, header.clone());
            
            // Add to headers by height
            self.headers_by_height
                .entry(header.height)
                .or_insert_with(Vec::new)
                .push(hash);
            
            // Update header progress
            self.header_progress.0 += 1;
            
            // Queue previous header if needed
            if !self.processed_blocks.contains(&header.prev_hash) && 
               !self.block_requests.contains_key(&header.prev_hash) &&
               !self.known_headers.contains_key(&header.prev_hash) {
                // Add to header download queue
                self.header_queue.push_back(header.prev_hash);
            }
            
            // Add to body download queue
            self.body_queue.push_back(hash);
            
            // Update body progress estimate
            self.body_progress = (0, self.header_progress.0);
        }
        
        Ok(())
    }
    
    /// Process a downloaded block
    pub fn process_block(&mut self, block: Block) -> Result<(), String> {
        let hash = block.hash();
        
        // Check if we were waiting for this block
        if let Some(request) = self.block_requests.remove(&hash) {
            // Add to pending blocks
            self.pending_blocks.insert(hash, block);
            
            // Update body progress
            self.body_progress.0 += 1;
        }
        
        Ok(())
    }
    
    /// Process downloaded state data
    pub fn process_state_data(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String> {
        // In a real implementation, this would update the world state
        // For now, we'll just update progress
        self.state_progress.0 += 1;
        
        Ok(())
    }
    
    /// Get next block to request
    pub fn get_next_block_request(&mut self) -> Option<Hash> {
        // Check if we have capacity for more requests
        if self.block_requests.len() >= MAX_BLOCK_REQUESTS {
            return None;
        }
        
        let hash = match self.state {
            SyncState::HeaderSync => self.header_queue.pop_front(),
            SyncState::BodySync => self.body_queue.pop_front(),
            _ => None,
        }?;
        
        // Create a new request
        let request = BlockRequest {
            hash,
            timestamp: Instant::now(),
            retries: 0,
        };
        
        // Add to in-flight requests
        self.block_requests.insert(hash, request);
        
        Some(hash)
    }
    
    /// Check for timed out requests
    pub fn check_timeouts(&mut self) -> Vec<Hash> {
        let now = Instant::now();
        let timeout = Duration::from_secs(BLOCK_REQUEST_TIMEOUT);
        let mut timed_out = Vec::new();
        let mut to_retry = Vec::new();
        
        // Find timed out requests
        for (hash, request) in &self.block_requests {
            if now.duration_since(request.timestamp) > timeout {
                if request.retries < 3 {
                    to_retry.push(*hash);
                } else {
                    timed_out.push(*hash);
                }
            }
        }
        
        // Retry requests
        for hash in to_retry {
            if let Some(request) = self.block_requests.get_mut(&hash) {
                request.timestamp = now;
                request.retries += 1;
            }
        }
        
        // Remove failed requests
        for hash in &timed_out {
            self.block_requests.remove(hash);
        }
        
        timed_out
    }
    
    /// Process pending blocks
    pub fn process_pending_blocks(&mut self) -> Result<usize, String> {
        if self.state != SyncState::Processing {
            return Ok(0);
        }
        
        let mut processed = 0;
        
        // Find blocks that can be processed (have parent)
        let mut to_process = Vec::new();
        
        for (hash, block) in &self.pending_blocks {
            // Skip if already processed
            if self.processed_blocks.contains(hash) {
                continue;
            }
            
            // Check if parent is processed
            if self.processed_blocks.contains(&block.header.prev_hash) {
                to_process.push(*hash);
            }
        }
        
        // Sort by height
        to_process.sort_by_key(|hash| {
            self.pending_blocks.get(hash).map(|b| b.header.height).unwrap_or(0)
        });
        
        // Process blocks
        for hash in to_process {
            if let Some(block) = self.pending_blocks.get(&hash) {
                // Validate and process block
                let result = {
                    let mut consensus = self.consensus.write().unwrap();
                    let consensus_header = crate::consensus::types::ConsensusBlockHeader {
                        hash: block.hash(),
                        prev_hash: block.header.prev_hash,
                        height: block.header.height,
                        slot: block.header.slot,
                        timestamp: block.header.timestamp,
                        state_root: block.header.state_root,
                        transactions_root: block.header.transactions_root,
                    };
                    consensus.process_block(&consensus_header)
                };
                
                if result.is_ok() {
                    // Block processed successfully
                    self.processed_blocks.insert(hash);
                    processed += 1;
                    
                    // Update world state with new block
                    {
                        let mut world_state = self.world_state.write().unwrap();
                        
                        // Apply block transactions to state
                        for tx in &block.body.transactions {
                            // In a real implementation, this would validate and apply the transaction
                            // to the world state.
                            // For simplicity, we'll just increment the sender's nonce
                            if let Some(account) = world_state.get_or_create_account(&tx.from) {
                                account.nonce += 1;
                            }
                        }
                    }
                } else {
                    // Failed to process block
                    return Err(format!("Failed to process block: {:?}", result));
                }
            }
        }
        
        Ok(processed)
    }
    
    /// Update sync state based on progress
    pub fn update_state(&mut self) {
        // Check current state
        match self.state {
            SyncState::HeaderSync => {
                // Check if header sync is complete
                if self.header_queue.is_empty() && self.block_requests.is_empty() {
                    self.state = SyncState::BodySync;
                }
            },
            SyncState::BodySync => {
                // Check if body sync is complete
                if self.body_queue.is_empty() && self.block_requests.is_empty() {
                    self.state = SyncState::Processing;
                }
            },
            SyncState::Processing => {
                // Check if processing is complete
                if self.pending_blocks.keys().all(|hash| self.processed_blocks.contains(hash)) {
                    // Check if we need state sync
                    let target_head = self.target_head.as_ref().unwrap();
                    
                    // Get state root from target head
                    let target_state_root = target_head.state_root;
                    
                    // Get current state root
                    let current_state_root = {
                        let world_state = self.world_state.read().unwrap();
                        world_state.state_root()
                    };
                    
                    if target_state_root != current_state_root {
                        // Need state sync
                        self.state = SyncState::StateSync;
                        
                        // Initialize state sync progress
                        self.state_progress = (0, STATE_SYNC_BATCH_SIZE); // Estimation
                    } else {
                        // Sync complete
                        self.state = SyncState::Complete;
                    }
                }
            },
            SyncState::StateSync => {
                // Check if state sync is complete
                if self.state_progress.0 >= self.state_progress.1 {
                    self.state = SyncState::Complete;
                }
            },
            _ => {}
        }
    }
    
    /// Get current sync status
    pub fn status(&self) -> SyncStatus {
        let elapsed = self.start_time.elapsed().as_secs();
        
        SyncStatus {
            state: format!("{:?}", self.state),
            target_height: self.target_head.as_ref().map_or(0, |h| h.height),
            current_height: {
                let consensus = self.consensus.read().unwrap();
                consensus.status().current_height
            },
            headers_downloaded: self.header_progress.0 as u64,
            headers_total: self.header_progress.1 as u64,
            blocks_downloaded: self.body_progress.0 as u64,
            blocks_total: self.body_progress.1 as u64,
            state_objects_downloaded: self.state_progress.0 as u64,
            state_objects_total: self.state_progress.1 as u64,
            elapsed_seconds: elapsed,
            estimated_remaining_seconds: self.estimate_remaining_time(elapsed),
        }
    }
    
    /// Estimate remaining time based on progress
    fn estimate_remaining_time(&self, elapsed_seconds: u64) -> u64 {
        let total_progress = match self.state {
            SyncState::HeaderSync => {
                let (done, total) = self.header_progress;
                if total == 0 { return 0; }
                done as f64 / total as f64
            },
            SyncState::BodySync => {
                let header_weight = 0.3;
                let body_weight = 0.7;
                
                let header_progress = 1.0; // Headers done
                let (body_done, body_total) = self.body_progress;
                let body_progress = if body_total == 0 { 0.0 } else { body_done as f64 / body_total as f64 };
                
                header_weight * header_progress + body_weight * body_progress
            },
            SyncState::StateSync => {
                let header_weight = 0.2;
                let body_weight = 0.3;
                let state_weight = 0.5;
                
                let header_progress = 1.0; // Headers done
                let body_progress = 1.0; // Bodies done
                
                let (state_done, state_total) = self.state_progress;
                let state_progress = if state_total == 0 { 0.0 } else { state_done as f64 / state_total as f64 };
                
                header_weight * header_progress + body_weight * body_progress + state_weight * state_progress
            },
            SyncState::Processing => 0.95, // Almost done
            SyncState::Complete => 1.0,    // Done
            _ => 0.0,
        };
        
        if total_progress <= 0.0 {
            return 0;
        }
        
        // Estimate time remaining
        let time_per_percent = elapsed_seconds as f64 / (total_progress * 100.0);
        let remaining_percent = (1.0 - total_progress) * 100.0;
        
        (time_per_percent * remaining_percent) as u64
    }
} 