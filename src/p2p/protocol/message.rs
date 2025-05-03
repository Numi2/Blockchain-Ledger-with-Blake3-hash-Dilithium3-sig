use crate::core::block::{Block, BlockHeader, Transaction};
use crate::core::consensus::{Vote, VoteType};
use crate::types::{Hash, Signature, Slot, ValidatorIndex};
use serde::{Deserialize, Serialize};

/// Different types of P2P messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Block announcement message
    Block,
    
    /// Transaction announcement message
    Transaction,
    
    /// Vote message
    Vote,
    
    /// Status message for bootstrapping
    Status,
    
    /// Block request message
    BlockRequest,
    
    /// Block response message
    BlockResponse,
}

/// Basic P2P network message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message type
    pub msg_type: MessageType,
    
    /// Message payload
    pub payload: Vec<u8>,
}

/// Block announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMessage {
    /// The block being announced
    pub block: Block,
}

/// Transaction announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMessage {
    /// The transaction being announced
    pub transaction: Transaction,
}

/// Vote message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteMessage {
    /// The vote being announced
    pub vote: Vote,
}

/// Status message for peer handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    /// Current protocol version
    pub protocol_version: u32,
    
    /// Chain ID
    pub chain_id: u32,
    
    /// Latest block header
    pub latest_header: BlockHeader,
    
    /// Finalized block hash
    pub finalized_hash: Hash,
    
    /// Finalized block slot
    pub finalized_slot: Slot,
}

/// Block request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRequestMessage {
    /// Block hashes to request
    pub block_hashes: Vec<Hash>,
}

/// Block response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponseMessage {
    /// Requested blocks
    pub blocks: Vec<Block>,
} 