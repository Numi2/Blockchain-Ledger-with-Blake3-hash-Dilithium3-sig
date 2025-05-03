use crate::core::block::{Block, Transaction};
use crate::core::consensus::Vote;
use crate::p2p::protocol::{BlockMessage, Message, MessageType, TransactionMessage, VoteMessage};
use crate::ssz::serialize;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Topic identifiers for different gossip channels
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GossipTopic {
    /// Blocks being propagated
    Blocks,
    
    /// Transactions being propagated
    Transactions,
    
    /// Consensus votes being propagated
    Votes,
}

impl GossipTopic {
    /// Convert to string representation for libp2p
    pub fn as_str(&self) -> &'static str {
        match self {
            GossipTopic::Blocks => "/worldledger/blocks/1.0.0",
            GossipTopic::Transactions => "/worldledger/transactions/1.0.0",
            GossipTopic::Votes => "/worldledger/votes/1.0.0",
        }
    }

    /// Get all gossip topics
    pub fn all() -> Vec<GossipTopic> {
        vec![
            GossipTopic::Blocks,
            GossipTopic::Transactions,
            GossipTopic::Votes,
        ]
    }
}

/// Tracker for seen gossip messages to prevent duplicates
pub struct MessageCache {
    /// Set of seen message IDs
    seen_messages: HashSet<[u8; 32]>,
    
    /// When this cache was last pruned
    last_pruned: Instant,
    
    /// How frequently to prune old entries
    prune_interval: Duration,
    
    /// Maximum number of entries before pruning
    max_entries: usize,
}

impl MessageCache {
    /// Create a new message cache
    pub fn new() -> Self {
        Self {
            seen_messages: HashSet::new(),
            last_pruned: Instant::now(),
            prune_interval: Duration::from_secs(300), // 5 minutes
            max_entries: 10000,
        }
    }

    /// Check if a message has been seen before
    pub fn has_seen(&mut self, msg_id: &[u8; 32]) -> bool {
        // Prune if needed
        self.maybe_prune();
        
        self.seen_messages.contains(msg_id)
    }

    /// Mark a message as seen
    pub fn mark_seen(&mut self, msg_id: [u8; 32]) {
        self.seen_messages.insert(msg_id);
    }

    /// Prune old entries if necessary
    fn maybe_prune(&mut self) {
        let now = Instant::now();
        
        // Check if we need to prune based on time or size
        if now.duration_since(self.last_pruned) >= self.prune_interval
            || self.seen_messages.len() >= self.max_entries
        {
            // In a real implementation, we'd keep track of timestamps and prune oldest
            // For now, just clear half the cache randomly
            if self.seen_messages.len() > 1000 {
                let to_keep: Vec<_> = self.seen_messages.iter().take(self.seen_messages.len() / 2).cloned().collect();
                self.seen_messages.clear();
                for msg in to_keep {
                    self.seen_messages.insert(msg);
                }
            }
            
            self.last_pruned = now;
        }
    }
}

/// GossipHandler processes incoming and outgoing gossip messages
pub struct GossipHandler {
    /// Cache of seen messages
    message_cache: MessageCache,
}

impl GossipHandler {
    /// Create a new gossip handler
    pub fn new() -> Self {
        Self {
            message_cache: MessageCache::new(),
        }
    }

    /// Create a block gossip message
    pub fn create_block_message(&mut self, block: Block) -> (GossipTopic, Vec<u8>) {
        let block_message = BlockMessage { block };
        let payload = serialize(&block_message);
        let message = Message {
            msg_type: MessageType::Block,
            payload,
        };
        
        // In a real implementation, we'd compute a proper message ID
        let msg_id = [0u8; 32]; // Placeholder
        self.message_cache.mark_seen(msg_id);
        
        (GossipTopic::Blocks, serialize(&message))
    }

    /// Create a transaction gossip message
    pub fn create_transaction_message(&mut self, transaction: Transaction) -> (GossipTopic, Vec<u8>) {
        let tx_message = TransactionMessage { transaction };
        let payload = serialize(&tx_message);
        let message = Message {
            msg_type: MessageType::Transaction,
            payload,
        };
        
        // In a real implementation, we'd compute a proper message ID
        let msg_id = [0u8; 32]; // Placeholder
        self.message_cache.mark_seen(msg_id);
        
        (GossipTopic::Transactions, serialize(&message))
    }

    /// Create a vote gossip message
    pub fn create_vote_message(&mut self, vote: Vote) -> (GossipTopic, Vec<u8>) {
        let vote_message = VoteMessage { vote };
        let payload = serialize(&vote_message);
        let message = Message {
            msg_type: MessageType::Vote,
            payload,
        };
        
        // In a real implementation, we'd compute a proper message ID
        let msg_id = [0u8; 32]; // Placeholder
        self.message_cache.mark_seen(msg_id);
        
        (GossipTopic::Votes, serialize(&message))
    }

    /// Process an incoming gossip message
    pub fn process_message(&mut self, topic: &GossipTopic, data: &[u8]) -> Option<Message> {
        // In a real implementation, we'd compute the message ID from the data
        let msg_id = [0u8; 32]; // Placeholder
        
        // Skip if we've seen this message before
        if self.message_cache.has_seen(&msg_id) {
            return None;
        }
        
        // Mark as seen
        self.message_cache.mark_seen(msg_id);
        
        // Deserialize the message
        // In a real implementation, we'd use proper SSZ deserialization
        // For now, just return None as a placeholder
        None
    }
} 