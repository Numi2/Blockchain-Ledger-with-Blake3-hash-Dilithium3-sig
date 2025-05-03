use serde::{Serialize, Deserialize};

/// Topic used for gossiping blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GossipTopic {
    /// Gossip new blocks
    Blocks,
    
    /// Gossip transaction announcements
    Transactions,
    
    /// Gossip validator messages (for PoS)
    ValidatorMessages,
    
    /// Gossip network status
    Status,
}

impl GossipTopic {
    /// Convert topic to string
    pub fn as_str(&self) -> &'static str {
        match self {
            GossipTopic::Blocks => "/worldledger/blocks/1.0.0",
            GossipTopic::Transactions => "/worldledger/txs/1.0.0",
            GossipTopic::ValidatorMessages => "/worldledger/validator/1.0.0",
            GossipTopic::Status => "/worldledger/status/1.0.0",
        }
    }
    
    /// Get all topics
    pub fn all() -> Vec<GossipTopic> {
        vec![
            GossipTopic::Blocks,
            GossipTopic::Transactions,
            GossipTopic::ValidatorMessages,
            GossipTopic::Status,
        ]
    }
}

/// Detailed sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Current sync state
    pub state: String,
    
    /// Target block height
    pub target_height: u64,
    
    /// Current block height
    pub current_height: u64,
    
    /// Number of headers downloaded
    pub headers_downloaded: u64,
    
    /// Total number of headers to download
    pub headers_total: u64,
    
    /// Number of blocks downloaded
    pub blocks_downloaded: u64,
    
    /// Total number of blocks to download
    pub blocks_total: u64,
    
    /// Number of state objects downloaded
    pub state_objects_downloaded: u64,
    
    /// Total number of state objects to download
    pub state_objects_total: u64,
    
    /// Elapsed time in seconds
    pub elapsed_seconds: u64,
    
    /// Estimated remaining time in seconds
    pub estimated_remaining_seconds: u64,
}

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    /// Protocol version
    pub protocol_version: String,
    
    /// Client version (e.g., "world-ledger/0.1.0")
    pub client_version: String,
    
    /// Current block height
    pub current_height: u64,
    
    /// Current block hash
    pub current_hash: String,
    
    /// Genesis block hash
    pub genesis_hash: String,
    
    /// Network ID
    pub network_id: String,
    
    /// Sync status (if syncing)
    pub sync_status: Option<SyncStatus>,
    
    /// Number of peers
    pub peer_count: u32,
    
    /// Number of pending transactions
    pub pending_transactions: u32,
}

/// Block request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRequest {
    /// Block hash
    pub hash: Option<String>,
    
    /// Block number
    pub number: Option<u64>,
    
    /// Request ID
    pub request_id: u64,
}

/// Block response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponse {
    /// Block data (serialized Block)
    pub block: Vec<u8>,
    
    /// Request ID (matching the request)
    pub request_id: u64,
}

/// Transaction broadcast message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBroadcast {
    /// Transaction data (serialized Transaction)
    pub transaction: Vec<u8>,
    
    /// Transaction hash
    pub hash: String,
}

/// Message types that can be sent over the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Status update
    Status(NodeStatus),
    
    /// Block request
    BlockRequest(BlockRequest),
    
    /// Block response
    BlockResponse(BlockResponse),
    
    /// Transaction broadcast
    Transaction(TransactionBroadcast),
    
    /// Peer discovery request
    DiscoveryRequest,
    
    /// Peer discovery response
    DiscoveryResponse(Vec<String>), // List of peer multiaddrs
}

/// Decode a message from bytes
pub fn decode_message(data: &[u8]) -> Result<Message, String> {
    bincode::deserialize(data)
        .map_err(|e| format!("Failed to deserialize message: {}", e))
}

/// Encode a message to bytes
pub fn encode_message(message: &Message) -> Result<Vec<u8>, String> {
    bincode::serialize(message)
        .map_err(|e| format!("Failed to serialize message: {}", e))
} 