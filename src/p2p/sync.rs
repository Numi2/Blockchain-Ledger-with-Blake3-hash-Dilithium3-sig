use crate::core::{Block, BlockHeader};
use crate::p2p::protocol::{BlockRequest, BlockResponse, ChainInfoRequest, ChainInfoResponse};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use thiserror::Error;
use serde::{Serialize, Deserialize};
use libp2p::PeerId;

/// Block sync errors
#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Peer error: {0}")]
    PeerError(String),
    
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Sync aborted")]
    Aborted,
}

/// Result type for sync operations
pub type SyncResult<T> = Result<T, SyncError>;

/// Sync strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStrategy {
    /// Sync from the tip of the chain (for nodes that are close to the tip)
    Tip,
    
    /// Sync headers first, then bodies (for fast initial sync)
    HeadersFirst,
    
    /// Sync full blocks in parallel (for nodes with good bandwidth)
    Parallel,
    
    /// Warp sync using STARK state proofs (fastest, requires trust)
    Warp,
}

impl Default for SyncStrategy {
    fn default() -> Self {
        Self::HeadersFirst
    }
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Maximum number of blocks to request at once
    pub max_blocks_per_request: u32,
    
    /// Maximum number of headers to request at once
    pub max_headers_per_request: u32,
    
    /// Maximum number of concurrent block requests
    pub max_concurrent_requests: u32,
    
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    
    /// Sync strategy
    pub strategy: SyncStrategy,
    
    /// Number of blocks to process in parallel
    pub parallel_blocks: u32,
    
    /// Whether to verify all blocks during sync
    pub verify_all_blocks: bool,
    
    /// Warp sync interval (in blocks)
    pub warp_sync_interval: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_blocks_per_request: 128,
            max_headers_per_request: 512,
            max_concurrent_requests: 5,
            request_timeout_seconds: 30,
            strategy: SyncStrategy::default(),
            parallel_blocks: 10,
            verify_all_blocks: true,
            warp_sync_interval: 10000,
        }
    }
}

/// Sync state for a header-first sync
#[derive(Debug)]
struct HeaderSyncState {
    /// Headers that have been downloaded but not bodies
    headers: Vec<BlockHeader>,
    
    /// Header request ranges (start_height, end_height) that are in progress
    header_requests: HashMap<u64, (u64, u64, Instant)>,
    
    /// Block request hashes that are in progress
    block_requests: HashMap<u64, (Vec<[u8; 32]>, Instant)>,
    
    /// Block hashes that have been processed
    processed_blocks: HashSet<[u8; 32]>,
}

/// Sync state for a parallel sync
#[derive(Debug)]
struct ParallelSyncState {
    /// Request ranges (start_height, end_height) that are in progress
    block_requests: HashMap<u64, (u64, u64, Instant)>,
    
    /// Block hashes that have been processed
    processed_blocks: HashSet<[u8; 32]>,
}

/// Sync state for a warp sync
#[derive(Debug)]
struct WarpSyncState {
    /// Height of the last warp sync point
    last_warp_point: u64,
    
    /// Target height for the next warp sync
    next_warp_target: u64,
    
    /// Whether we're currently warp syncing
    is_warp_syncing: bool,
    
    /// Request ID for the current warp sync
    warp_request_id: Option<u64>,
}

/// Sync state for a tip sync
#[derive(Debug)]
struct TipSyncState {
    /// Request ranges (start_height, end_height) that are in progress
    block_requests: HashMap<u64, (u64, u64, Instant)>,
    
    /// Block hashes that have been processed
    processed_blocks: HashSet<[u8; 32]>,
}

/// Sync manager state
#[derive(Debug)]
enum SyncState {
    /// Not syncing
    Idle,
    
    /// Syncing from the tip
    Tip(TipSyncState),
    
    /// Syncing headers first
    HeadersFirst(HeaderSyncState),
    
    /// Syncing blocks in parallel
    Parallel(ParallelSyncState),
    
    /// Warp syncing
    Warp(WarpSyncState),
}

/// Block sync manager
pub struct SyncManager {
    /// Current sync state
    state: RwLock<SyncState>,
    
    /// Sync configuration
    config: RwLock<SyncConfig>,
    
    /// Current blockchain height
    current_height: RwLock<u64>,
    
    /// Target blockchain height
    target_height: RwLock<u64>,
    
    /// Best peers for syncing
    best_peers: RwLock<HashMap<PeerId, (u64, Instant)>>,
    
    /// Next request ID
    next_request_id: Mutex<u64>,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(config: SyncConfig, current_height: u64) -> Self {
        Self {
            state: RwLock::new(SyncState::Idle),
            config: RwLock::new(config),
            current_height: RwLock::new(current_height),
            target_height: RwLock::new(current_height),
            best_peers: RwLock::new(HashMap::new()),
            next_request_id: Mutex::new(0),
        }
    }
    
    /// Set the sync configuration
    pub fn set_config(&self, config: SyncConfig) {
        let mut cfg = self.config.write().unwrap();
        *cfg = config;
    }
    
    /// Get the sync configuration
    pub fn config(&self) -> SyncConfig {
        let cfg = self.config.read().unwrap();
        cfg.clone()
    }
    
    /// Set current blockchain height
    pub fn set_current_height(&self, height: u64) {
        let mut h = self.current_height.write().unwrap();
        *h = height;
    }
    
    /// Get current blockchain height
    pub fn current_height(&self) -> u64 {
        let h = self.current_height.read().unwrap();
        *h
    }
    
    /// Set target blockchain height
    pub fn set_target_height(&self, height: u64) {
        let mut h = self.target_height.write().unwrap();
        *h = height;
    }
    
    /// Get target blockchain height
    pub fn target_height(&self) -> u64 {
        let h = self.target_height.read().unwrap();
        *h
    }
    
    /// Update peer height
    pub fn update_peer_height(&self, peer_id: PeerId, height: u64) {
        let mut peers = self.best_peers.write().unwrap();
        peers.insert(peer_id, (height, Instant::now()));
    }
    
    /// Get the best peer for syncing
    pub fn best_peer(&self) -> Option<PeerId> {
        let peers = self.best_peers.read().unwrap();
        
        peers.iter()
            .max_by_key(|(_, (height, _))| height)
            .map(|(peer_id, _)| *peer_id)
    }
    
    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &PeerId) {
        let mut peers = self.best_peers.write().unwrap();
        peers.remove(peer_id);
    }
    
    /// Get the next request ID
    fn next_request_id(&self) -> u64 {
        let mut id = self.next_request_id.lock().unwrap();
        let result = *id;
        *id += 1;
        result
    }
    
    /// Start syncing
    pub fn start_sync(&self) -> SyncResult<()> {
        let mut state = self.state.write().unwrap();
        
        // If we're already syncing, do nothing
        if !matches!(*state, SyncState::Idle) {
            return Ok(());
        }
        
        // Get current and target heights
        let current_height = self.current_height();
        let target_height = self.target_height();
        
        // If we're already caught up, do nothing
        if current_height >= target_height {
            return Ok(());
        }
        
        // Choose sync strategy based on config and heights
        let config = self.config();
        let strategy = self.choose_strategy(current_height, target_height, config.strategy);
        
        // Initialize sync state based on strategy
        *state = match strategy {
            SyncStrategy::Tip => SyncState::Tip(TipSyncState {
                block_requests: HashMap::new(),
                processed_blocks: HashSet::new(),
            }),
            
            SyncStrategy::HeadersFirst => SyncState::HeadersFirst(HeaderSyncState {
                headers: Vec::new(),
                header_requests: HashMap::new(),
                block_requests: HashMap::new(),
                processed_blocks: HashSet::new(),
            }),
            
            SyncStrategy::Parallel => SyncState::Parallel(ParallelSyncState {
                block_requests: HashMap::new(),
                processed_blocks: HashSet::new(),
            }),
            
            SyncStrategy::Warp => SyncState::Warp(WarpSyncState {
                last_warp_point: current_height,
                next_warp_target: Self::calculate_next_warp_target(current_height, target_height, config.warp_sync_interval),
                is_warp_syncing: false,
                warp_request_id: None,
            }),
        };
        
        Ok(())
    }
    
    /// Stop syncing
    pub fn stop_sync(&self) {
        let mut state = self.state.write().unwrap();
        *state = SyncState::Idle;
    }
    
    /// Check if we're currently syncing
    pub fn is_syncing(&self) -> bool {
        let state = self.state.read().unwrap();
        !matches!(*state, SyncState::Idle)
    }
    
    /// Choose the appropriate sync strategy
    fn choose_strategy(&self, current_height: u64, target_height: u64, preferred: SyncStrategy) -> SyncStrategy {
        // Gap between current and target height
        let gap = target_height.saturating_sub(current_height);
        
        match preferred {
            // Warp sync is always respected if specified
            SyncStrategy::Warp => {
                // Only use warp sync for large gaps
                if gap > 1000 {
                    SyncStrategy::Warp
                } else {
                    SyncStrategy::HeadersFirst
                }
            },
            
            // Tip sync is only used for small gaps
            SyncStrategy::Tip => {
                if gap < 100 {
                    SyncStrategy::Tip
                } else {
                    SyncStrategy::HeadersFirst
                }
            },
            
            // Headers-first is the default strategy
            SyncStrategy::HeadersFirst => SyncStrategy::HeadersFirst,
            
            // Parallel is used for medium gaps with good bandwidth
            SyncStrategy::Parallel => {
                if gap < 10000 {
                    SyncStrategy::Parallel
                } else {
                    SyncStrategy::HeadersFirst
                }
            },
        }
    }
    
    /// Calculate next warp sync target
    fn calculate_next_warp_target(current: u64, target: u64, interval: u64) -> u64 {
        let next = ((current / interval) + 1) * interval;
        next.min(target)
    }
    
    /// Process a chain info response
    pub fn process_chain_info(&self, peer_id: PeerId, response: ChainInfoResponse) {
        // Update peer height
        self.update_peer_height(peer_id, response.height);
        
        // Update target height if higher
        let target = self.target_height();
        if response.height > target {
            self.set_target_height(response.height);
        }
    }
    
    /// Process a block response
    pub fn process_block_response(&self, peer_id: PeerId, response: BlockResponse) -> SyncResult<Vec<Block>> {
        let mut state = self.state.write().unwrap();
        
        match &mut *state {
            SyncState::Idle => {
                // Not syncing, ignore response
                Ok(Vec::new())
            },
            
            SyncState::Tip(sync_state) => {
                // Remove the matching request
                for (req_id, (_, _, _)) in sync_state.block_requests.iter() {
                    if *req_id == response.request_id {
                        sync_state.block_requests.remove(req_id);
                        break;
                    }
                }
                
                // Mark blocks as processed
                for block in &response.blocks {
                    sync_state.processed_blocks.insert(block.header.hash());
                }
                
                Ok(response.blocks)
            },
            
            SyncState::HeadersFirst(sync_state) => {
                // Check if this is a header or block response
                if let Some((hashes, _)) = sync_state.block_requests.remove(&response.request_id) {
                    // Block response
                    // Mark blocks as processed
                    for block in &response.blocks {
                        sync_state.processed_blocks.insert(block.header.hash());
                    }
                    
                    Ok(response.blocks)
                } else {
                    // Could be a header response packed as BlockResponse
                    // Store headers for later body retrieval
                    for block in &response.blocks {
                        if !sync_state.processed_blocks.contains(&block.header.hash()) {
                            sync_state.headers.push(block.header.clone());
                        }
                    }
                    
                    Ok(response.blocks)
                }
            },
            
            SyncState::Parallel(sync_state) => {
                // Remove the matching request
                for (req_id, (_, _, _)) in sync_state.block_requests.iter() {
                    if *req_id == response.request_id {
                        sync_state.block_requests.remove(req_id);
                        break;
                    }
                }
                
                // Mark blocks as processed
                for block in &response.blocks {
                    sync_state.processed_blocks.insert(block.header.hash());
                }
                
                Ok(response.blocks)
            },
            
            SyncState::Warp(sync_state) => {
                // Check if this is a warp response
                if let Some(req_id) = sync_state.warp_request_id {
                    if req_id == response.request_id {
                        sync_state.warp_request_id = None;
                        sync_state.is_warp_syncing = false;
                        
                        // Update last warp point
                        if let Some(last_block) = response.blocks.last() {
                            sync_state.last_warp_point = last_block.header.height;
                            
                            // Calculate next warp target
                            let config = self.config();
                            let target = self.target_height();
                            sync_state.next_warp_target = Self::calculate_next_warp_target(
                                sync_state.last_warp_point,
                                target,
                                config.warp_sync_interval,
                            );
                        }
                        
                        Ok(response.blocks)
                    } else {
                        // Not our warp response
                        Ok(Vec::new())
                    }
                } else {
                    // Regular block response during warp sync
                    Ok(response.blocks)
                }
            },
        }
    }
    
    /// Get the next block request
    pub fn next_block_request(&self) -> SyncResult<Option<(PeerId, BlockRequest)>> {
        // Find best peer
        let best_peer = match self.best_peer() {
            Some(peer) => peer,
            None => return Ok(None), // No peers available
        };
        
        let mut state = self.state.write().unwrap();
        let config = self.config();
        
        match &mut *state {
            SyncState::Idle => {
                // Not syncing
                Ok(None)
            },
            
            SyncState::Tip(sync_state) => {
                let current_height = self.current_height();
                let target_height = self.target_height();
                
                if current_height >= target_height {
                    // Already caught up
                    *state = SyncState::Idle;
                    return Ok(None);
                }
                
                // Calculate request range
                let start_height = current_height + 1;
                let end_height = (start_height + config.max_blocks_per_request - 1).min(target_height);
                
                // Create request
                let request_id = self.next_request_id();
                let request = BlockRequest {
                    request_id,
                    start_height,
                    end_height,
                    include_bodies: true,
                };
                
                // Store request
                sync_state.block_requests.insert(request_id, (start_height, end_height, Instant::now()));
                
                Ok(Some((best_peer, request)))
            },
            
            SyncState::HeadersFirst(sync_state) => {
                let current_height = self.current_height();
                let target_height = self.target_height();
                
                if current_height >= target_height {
                    // Already caught up
                    *state = SyncState::Idle;
                    return Ok(None);
                }
                
                // Check if we have pending headers to download
                if sync_state.header_requests.values().len() < config.max_concurrent_requests as usize &&
                   sync_state.headers.len() < 1000 {
                    // Calculate request range for headers
                    let mut start_height = current_height + 1;
                    
                    // Skip heights we already requested
                    for (_, (s, e, _)) in &sync_state.header_requests {
                        if *s <= start_height && start_height <= *e {
                            start_height = *e + 1;
                        }
                    }
                    
                    if start_height <= target_height {
                        let end_height = (start_height + config.max_headers_per_request - 1).min(target_height);
                        
                        // Create headers request
                        let request_id = self.next_request_id();
                        let request = BlockRequest {
                            request_id,
                            start_height,
                            end_height,
                            include_bodies: false,
                        };
                        
                        // Store request
                        sync_state.header_requests.insert(request_id, (start_height, end_height, Instant::now()));
                        
                        return Ok(Some((best_peer, request)));
                    }
                }
                
                // Check if we have headers without bodies to download
                if sync_state.block_requests.values().len() < config.max_concurrent_requests as usize &&
                   !sync_state.headers.is_empty() {
                    // Take up to max_blocks_per_request headers
                    let count = config.max_blocks_per_request.min(sync_state.headers.len() as u32);
                    let headers: Vec<_> = sync_state.headers.drain(0..count as usize).collect();
                    
                    // Extract hashes
                    let hashes: Vec<_> = headers.iter().map(|h| h.hash()).collect();
                    
                    // Create block bodies request
                    let request_id = self.next_request_id();
                    let request = BlockRequest {
                        request_id,
                        start_height: 0, // Not used for hash-based requests
                        end_height: 0,   // Not used for hash-based requests
                        include_bodies: true,
                        block_hashes: Some(hashes.clone()),
                    };
                    
                    // Store request
                    sync_state.block_requests.insert(request_id, (hashes, Instant::now()));
                    
                    return Ok(Some((best_peer, request)));
                }
                
                // No request needed right now
                Ok(None)
            },
            
            SyncState::Parallel(sync_state) => {
                let current_height = self.current_height();
                let target_height = self.target_height();
                
                if current_height >= target_height {
                    // Already caught up
                    *state = SyncState::Idle;
                    return Ok(None);
                }
                
                // Check if we can make more requests
                if sync_state.block_requests.values().len() < config.max_concurrent_requests as usize {
                    // Calculate request range
                    let mut start_height = current_height + 1;
                    
                    // Skip heights we already requested
                    for (_, (s, e, _)) in &sync_state.block_requests {
                        if *s <= start_height && start_height <= *e {
                            start_height = *e + 1;
                        }
                    }
                    
                    if start_height <= target_height {
                        let end_height = (start_height + config.max_blocks_per_request - 1).min(target_height);
                        
                        // Create block request
                        let request_id = self.next_request_id();
                        let request = BlockRequest {
                            request_id,
                            start_height,
                            end_height,
                            include_bodies: true,
                        };
                        
                        // Store request
                        sync_state.block_requests.insert(request_id, (start_height, end_height, Instant::now()));
                        
                        return Ok(Some((best_peer, request)));
                    }
                }
                
                // No request needed right now
                Ok(None)
            },
            
            SyncState::Warp(sync_state) => {
                let current_height = self.current_height();
                let target_height = self.target_height();
                
                if current_height >= target_height {
                    // Already caught up
                    *state = SyncState::Idle;
                    return Ok(None);
                }
                
                // Check if we're currently warp syncing
                if sync_state.is_warp_syncing {
                    // Already have a warp sync in progress
                    return Ok(None);
                }
                
                // Check if we should do a warp sync
                if current_height < sync_state.next_warp_target {
                    // Create warp sync request
                    let request_id = self.next_request_id();
                    let request = BlockRequest {
                        request_id,
                        start_height: sync_state.last_warp_point,
                        end_height: sync_state.next_warp_target,
                        include_bodies: true,
                        warp_sync: true,
                    };
                    
                    // Store request
                    sync_state.warp_request_id = Some(request_id);
                    sync_state.is_warp_syncing = true;
                    
                    return Ok(Some((best_peer, request)));
                }
                
                // Fall back to regular sync for small gaps
                let start_height = current_height + 1;
                let end_height = (start_height + config.max_blocks_per_request - 1).min(target_height);
                
                // Create block request
                let request_id = self.next_request_id();
                let request = BlockRequest {
                    request_id,
                    start_height,
                    end_height,
                    include_bodies: true,
                };
                
                Ok(Some((best_peer, request)))
            },
        }
    }
    
    /// Check for timed out requests
    pub fn check_timeouts(&self) -> Vec<(PeerId, u64)> {
        let mut result = Vec::new();
        let mut state = self.state.write().unwrap();
        let config = self.config();
        let timeout = Duration::from_secs(config.request_timeout_seconds);
        
        match &mut *state {
            SyncState::Idle => {
                // Not syncing
            },
            
            SyncState::Tip(sync_state) => {
                // Check block requests
                let timed_out: Vec<_> = sync_state.block_requests.iter()
                    .filter(|(_, (_, _, time))| time.elapsed() > timeout)
                    .map(|(id, _)| *id)
                    .collect();
                
                for id in timed_out {
                    sync_state.block_requests.remove(&id);
                    result.push((self.best_peer().unwrap_or(PeerId::random()), id));
                }
            },
            
            SyncState::HeadersFirst(sync_state) => {
                // Check header requests
                let timed_out_headers: Vec<_> = sync_state.header_requests.iter()
                    .filter(|(_, (_, _, time))| time.elapsed() > timeout)
                    .map(|(id, _)| *id)
                    .collect();
                
                for id in timed_out_headers {
                    sync_state.header_requests.remove(&id);
                    result.push((self.best_peer().unwrap_or(PeerId::random()), id));
                }
                
                // Check block requests
                let timed_out_blocks: Vec<_> = sync_state.block_requests.iter()
                    .filter(|(_, (_, time))| time.elapsed() > timeout)
                    .map(|(id, _)| *id)
                    .collect();
                
                for id in timed_out_blocks {
                    sync_state.block_requests.remove(&id);
                    result.push((self.best_peer().unwrap_or(PeerId::random()), id));
                }
            },
            
            SyncState::Parallel(sync_state) => {
                // Check block requests
                let timed_out: Vec<_> = sync_state.block_requests.iter()
                    .filter(|(_, (_, _, time))| time.elapsed() > timeout)
                    .map(|(id, _)| *id)
                    .collect();
                
                for id in timed_out {
                    sync_state.block_requests.remove(&id);
                    result.push((self.best_peer().unwrap_or(PeerId::random()), id));
                }
            },
            
            SyncState::Warp(sync_state) => {
                // Check warp request
                if let Some(id) = sync_state.warp_request_id {
                    if sync_state.is_warp_syncing && Instant::now().elapsed() > timeout {
                        sync_state.warp_request_id = None;
                        sync_state.is_warp_syncing = false;
                        result.push((self.best_peer().unwrap_or(PeerId::random()), id));
                    }
                }
            },
        }
        
        result
    }
    
    /// Get sync progress information
    pub fn sync_progress(&self) -> (u64, u64, SyncStrategy) {
        let current = self.current_height();
        let target = self.target_height();
        
        let state = self.state.read().unwrap();
        let strategy = match &*state {
            SyncState::Idle => SyncStrategy::Tip,
            SyncState::Tip(_) => SyncStrategy::Tip,
            SyncState::HeadersFirst(_) => SyncStrategy::HeadersFirst,
            SyncState::Parallel(_) => SyncStrategy::Parallel,
            SyncState::Warp(_) => SyncStrategy::Warp,
        };
        
        (current, target, strategy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_strategy_selection() {
        let config = SyncConfig::default();
        let sync = SyncManager::new(config, 0);
        
        // Test strategy selection for different gaps
        assert_eq!(sync.choose_strategy(0, 50, SyncStrategy::Tip), SyncStrategy::Tip);
        assert_eq!(sync.choose_strategy(0, 500, SyncStrategy::Tip), SyncStrategy::HeadersFirst);
        
        assert_eq!(sync.choose_strategy(0, 5000, SyncStrategy::Parallel), SyncStrategy::Parallel);
        assert_eq!(sync.choose_strategy(0, 50000, SyncStrategy::Parallel), SyncStrategy::HeadersFirst);
        
        assert_eq!(sync.choose_strategy(0, 500, SyncStrategy::Warp), SyncStrategy::HeadersFirst);
        assert_eq!(sync.choose_strategy(0, 5000, SyncStrategy::Warp), SyncStrategy::Warp);
        
        assert_eq!(sync.choose_strategy(0, 500000, SyncStrategy::HeadersFirst), SyncStrategy::HeadersFirst);
    }
    
    #[test]
    fn test_warp_sync_target_calculation() {
        assert_eq!(SyncManager::calculate_next_warp_target(0, 100000, 10000), 10000);
        assert_eq!(SyncManager::calculate_next_warp_target(5000, 100000, 10000), 10000);
        assert_eq!(SyncManager::calculate_next_warp_target(10000, 100000, 10000), 20000);
        assert_eq!(SyncManager::calculate_next_warp_target(95000, 100000, 10000), 100000);
        assert_eq!(SyncManager::calculate_next_warp_target(95000, 96000, 10000), 96000);
    }
} 