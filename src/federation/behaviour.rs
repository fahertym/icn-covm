use libp2p::{
    core::upgrade,
    identify, kad, mdns, noise, ping, swarm::NetworkBehaviour, tcp, yamux,
};
use std::time::Duration;

/// Combines all the network protocols used by the federation into a single type.
#[derive(NetworkBehaviour)]
pub struct IcnBehaviour {
    /// Ping protocol for measuring peer latency
    pub ping: ping::Behaviour,
    
    /// Kademlia DHT for peer discovery and data storage
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    
    /// mDNS for local network peer discovery
    pub mdns: mdns::async_io::Behaviour,
    
    /// Identify protocol for sharing metadata about nodes
    pub identify: identify::Behaviour,
}

/// Creates a new ICN network behavior with default configuration
pub fn create_behaviour(
    local_key: &libp2p::identity::Keypair,
    protocol_version: String,
) -> IcnBehaviour {
    // Set up the ping protocol
    let ping = ping::Behaviour::new(ping::Config::new()
        .with_interval(Duration::from_secs(30))
        .with_timeout(Duration::from_secs(10))
        .with_max_failures(5));
    
    // Set up Kademlia DHT for peer discovery
    let kademlia_store = kad::store::MemoryStore::new(local_key.public().to_peer_id());
    let mut kademlia = kad::Behaviour::new(
        local_key.public().to_peer_id(),
        kademlia_store,
    );
    kademlia.set_protocol_name(format!("/icn/kad/{}", protocol_version).into_bytes());
    
    // Set up local network discovery with mDNS
    let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), local_key.public().to_peer_id())
        .expect("Failed to create mDNS behavior");
    
    // Set up identify protocol
    let identify = identify::Behaviour::new(identify::Config::new(
        format!("/icn/{}", protocol_version),
        local_key.public(),
    ));

    IcnBehaviour {
        ping,
        kademlia,
        mdns,
        identify,
    }
} 