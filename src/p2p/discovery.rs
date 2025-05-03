use libp2p::{
    core::ConnectedPoint,
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    kad::{
        record::store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent, QueryId, QueryResult,
    },
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    ping::{Ping, PingConfig, PingEvent},
    swarm::{NetworkBehaviour, NetworkBehaviourEventProcess},
    PeerId,
};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::p2p::reputation::ReputationManager;

/// Protocol name for the blockchain
pub const PROTOCOL_ID: &str = "/world-ledger/1.0.0";

/// Peer discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable Kademlia DHT
    pub enable_kademlia: bool,
    
    /// Enable mDNS discovery for local networks
    pub enable_mdns: bool,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    
    /// Peer discovery interval in seconds
    pub discovery_interval: u64,
    
    /// Max peers to discover
    pub max_peers: usize,
    
    /// Maximum number of outbound connections to maintain
    pub max_outbound: usize,
    
    /// Maximum number of inbound connections to accept
    pub max_inbound: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_kademlia: true,
            enable_mdns: true,
            bootstrap_nodes: vec![
                "/dns4/bootstrap-1.world-ledger.com/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp",
                "/dns4/bootstrap-2.world-ledger.com/tcp/30333/p2p/12D3KooWP2onT3zT3YGjkg6eTmqfBuX19JCvzRLZ5e9vTBYMqyJ3",
            ].into_iter().map(String::from).collect(),
            discovery_interval: 30,
            max_peers: 25,
            max_outbound: 8,
            max_inbound: 32,
        }
    }
}

/// Peer info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: String,
    
    /// Multi-addresses
    pub addresses: Vec<String>,
    
    /// Last seen time
    pub last_seen: u64,
    
    /// Client version
    pub client_version: Option<String>,
    
    /// Protocol version
    pub protocol_version: Option<String>,
    
    /// Connected state
    pub connected: bool,
    
    /// Inbound or outbound
    pub direction: Option<String>,
    
    /// Current reputation score
    pub reputation: i32,
}

/// Discovery events
#[derive(Debug)]
pub enum DiscoveryEvent {
    /// New peer discovered
    PeerDiscovered(PeerId, Vec<String>), // PeerID and multiaddresses
    
    /// Peer connected
    PeerConnected(PeerId, bool), // PeerID and is_outbound
    
    /// Peer disconnected
    PeerDisconnected(PeerId),
    
    /// Peer identified (client info)
    PeerIdentified(PeerId, String, String), // PeerID, client version, protocol version
}

/// Network behavior for peer discovery
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct DiscoveryBehaviour {
    /// Kademlia DHT
    kad: Kademlia<MemoryStore>,
    
    /// Ping protocol
    ping: Ping,
    
    /// mDNS discovery
    #[behaviour(ignore)]
    mdns: Option<Mdns>,
    
    /// Identify protocol
    identify: Identify,
    
    /// Active Kademlia queries
    #[behaviour(ignore)]
    active_queries: HashMap<QueryId, Instant>,
    
    /// Known peers
    #[behaviour(ignore)]
    known_peers: HashMap<PeerId, PeerInfo>,
    
    /// Reputation manager
    #[behaviour(ignore)]
    reputation: Arc<RwLock<ReputationManager>>,
    
    /// Discovered peers waiting for connection
    #[behaviour(ignore)]
    discovered_peers: HashSet<PeerId>,
    
    /// Configuration
    #[behaviour(ignore)]
    config: DiscoveryConfig,
    
    /// Last discovery time
    #[behaviour(ignore)]
    last_discovery: Instant,
}

impl DiscoveryBehaviour {
    /// Create a new discovery behavior
    pub async fn new(
        local_peer_id: PeerId,
        config: DiscoveryConfig,
        reputation: Arc<RwLock<ReputationManager>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Set up Kademlia
        let store = MemoryStore::new(local_peer_id);
        let mut kad_config = KademliaConfig::default();
        kad_config.set_protocol_name(PROTOCOL_ID.as_bytes());
        let mut kad = Kademlia::with_config(local_peer_id, store, kad_config);
        
        // Set up bootstrap nodes
        for node in &config.bootstrap_nodes {
            if let Ok((peer_id, addr)) = parse_peer_addr(node) {
                kad.add_address(&peer_id, addr);
            }
        }
        
        // Set up Ping
        let ping_config = PingConfig::new().with_interval(Duration::from_secs(30));
        let ping = Ping::new(ping_config);
        
        // Set up mDNS if enabled
        let mdns = if config.enable_mdns {
            Some(Mdns::new(MdnsConfig::default()).await?)
        } else {
            None
        };
        
        // Set up Identify
        let identify_config = IdentifyConfig::new("/world-ledger/1.0.0".to_string(), local_peer_id);
        let identify = Identify::new(identify_config);
        
        Ok(Self {
            kad,
            ping,
            mdns,
            identify,
            active_queries: HashMap::new(),
            known_peers: HashMap::new(),
            reputation,
            discovered_peers: HashSet::new(),
            config,
            last_discovery: Instant::now(),
        })
    }
    
    /// Start peer discovery
    pub fn start_discovery(&mut self) {
        if self.config.enable_kademlia {
            // Start a random Kademlia query to discover peers
            let query_id = self.kad.get_closest_peers(PeerId::random());
            self.active_queries.insert(query_id, Instant::now());
        }
        
        self.last_discovery = Instant::now();
    }
    
    /// Check if we should run discovery again
    pub fn should_discover(&self) -> bool {
        // Run discovery if enough time has passed
        self.last_discovery.elapsed() > Duration::from_secs(self.config.discovery_interval) &&
        // And we have few enough connected peers
        self.known_peers.values().filter(|p| p.connected).count() < self.config.max_peers
    }
    
    /// Get all known peers
    pub fn known_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.values().cloned().collect()
    }
    
    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.values()
            .filter(|p| p.connected)
            .cloned()
            .collect()
    }
    
    /// Check if a peer is connected
    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.known_peers.get(peer_id)
            .map(|p| p.connected)
            .unwrap_or(false)
    }
    
    /// Get peer info
    pub fn get_peer_info(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.known_peers.get(peer_id).cloned()
    }
    
    /// Add or update peer info
    fn update_peer_info(&mut self, peer_id: PeerId, update: impl FnOnce(&mut PeerInfo)) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let peer_info = self.known_peers
            .entry(peer_id)
            .or_insert_with(|| PeerInfo {
                peer_id: peer_id.to_string(),
                addresses: Vec::new(),
                last_seen: now,
                client_version: None,
                protocol_version: None,
                connected: false,
                direction: None,
                reputation: 0,
            });
        
        peer_info.last_seen = now;
        update(peer_info);
    }
    
    /// Check if we have capacity for more outbound connections
    pub fn can_connect_outbound(&self) -> bool {
        let outbound_count = self.known_peers.values()
            .filter(|p| p.connected && p.direction.as_deref() == Some("outbound"))
            .count();
        
        outbound_count < self.config.max_outbound
    }
    
    /// Check if we have capacity for more inbound connections
    pub fn can_accept_inbound(&self) -> bool {
        let inbound_count = self.known_peers.values()
            .filter(|p| p.connected && p.direction.as_deref() == Some("inbound"))
            .count();
        
        inbound_count < self.config.max_inbound
    }
    
    /// Get reputation for a peer
    pub fn get_reputation(&self, peer_id: &PeerId) -> i32 {
        let reputation = self.reputation.read().unwrap();
        reputation.get_score(&peer_id.to_string())
    }
    
    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        let reputation = self.reputation.read().unwrap();
        reputation.is_banned(&peer_id.to_string())
    }
    
    /// Bootstrap the node with initial peers
    pub fn bootstrap(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // If we have no peers or too few, use the bootstrap nodes
        if self.known_peers.values().filter(|p| p.connected).count() < 3 {
            for node in &self.config.bootstrap_nodes {
                match parse_peer_addr(node) {
                    Ok((peer_id, addr)) => {
                        // Add to Kademlia
                        self.kad.add_address(&peer_id, addr.clone());
                        
                        // Add to known peers
                        self.update_peer_info(peer_id, |info| {
                            if !info.addresses.contains(&addr.to_string()) {
                                info.addresses.push(addr.to_string());
                            }
                            info.last_seen = current_time_secs();
                        });
                        
                        // Add to discovered peers for connection
                        if !self.is_connected(&peer_id) && !self.is_banned(&peer_id) {
                            self.discovered_peers.insert(peer_id);
                        }
                    },
                    Err(e) => {
                        println!("Failed to parse bootstrap node {}: {}", node, e);
                    }
                }
            }
        }
        
        // Prioritize bootstrap nodes
        self.start_discovery();
        
        Ok(())
    }
    
    /// Get the next peers to attempt connections to
    pub fn next_connection_candidates(&mut self, limit: usize) -> Vec<(PeerId, Vec<String>)> {
        let mut candidates = Vec::new();
        
        // Take peers from the discovered list
        let mut to_try: Vec<PeerId> = self.discovered_peers
            .iter()
            .filter(|p| !self.is_connected(p) && !self.is_banned(p))
            .cloned()
            .collect();
        
        // Sort by reputation
        to_try.sort_by(|a, b| {
            let a_rep = self.get_reputation(a);
            let b_rep = self.get_reputation(b);
            b_rep.cmp(&a_rep) // Higher reputation first
        });
        
        // Take the top 'limit' peers
        for peer_id in to_try.into_iter().take(limit) {
            if let Some(info) = self.known_peers.get(&peer_id) {
                candidates.push((peer_id, info.addresses.clone()));
            }
        }
        
        candidates
    }
    
    /// Mark a peer as connected
    pub fn mark_connected(&mut self, peer_id: &PeerId, is_outbound: bool) {
        let direction = if is_outbound { "outbound" } else { "inbound" };
        
        self.update_peer_info(*peer_id, |info| {
            info.connected = true;
            info.direction = Some(direction.to_string());
        });
        
        // Remove from discovered peers
        self.discovered_peers.remove(peer_id);
    }
    
    /// Mark a peer as disconnected
    pub fn mark_disconnected(&mut self, peer_id: &PeerId) {
        self.update_peer_info(*peer_id, |info| {
            info.connected = false;
            info.direction = None;
        });
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for DiscoveryBehaviour {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryCompleted { id, result, .. } => {
                // Remove from active queries
                self.active_queries.remove(&id);
                
                match result {
                    QueryResult::GetClosestPeers(Ok(peers)) => {
                        for peer in peers {
                            if !self.is_banned(&peer) {
                                self.discovered_peers.insert(peer);
                            }
                        }
                    }
                    _ => {} // Ignore other query results
                }
            }
            KademliaEvent::RoutingUpdated { peer, .. } => {
                let addresses = self.kad.addresses_of_peer(&peer)
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>();
                
                self.update_peer_info(peer, |info| {
                    info.addresses = addresses.clone();
                });
                
                // If we're not connected to this peer yet, consider it discovered
                if !self.is_connected(&peer) && !self.is_banned(&peer) {
                    self.discovered_peers.insert(peer);
                }
            }
            _ => {} // Ignore other events
        }
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for DiscoveryBehaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        if let IdentifyEvent::Received { peer_id, info } = event {
            self.update_peer_info(peer_id, |peer_info| {
                peer_info.client_version = Some(info.agent_version);
                peer_info.protocol_version = Some(info.protocol_version);
                
                // Add the addresses it's listening on
                for addr in info.listen_addrs {
                    if !peer_info.addresses.contains(&addr.to_string()) {
                        peer_info.addresses.push(addr.to_string());
                    }
                }
            });
            
            // Add Kademlia addresses
            for addr in info.listen_addrs {
                self.kad.add_address(&peer_id, addr);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<PingEvent> for DiscoveryBehaviour {
    fn inject_event(&mut self, event: PingEvent) {
        if let PingEvent::Ping { peer, .. } = event {
            // Update last seen time
            if let Some(info) = self.known_peers.get_mut(&peer) {
                info.last_seen = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for DiscoveryBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    if !self.is_banned(&peer_id) {
                        // Add to Kademlia routing table
                        self.kad.add_address(&peer_id, addr.clone());
                        
                        // Update our peer info
                        self.update_peer_info(peer_id, |info| {
                            if !info.addresses.contains(&addr.to_string()) {
                                info.addresses.push(addr.to_string());
                            }
                        });
                        
                        // Add to discovered peers
                        self.discovered_peers.insert(peer_id);
                    }
                }
            }
            MdnsEvent::Expired(peers) => {
                for (peer_id, addr) in peers {
                    // Remove address from Kademlia
                    self.kad.remove_address(&peer_id, &addr);
                    
                    // Update our peer info
                    self.update_peer_info(peer_id, |info| {
                        info.addresses.retain(|a| a != &addr.to_string());
                    });
                }
            }
        }
    }
}

/// Parse a peer address string into a PeerId and Multiaddr
fn parse_peer_addr(addr_str: &str) -> Result<(PeerId, libp2p::core::Multiaddr), String> {
    let addr = addr_str.parse::<libp2p::core::Multiaddr>()
        .map_err(|e| format!("Invalid multiaddr: {}", e))?;
    
    let peer_id = match addr.iter().last() {
        Some(libp2p::core::multiaddr::Protocol::P2p(hash)) => {
            PeerId::from_multihash(hash)
                .map_err(|_| "Invalid peer ID hash".to_string())?
        }
        _ => return Err("Multiaddr doesn't contain peer ID".to_string()),
    };
    
    // Remove the peer ID from the address
    let mut addr_without_peer_id = addr.clone();
    addr_without_peer_id.pop();
    
    Ok((peer_id, addr_without_peer_id))
} 