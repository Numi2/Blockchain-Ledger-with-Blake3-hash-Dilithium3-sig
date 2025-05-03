use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Reputation score thresholds
pub const REPUTATION_GOOD: i32 = 100;
pub const REPUTATION_NEUTRAL: i32 = 0;
pub const REPUTATION_BAD: i32 = -50;
pub const REPUTATION_BAN: i32 = -100;

/// Reputation score changes
pub const SCORE_SUCCESSFUL_INTERACTION: i32 = 1;
pub const SCORE_USEFUL_DATA: i32 = 5;
pub const SCORE_GOOD_BLOCK: i32 = 10;
pub const SCORE_INVALID_DATA: i32 = -10;
pub const SCORE_INVALID_BLOCK: i32 = -20;
pub const SCORE_TIMEOUT: i32 = -5;
pub const SCORE_DISCONNECT: i32 = -1;
pub const SCORE_SPAM: i32 = -15;
pub const SCORE_DOS_ATTEMPT: i32 = -50;

/// Reputation events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReputationEvent {
    /// Successful interaction (ping, etc.)
    SuccessfulInteraction,
    
    /// Provided useful data
    UsefulData,
    
    /// Provided a good block
    GoodBlock,
    
    /// Provided invalid data
    InvalidData,
    
    /// Provided an invalid block
    InvalidBlock,
    
    /// Timed out on request
    Timeout,
    
    /// Disconnected unexpectedly
    Disconnect,
    
    /// Sent spam
    Spam,
    
    /// DoS attempt
    DosAttempt,
    
    /// Custom score adjustment
    Custom(i32),
}

/// Peer reputation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReputation {
    /// Peer ID
    pub peer_id: String,
    
    /// Current reputation score
    pub score: i32,
    
    /// Last updated time (seconds since epoch)
    pub last_updated: u64,
    
    /// Reputation events and their timestamps
    pub events: Vec<(String, u64)>,
    
    /// Is the peer banned
    pub banned: bool,
    
    /// Ban expiration time (seconds since epoch)
    pub ban_expires: Option<u64>,
}

impl PeerReputation {
    /// Create a new peer reputation record
    pub fn new(peer_id: &str) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            score: REPUTATION_NEUTRAL,
            last_updated: current_time_secs(),
            events: Vec::new(),
            banned: false,
            ban_expires: None,
        }
    }
    
    /// Add an event to the reputation record
    pub fn add_event(&mut self, event: ReputationEvent) {
        let now = current_time_secs();
        self.last_updated = now;
        
        // Get score change for this event
        let score_change = event_to_score(event);
        
        // Update score
        self.score += score_change;
        
        // Add event to history (limited to last 100 events)
        let event_name = format!("{:?}", event);
        self.events.push((event_name, now));
        if self.events.len() > 100 {
            self.events.remove(0);
        }
        
        // Check for ban
        if self.score <= REPUTATION_BAN && !self.banned {
            self.banned = true;
            
            // Ban for a day by default
            self.ban_expires = Some(now + 86400);
        }
    }
    
    /// Check if ban has expired
    pub fn check_ban_expired(&mut self) -> bool {
        if !self.banned {
            return false;
        }
        
        if let Some(expiry) = self.ban_expires {
            let now = current_time_secs();
            if now >= expiry {
                self.banned = false;
                self.ban_expires = None;
                
                // Reset score to neutral
                self.score = REPUTATION_NEUTRAL;
                
                return true;
            }
        }
        
        false
    }
}

/// Reputation manager for tracking peer reputation
pub struct ReputationManager {
    /// Peer reputations
    reputations: RwLock<HashMap<String, PeerReputation>>,
    
    /// Banned peers set (for quick lookup)
    banned_peers: RwLock<HashSet<String>>,
    
    /// Last cleanup time
    last_cleanup: Mutex<Instant>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub fn new() -> Self {
        Self {
            reputations: RwLock::new(HashMap::new()),
            banned_peers: RwLock::new(HashSet::new()),
            last_cleanup: Mutex::new(Instant::now()),
        }
    }
    
    /// Record a reputation event for a peer
    pub fn record_event(&self, peer_id: &str, event: ReputationEvent) {
        let mut reputations = self.reputations.write().unwrap();
        
        // Get or create reputation record
        let rep = reputations
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerReputation::new(peer_id));
            
        // Add event
        rep.add_event(event);
        
        // Update banned peers set
        if rep.banned {
            let mut banned = self.banned_peers.write().unwrap();
            banned.insert(peer_id.to_string());
        }
    }
    
    /// Get reputation score for a peer
    pub fn get_score(&self, peer_id: &str) -> i32 {
        let reputations = self.reputations.read().unwrap();
        
        match reputations.get(peer_id) {
            Some(rep) => rep.score,
            None => REPUTATION_NEUTRAL,
        }
    }
    
    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &str) -> bool {
        // Quick check in banned set
        let banned = self.banned_peers.read().unwrap();
        if !banned.contains(peer_id) {
            return false;
        }
        
        // Double-check the actual ban status
        let mut reputations = self.reputations.write().unwrap();
        
        if let Some(rep) = reputations.get_mut(peer_id) {
            // Check if ban has expired
            if rep.check_ban_expired() {
                // Ban expired, update banned set
                let mut banned = self.banned_peers.write().unwrap();
                banned.remove(peer_id);
                return false;
            }
            
            rep.banned
        } else {
            false
        }
    }
    
    /// Ban a peer
    pub fn ban_peer(&self, peer_id: &str, duration_secs: Option<u64>) {
        let mut reputations = self.reputations.write().unwrap();
        
        // Get or create reputation record
        let rep = reputations
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerReputation::new(peer_id));
            
        // Ban the peer
        rep.banned = true;
        
        // Set ban expiration
        let now = current_time_secs();
        rep.ban_expires = Some(now + duration_secs.unwrap_or(86400)); // Default: 1 day
        
        // Set score to ban threshold
        rep.score = REPUTATION_BAN;
        
        // Update banned peers set
        let mut banned = self.banned_peers.write().unwrap();
        banned.insert(peer_id.to_string());
    }
    
    /// Unban a peer
    pub fn unban_peer(&self, peer_id: &str) {
        let mut reputations = self.reputations.write().unwrap();
        
        if let Some(rep) = reputations.get_mut(peer_id) {
            rep.banned = false;
            rep.ban_expires = None;
            rep.score = REPUTATION_NEUTRAL;
            
            // Update banned peers set
            let mut banned = self.banned_peers.write().unwrap();
            banned.remove(peer_id);
        }
    }
    
    /// Get reputation for a peer
    pub fn get_reputation(&self, peer_id: &str) -> Option<PeerReputation> {
        let reputations = self.reputations.read().unwrap();
        reputations.get(peer_id).cloned()
    }
    
    /// Get all peer reputations
    pub fn get_all_reputations(&self) -> Vec<PeerReputation> {
        let reputations = self.reputations.read().unwrap();
        reputations.values().cloned().collect()
    }
    
    /// Get all banned peers
    pub fn get_banned_peers(&self) -> Vec<String> {
        let banned = self.banned_peers.read().unwrap();
        banned.iter().cloned().collect()
    }
    
    /// Run cleanup (removes old records, checks expired bans)
    pub fn cleanup(&self) {
        // Only run cleanup once per hour
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        if last_cleanup.elapsed() < Duration::from_secs(3600) {
            return;
        }
        *last_cleanup = Instant::now();
        
        let mut reputations = self.reputations.write().unwrap();
        let mut banned = self.banned_peers.write().unwrap();
        
        // Current time
        let now = current_time_secs();
        
        // Peers to remove (inactive for over 30 days)
        let inactive_threshold = now - 30 * 86400;
        let mut to_remove = Vec::new();
        
        // Check each peer
        for (peer_id, rep) in reputations.iter_mut() {
            // Check if ban has expired
            if rep.banned && rep.check_ban_expired() {
                banned.remove(peer_id);
            }
            
            // Check if inactive
            if rep.last_updated < inactive_threshold {
                to_remove.push(peer_id.clone());
            }
        }
        
        // Remove inactive peers
        for peer_id in to_remove {
            reputations.remove(&peer_id);
            banned.remove(&peer_id);
        }
    }
}

/// Convert a reputation event to a score change
fn event_to_score(event: ReputationEvent) -> i32 {
    match event {
        ReputationEvent::SuccessfulInteraction => SCORE_SUCCESSFUL_INTERACTION,
        ReputationEvent::UsefulData => SCORE_USEFUL_DATA,
        ReputationEvent::GoodBlock => SCORE_GOOD_BLOCK,
        ReputationEvent::InvalidData => SCORE_INVALID_DATA,
        ReputationEvent::InvalidBlock => SCORE_INVALID_BLOCK,
        ReputationEvent::Timeout => SCORE_TIMEOUT,
        ReputationEvent::Disconnect => SCORE_DISCONNECT,
        ReputationEvent::Spam => SCORE_SPAM,
        ReputationEvent::DosAttempt => SCORE_DOS_ATTEMPT,
        ReputationEvent::Custom(score) => score,
    }
}

/// Get current time in seconds since epoch
fn current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reputation_scoring() {
        let manager = ReputationManager::new();
        let peer_id = "test_peer";
        
        // Initially neutral
        assert_eq!(manager.get_score(peer_id), REPUTATION_NEUTRAL);
        
        // Record some positive events
        manager.record_event(peer_id, ReputationEvent::SuccessfulInteraction);
        manager.record_event(peer_id, ReputationEvent::UsefulData);
        
        // Score should increase
        assert!(manager.get_score(peer_id) > REPUTATION_NEUTRAL);
        
        // Record a negative event
        manager.record_event(peer_id, ReputationEvent::InvalidData);
        
        // Check reputation
        let rep = manager.get_reputation(peer_id).unwrap();
        assert_eq!(rep.events.len(), 3);
        
        // Peer shouldn't be banned yet
        assert!(!manager.is_banned(peer_id));
        
        // Ban the peer
        manager.ban_peer(peer_id, Some(1)); // Ban for 1 second
        
        // Should be banned now
        assert!(manager.is_banned(peer_id));
        
        // Wait for ban to expire
        std::thread::sleep(Duration::from_secs(2));
        
        // Check if ban expired
        assert!(!manager.is_banned(peer_id));
        
        // Score should be reset to neutral
        assert_eq!(manager.get_score(peer_id), REPUTATION_NEUTRAL);
    }
} 