use crate::federation::{
    behaviour::{create_behaviour, IcnBehaviour, IcnBehaviourEvent},
    error::FederationError,
    events::NetworkEvent,
    messages::{NetworkMessage, NodeAnnouncement, Ping, Pong},
    PROTOCOL_ID,
};

use futures::{
    channel::mpsc,
    stream::StreamExt,
    SinkExt,
};
use libp2p::{
    core::{transport::Boxed, upgrade},
    identity, noise, swarm::{SwarmEvent, ConnectionId, ConnectionError}, tcp, yamux, Multiaddr, PeerId, Swarm, 
    Transport,
};
use log::{debug, info, warn, error};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc as tokio_mpsc, Mutex};

/// Configuration options for a network node
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Optional fixed port to listen on (otherwise uses an ephemeral port)
    pub port: Option<u16>,
    
    /// List of bootstrap nodes to connect to when starting
    pub bootstrap_nodes: Vec<Multiaddr>,
    
    /// Node's human-readable name
    pub name: Option<String>,
    
    /// Node capabilities (services/features provided)
    pub capabilities: Vec<String>,
    
    /// Protocol version
    pub protocol_version: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            port: None,
            bootstrap_nodes: Vec::new(),
            name: None,
            capabilities: Vec::new(),
            protocol_version: "1.0.0".to_string(),
        }
    }
}

/// Main network node for the federation layer
pub struct NetworkNode {
    /// Libp2p swarm that handles network events
    swarm: Swarm<IcnBehaviour>,
    
    /// Local peer ID
    local_peer_id: PeerId,
    
    /// Network configuration
    config: NodeConfig,
    
    /// Flag indicating if the node is running
    running: Arc<AtomicBool>,
    
    /// Channel for receiving network events
    event_receiver: mpsc::Receiver<NetworkEvent>,
    
    /// Channel for sending network events
    event_sender: mpsc::Sender<NetworkEvent>,
    
    /// Store tracking known peers
    known_peers: Arc<Mutex<HashSet<PeerId>>>,
}

impl NetworkNode {
    /// Create a new network node with the specified configuration
    pub async fn new(config: NodeConfig) -> Result<Self, FederationError> {
        // Generate a random keypair for this node
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        // Create the transport layer (TCP + Noise for encryption + Yamux for multiplexing)
        let transport = libp2p::tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(local_key.clone()).into_authenticated())
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed();
        
        // Create the network behavior
        let behaviour = create_behaviour(&local_key, config.protocol_version.clone()).await;
        
        // Build the swarm
        let swarm = libp2p::SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();
        
        // Create a channel for network events
        let (event_sender, event_receiver) = mpsc::channel::<NetworkEvent>(32);
        
        Ok(Self {
            swarm,
            local_peer_id,
            config,
            running: Arc::new(AtomicBool::new(false)),
            event_receiver,
            event_sender,
            known_peers: Arc::new(Mutex::new(HashSet::new())),
        })
    }
    
    /// Start the network node and begin processing events
    pub async fn start(&mut self) -> Result<(), FederationError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        // Set the running flag
        self.running.store(true, Ordering::SeqCst);
        
        // Listen on the provided port or an ephemeral port
        let listen_addr = match self.config.port {
            Some(port) => format!("/ip4/0.0.0.0/tcp/{}", port),
            None => "/ip4/0.0.0.0/tcp/0".to_string(),
        };
        
        match self.swarm.listen_on(listen_addr.parse()?) {
            Ok(_) => {
                info!("Node listening for connections");
            }
            Err(e) => {
                error!("Failed to listen: {}", e);
                return Err(FederationError::NetworkError(format!("Failed to listen: {}", e)));
            }
        }
        
        // Connect to bootstrap nodes
        for addr in &self.config.bootstrap_nodes {
            debug!("Dialing bootstrap node: {}", addr);
            match self.swarm.dial(addr.clone()) {
                Ok(_) => {}
                Err(e) => {
                    warn!("Failed to dial bootstrap node {}: {}", addr, e);
                }
            }
        }
        
        // Create node announcement
        let announcement = self.create_node_announcement();
        debug!("Created node announcement: {:?}", announcement);
        
        // Start the event loop
        self.process_events().await?;
        
        Ok(())
    }
    
    /// Stop the network node
    pub async fn stop(&mut self) {
        info!("Stopping network node");
        self.running.store(false, Ordering::SeqCst);
    }
    
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }
    
    /// Create a node announcement message
    fn create_node_announcement(&self) -> NodeAnnouncement {
        NodeAnnouncement {
            node_id: self.local_peer_id.to_string(),
            capabilities: self.config.capabilities.clone(),
            version: self.config.protocol_version.clone(),
            name: self.config.name.clone(),
        }
    }
    
    /// Process network events in a loop
    async fn process_events(&mut self) -> Result<(), FederationError> {
        info!("Starting network event processing loop");
        
        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                swarm_event = self.swarm.select_next_some() => {
                    if let Err(e) = self.handle_swarm_event(swarm_event).await {
                        error!("Error handling swarm event: {}", e);
                        // Send error to event channel
                        let _ = self.event_sender.send(NetworkEvent::Error(e.to_string())).await;
                    }
                }
            }
        }
        
        info!("Network event processing loop stopped");
        Ok(())
    }
    
    /// Handle events from the libp2p swarm
    async fn handle_swarm_event(&mut self, event: SwarmEvent<IcnBehaviourEvent, ConnectionError>) -> Result<(), FederationError> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Node listening on {}", address);
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("Connected to {}", peer_id);
                
                // Add peer to Kademlia routing table if using discovered address
                if let Some(addr) = endpoint.get_remote_address() {
                    debug!("Adding {} with address {} to Kademlia", peer_id, addr);
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                }
                
                // Add peer to known peers
                let mut peers = self.known_peers.lock().await;
                peers.insert(peer_id);
                
                // Notify about new connection
                let _ = self.event_sender.send(NetworkEvent::PeerConnected(peer_id)).await;
            }
            
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                if let Some(error) = cause {
                    warn!("Connection to {} closed due to error: {:?}", peer_id, error);
                } else {
                    info!("Disconnected from {}", peer_id);
                }
                
                // Notify about disconnection
                let _ = self.event_sender.send(NetworkEvent::PeerDisconnected(peer_id)).await;
            }
            
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer) = peer_id {
                    warn!("Error connecting to {}: {}", peer, error);
                } else {
                    warn!("Outgoing connection error: {}", error);
                }
            }
            
            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error } => {
                warn!("Error with incoming connection from {} to {}: {}", send_back_addr, local_addr, error);
            }
            
            SwarmEvent::Dialing(peer_id) => {
                debug!("Dialing peer: {}", peer_id);
            }
            
            SwarmEvent::ListenerClosed { addresses, reason, .. } => {
                warn!("Listener closed for addresses {:?}, reason: {:?}", addresses, reason);
            }
            
            SwarmEvent::ListenerError { error, .. } => {
                error!("Listener error: {}", error);
            }
            
            SwarmEvent::Behaviour(behaviour_event) => {
                // Handle protocol-specific events
                self.handle_behaviour_event(behaviour_event).await?;
            }
            
            _ => {
                debug!("Unhandled swarm event: {:?}", event);
            }
        }
        
        Ok(())
    }
    
    /// Handle events from the network behavior
    async fn handle_behaviour_event(&mut self, event: IcnBehaviourEvent) -> Result<(), FederationError> {
        match event {
            IcnBehaviourEvent::Ping(ping_event) => {
                self.handle_ping_event(ping_event).await
            }
            
            IcnBehaviourEvent::Kademlia(kad_event) => {
                self.handle_kademlia_event(kad_event).await
            }
            
            IcnBehaviourEvent::Mdns(mdns_event) => {
                self.handle_mdns_event(mdns_event).await
            }
            
            IcnBehaviourEvent::Identify(identify_event) => {
                self.handle_identify_event(*identify_event).await
            }
        }
    }
    
    /// Handle events from the ping protocol
    async fn handle_ping_event(&mut self, event: libp2p::ping::Event) -> Result<(), FederationError> {
        match event {
            libp2p::ping::Event {
                peer,
                result: Ok(ping::Success { rtt }),
                ..
            } => {
                debug!("Ping success from {}, rtt: {:?}", peer, rtt);
            }
            
            libp2p::ping::Event {
                peer,
                result: Err(err),
                ..
            } => {
                warn!("Ping failure for {}: {}", peer, err);
            }
        }
        
        Ok(())
    }
    
    /// Handle events from the Kademlia DHT
    async fn handle_kademlia_event(&mut self, event: libp2p::kad::Event) -> Result<(), FederationError> {
        match event {
            kad::Event::OutboundQueryProgressed { 
                id, 
                result: kad::QueryResult::GetClosestPeers(Ok(peers)), 
                stats, 
                .. 
            } => {
                info!("Kademlia query {} found {} peers", id, peers.len());
                
                let _ = self.event_sender.send(NetworkEvent::DhtQueryCompleted {
                    peers_found: peers.clone(),
                    success: true,
                }).await;
                
                // Optionally dial discovered peers
                for peer in &peers {
                    if !self.known_peers.lock().await.contains(peer) {
                        debug!("Discovered new peer via DHT: {}", peer);
                    }
                }
            }
            
            kad::Event::OutboundQueryProgressed { 
                id, 
                result: kad::QueryResult::Bootstrap(Ok(stats)), 
                .. 
            } => {
                info!("Kademlia bootstrap query {} completed with {} total peers", 
                      id, stats.num_inserted_peers);
            }
            
            kad::Event::OutboundQueryProgressed { 
                id, 
                result: kad::QueryResult::GetClosestPeers(Err(err)), 
                .. 
            } => {
                warn!("Kademlia GetClosestPeers query {} failed: {}", id, err);
                
                let _ = self.event_sender.send(NetworkEvent::DhtQueryCompleted {
                    peers_found: Vec::new(),
                    success: false,
                }).await;
            }
            
            kad::Event::OutboundQueryProgressed { 
                id, 
                result: kad::QueryResult::Bootstrap(Err(err)), 
                .. 
            } => {
                warn!("Kademlia bootstrap query {} failed: {}", id, err);
            }
            
            kad::Event::RoutingUpdated {
                peer,
                is_new_peer,
                addresses,
                ..
            } => {
                if is_new_peer {
                    debug!("New peer in routing table: {} with {} addresses", peer, addresses.len());
                    let _ = self.event_sender.send(NetworkEvent::PeerDiscovered(peer)).await;
                }
            }
            
            _ => {
                debug!("Unhandled Kademlia event: {:?}", event);
            }
        }
        
        Ok(())
    }
    
    /// Handle events from the mDNS discovery
    async fn handle_mdns_event(&mut self, event: libp2p::mdns::Event) -> Result<(), FederationError> {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer, addr) in list {
                    info!("mDNS discovered peer {} at {}", peer, addr);
                    
                    // Add address to Kademlia
                    self.swarm.behaviour_mut().kademlia.add_address(&peer, addr.clone());
                    
                    // Notify about discovery
                    let _ = self.event_sender.send(NetworkEvent::PeerDiscovered(peer)).await;
                    
                    // Optionally, dial the peer if not already connected
                    let is_known = self.known_peers.lock().await.contains(&peer);
                    if !is_known {
                        debug!("Dialing newly discovered peer: {}", peer);
                        if let Err(e) = self.swarm.dial(addr) {
                            warn!("Failed to dial discovered peer {}: {}", peer, e);
                        }
                    }
                }
            }
            
            mdns::Event::Expired(list) => {
                for (peer, addr) in list {
                    debug!("mDNS peer {} at {} expired", peer, addr);
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle events from the identify protocol
    async fn handle_identify_event(&mut self, event: identify::Event) -> Result<(), FederationError> {
        match event {
            identify::Event::Received {
                peer_id,
                info:
                    identify::Info {
                        protocol_version,
                        agent_version,
                        listen_addrs,
                        protocols,
                        observed_addr,
                        ..
                    },
            } => {
                info!(
                    "Identify info from {}: agent={}, protocol={}, observed_addr={}",
                    peer_id, agent_version, protocol_version, observed_addr
                );
                
                debug!("Protocols supported by {}: {:?}", peer_id, protocols);
                
                // Add all listen addresses to Kademlia
                for addr in listen_addrs {
                    debug!("Adding address {} for peer {}", addr, peer_id);
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }
            
            identify::Event::Sent { peer_id } => {
                debug!("Sent identify info to {}", peer_id);
            }
            
            identify::Event::Error { peer_id, error } => {
                warn!("Identify error with {}: {}", peer_id, error);
            }
            
            _ => {
                debug!("Unhandled identify event: {:?}", event);
            }
        }
        
        Ok(())
    }
} 