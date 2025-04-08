use crate::federation::{
    behaviour::{create_behaviour, IcnBehaviour},
    error::FederationError,
    events::NetworkEvent,
    messages::{NetworkMessage, NodeAnnouncement, Ping, Pong},
    PROTOCOL_ID,
};

use futures::{
    channel::mpsc,
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt},
    Future, FutureExt, SinkExt,
};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    identity, noise, swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
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
        let transport = tcp::tokio::Transport::default()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::NoiseAuthenticated::xx(&local_key).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .timeout(Duration::from_secs(20))
            .boxed();
        
        // Create the network behavior
        let behaviour = create_behaviour(&local_key, config.protocol_version.clone());
        
        // Build the swarm
        let mut swarm = libp2p::SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
            .build();
        
        // Listen on the provided port or an ephemeral port
        let listen_addr = match config.port {
            Some(port) => format!("/ip4/0.0.0.0/tcp/{}", port),
            None => "/ip4/0.0.0.0/tcp/0".to_string(),
        };
        
        match swarm.listen_on(listen_addr.parse()?) {
            Ok(_) => {}
            Err(e) => return Err(FederationError::NetworkError(format!("Failed to listen: {}", e))),
        }
        
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
        
        // Connect to bootstrap nodes
        for addr in &self.config.bootstrap_nodes {
            match self.swarm.dial(addr.clone()) {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Failed to dial bootstrap node {}: {}", addr, e);
                }
            }
        }
        
        // Create node announcement
        let announcement = self.create_node_announcement();
        
        // Start the event loop
        self.process_events().await?;
        
        Ok(())
    }
    
    /// Stop the network node
    pub async fn stop(&mut self) {
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
        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle events from the libp2p swarm
    async fn handle_swarm_event(&mut self, event: SwarmEvent<IcnBehaviour>) -> Result<(), FederationError> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                log::info!("Connected to {}", peer_id);
                let mut peers = self.known_peers.lock().await;
                peers.insert(peer_id);
                let _ = self.event_sender.send(NetworkEvent::PeerConnected(peer_id)).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                log::info!("Disconnected from {}", peer_id);
                let _ = self.event_sender.send(NetworkEvent::PeerDisconnected(peer_id)).await;
            }
            SwarmEvent::Behaviour(behaviour_event) => {
                // Handle specific protocol events
                // This would need to be expanded based on the actual events from IcnBehaviour
                if let Some(peer) = self.handle_behaviour_event(behaviour_event).await? {
                    let mut peers = self.known_peers.lock().await;
                    peers.insert(peer);
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Handle events from the network behavior
    async fn handle_behaviour_event(&mut self, event: IcnBehaviourEvent) -> Result<Option<PeerId>, FederationError> {
        // This method would need to be implemented to handle the specific events from IcnBehaviour
        // For now, we'll return None as a placeholder
        Ok(None)
    }
} 