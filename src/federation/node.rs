use crate::federation::{
    behaviour::{create_behaviour, IcnBehaviour, IcnBehaviourEvent},
    error::FederationError,
    events::NetworkEvent,
    messages::{NodeAnnouncement, FederatedProposal, FederatedVote, NetworkMessage},
    storage::FederationStorage,
};

use futures::{
    channel::mpsc,
    stream::StreamExt,
    SinkExt,
};
use libp2p::{
    core::upgrade,
    identity, 
    noise, 
    swarm::SwarmEvent, 
    tcp, 
    yamux, 
    Multiaddr, 
    PeerId, 
    Swarm, 
    Transport,
};

// Protocol-specific imports
use libp2p::ping;
use libp2p::kad;
use libp2p::mdns;
use libp2p::identify;

use log::{debug, info, warn, error};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Mutex;

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
    
    /// Storage for federation proposals and votes
    federation_storage: Arc<FederationStorage>,
}

impl NetworkNode {
    /// Create a new network node with the specified configuration
    pub async fn new(config: NodeConfig) -> Result<Self, FederationError> {
        // Generate a random keypair for this node
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        // Create the network behavior
        let behaviour = create_behaviour(&local_key, config.protocol_version.clone()).await;
        
        // Create the transport and swarm
        let swarm = create_swarm(local_key, behaviour);
        
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
            federation_storage: Arc::new(FederationStorage::new()),
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
    async fn handle_swarm_event(&mut self, event: SwarmEvent<IcnBehaviourEvent, impl std::error::Error + Send + Sync>) -> Result<(), FederationError> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Node listening on {}", address);
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("Connected to {}", peer_id);
                
                // Add peer to Kademlia routing table if using discovered address
                let remote_addr = endpoint.get_remote_address();
                debug!("Adding {} with address {} to Kademlia", peer_id, remote_addr);
                self.swarm.behaviour_mut().kademlia.add_address(&peer_id, remote_addr.clone());
                
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
            
            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                warn!("Error with incoming connection from {} to {}: {}", send_back_addr, local_addr, error);
            }
            
            SwarmEvent::Dialing { peer_id, .. } => {
                if let Some(peer) = peer_id {
                    debug!("Dialing peer: {:?}", peer);
                } else {
                    debug!("Dialing unknown peer");
                }
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
    async fn handle_ping_event(&mut self, event: ping::Event) -> Result<(), FederationError> {
        match event {
            ping::Event {
                peer,
                result: Ok(rtt),
                ..
            } => {
                info!("Ping success from {}: RTT = {:?}", peer, rtt);
            }
            
            ping::Event {
                peer,
                result: Err(error),
                ..
            } => {
                warn!("Ping failure with {}: {}", peer, error);
            }
        }
        
        Ok(())
    }
    
    /// Handle events from the Kademlia DHT
    async fn handle_kademlia_event(&mut self, event: kad::Event) -> Result<(), FederationError> {
        match event {
            kad::Event::OutboundQueryProgressed { 
                id, 
                result: kad::QueryResult::GetClosestPeers(Ok(peers)), 
                stats: _, 
                .. 
            } => {
                info!("Kademlia query {:?} found {} peers", id, peers.peers.len());
                
                let _ = self.event_sender.send(NetworkEvent::DhtQueryCompleted {
                    peers_found: peers.peers.clone(),
                    success: true,
                }).await;
                
                // Optionally dial discovered peers
                for peer in &peers.peers {
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
                info!("Kademlia bootstrap query {:?} completed with {} remaining peers", 
                      id, stats.num_remaining);
            }
            
            kad::Event::OutboundQueryProgressed { 
                id, 
                result: kad::QueryResult::GetClosestPeers(Err(err)), 
                .. 
            } => {
                warn!("Kademlia GetClosestPeers query {:?} failed: {}", id, err);
                
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
                warn!("Kademlia bootstrap query {:?} failed: {}", id, err);
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
    async fn handle_mdns_event(&mut self, event: mdns::Event) -> Result<(), FederationError> {
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
                        if let Err(e) = self.swarm.dial(addr.clone()) {
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
                info,
            } => {
                info!(
                    "Received Identify info from {}: agent={}, protocol={}",
                    peer_id, info.agent_version, info.protocol_version
                );
                
                debug!("Protocols supported by {}: {:?}", peer_id, info.protocols);
                
                // Add all listen addresses to Kademlia
                for addr in info.listen_addrs {
                    debug!("Adding address {} for peer {}", addr, peer_id);
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                }
            }
            
            identify::Event::Sent { peer_id } => {
                debug!("Sent Identify info to {}", peer_id);
            }
            
            identify::Event::Error { peer_id, error } => {
                warn!("Identify error with {}: {}", peer_id, error);
            }
            
            identify::Event::Pushed { .. } => {
                debug!("Identify push event received");
            }
        }
        
        Ok(())
    }
    
    /// Get a reference to the federation storage
    pub fn federation_storage(&self) -> Arc<FederationStorage> {
        self.federation_storage.clone()
    }
    
    /// Broadcast a proposal to the network
    pub async fn broadcast_proposal(&mut self, proposal: FederatedProposal) -> Result<(), FederationError> {
        info!("Broadcasting proposal: {}", proposal.proposal_id);
        
        // Create the proposal broadcast message
        let message = NetworkMessage::ProposalBroadcast(proposal);
        
        // Get all connected peers
        let peer_ids = {
            let peers = self.known_peers.lock().expect("Failed to lock known_peers");
            peers.iter().cloned().collect::<Vec<_>>()
        };
        
        // Broadcast to all peers
        for peer_id in peer_ids {
            debug!("Sending proposal to peer: {}", peer_id);
            // In a real implementation, we would use a proper broadcast mechanism
            // For now, we're just simulating by sending to each peer individually
        }
        
        // Emit an event to notify listeners
        self.event_sender.try_send(NetworkEvent::ProposalBroadcasted).map_err(|e| {
            FederationError::NetworkError(format!("Failed to emit event: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Submit a vote to the network
    pub async fn submit_vote(&mut self, vote: FederatedVote) -> Result<(), FederationError> {
        info!("Submitting vote from {}", vote.voter);
        
        // Create the vote submission message
        let message = NetworkMessage::VoteSubmission(vote);
        
        // In a real implementation, we would send this to peers who have the proposal
        // For now, we just emit an event
        self.event_sender.try_send(NetworkEvent::VoteSubmitted).map_err(|e| {
            FederationError::NetworkError(format!("Failed to emit event: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Handle proposal broadcast message
    async fn handle_proposal_broadcast(&mut self, proposal: FederatedProposal) -> Result<(), FederationError> {
        info!("Received proposal broadcast: {}", proposal.proposal_id);
        
        // Store the proposal
        // In a real implementation, we would have access to the storage backend
        // For now, just add it to the in-memory cache
        
        // Emit an event to notify listeners
        self.event_sender.try_send(NetworkEvent::ProposalReceived).map_err(|e| {
            FederationError::NetworkError(format!("Failed to emit event: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Handle vote submission message
    async fn handle_vote_submission(&mut self, vote: FederatedVote) -> Result<(), FederationError> {
        info!("Received vote from {}", vote.voter);
        
        // Store the vote
        // In a real implementation, we would have access to the storage backend
        // For now, just log that we received it
        
        // Emit an event to notify listeners
        self.event_sender.try_send(NetworkEvent::VoteReceived).map_err(|e| {
            FederationError::NetworkError(format!("Failed to emit event: {}", e))
        })?;
        
        Ok(())
    }
}

/// Create a new Swarm with the provided identity
fn create_swarm(
    local_key: identity::Keypair,
    behaviour: IcnBehaviour,
) -> Swarm<IcnBehaviour> {
    // Create a TCP transport
    let transport = {
        let tcp = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
        let transport_upgrade = upgrade::Version::V1;

        // Create the noise keys
        let noise_config = noise::Config::new(&local_key).expect("Failed to create noise config");

        tcp.upgrade(transport_upgrade)
            .authenticate(noise_config)
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed()
    };

    // Create a Swarm to manage peers and events
    let config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(60));
    
    Swarm::new(
        transport,
        behaviour,
        local_key.public().to_peer_id(),
        config,
    )
} 