use libp2p::{identify, kad, mdns, ping, request_response::{self, ProtocolName}};
use libp2p_swarm_derive::NetworkBehaviour;
use std::time::Duration;
use crate::federation::messages::NetworkMessage;
use serde::{Serialize, Deserialize};
use std::io;

// Define the name for our protocol
#[derive(Debug, Clone)]
pub struct FederationProtocol;

impl ProtocolName for FederationProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/icn-federation/1.0.0"
    }
}

// Define the request and response types for our federation protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRequest {
    pub message: NetworkMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationResponse {
    pub status: String,
    pub message: Option<String>,
}

// Codec for serializing request and response
#[derive(Debug, Clone)]
pub struct FederationCodec;

impl request_response::Codec for FederationCodec {
    type Protocol = FederationProtocol;
    type Request = FederationRequest;
    type Response = FederationResponse;

    fn read_request<T>(
        &mut self,
        _: &FederationProtocol,
        mut io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        libp2p::futures::AsyncReadExt::read_to_end(&mut io, &mut vec).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match serde_json::from_slice(&vec) {
            Ok(req) => Ok(req),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        }
    }

    fn read_response<T>(
        &mut self,
        _: &FederationProtocol,
        mut io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        libp2p::futures::AsyncReadExt::read_to_end(&mut io, &mut vec).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match serde_json::from_slice(&vec) {
            Ok(res) => Ok(res),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        }
    }

    fn write_request<T>(
        &mut self,
        _: &FederationProtocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        let vec = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        libp2p::futures::AsyncWriteExt::write_all(io, &vec).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn write_response<T>(
        &mut self,
        _: &FederationProtocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        let vec = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        libp2p::futures::AsyncWriteExt::write_all(io, &vec).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

/// Combines all the network protocols used by the federation into a single type.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "IcnBehaviourEvent")]
pub struct IcnBehaviour {
    /// Ping protocol for measuring peer latency
    pub ping: ping::Behaviour,

    /// Kademlia DHT for peer discovery and data storage
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,

    /// mDNS for local network peer discovery
    pub mdns: mdns::tokio::Behaviour,

    /// Identify protocol for sharing metadata about nodes
    pub identify: identify::Behaviour,
    
    /// Federation message protocol
    pub federation: request_response::Behaviour<FederationCodec>,
}

/// Events that can be emitted by the network behavior
#[derive(Debug)]
pub enum IcnBehaviourEvent {
    /// Events from the ping protocol
    Ping(ping::Event),

    /// Events from the Kademlia DHT
    Kademlia(kad::Event),

    /// Events from the mDNS discovery
    Mdns(mdns::Event),

    /// Events from the identify protocol
    Identify(Box<identify::Event>),
    
    /// Events from the federation protocol
    Federation(request_response::Event<FederationRequest, FederationResponse>),
}

impl From<ping::Event> for IcnBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        IcnBehaviourEvent::Ping(event)
    }
}

impl From<kad::Event> for IcnBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        IcnBehaviourEvent::Kademlia(event)
    }
}

impl From<mdns::Event> for IcnBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        IcnBehaviourEvent::Mdns(event)
    }
}

impl From<identify::Event> for IcnBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        IcnBehaviourEvent::Identify(Box::new(event))
    }
}

impl From<request_response::Event<FederationRequest, FederationResponse>> for IcnBehaviourEvent {
    fn from(event: request_response::Event<FederationRequest, FederationResponse>) -> Self {
        IcnBehaviourEvent::Federation(event)
    }
}

/// Creates a new ICN network behavior with default configuration
pub async fn create_behaviour(
    local_key: &libp2p::identity::Keypair,
    protocol_version: String,
) -> Result<IcnBehaviour, Box<dyn std::error::Error + Send + Sync>> {
    // Set up the ping protocol
    let ping = ping::Behaviour::new(
        ping::Config::new()
            .with_interval(Duration::from_secs(30))
            .with_timeout(Duration::from_secs(10)),
    );

    // Set up Kademlia DHT for peer discovery
    let kademlia_store = kad::store::MemoryStore::new(local_key.public().to_peer_id());
    let mut kademlia_config = kad::Config::default();

    // Create a protocol name with a static lifetime
    // Use a static string that will live for the lifetime of the program
    let protocol_str = format!("/icn/kad/{}", protocol_version);
    // Convert to a static string
    let protocol_name = libp2p::StreamProtocol::new(&*Box::leak(protocol_str.into_boxed_str()));

    kademlia_config.set_protocol_names(vec![protocol_name]);
    let kademlia = kad::Behaviour::with_config(
        local_key.public().to_peer_id(),
        kademlia_store,
        kademlia_config,
    );

    // Set up local network discovery with mDNS
    let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_key.public().to_peer_id())
        .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(format!("Failed to create mDNS behavior: {}", e)))?;

    // Set up identify protocol
    let identify = identify::Behaviour::new(identify::Config::new(
        format!("/icn/{}", protocol_version),
        local_key.public(),
    ));
    
    // Set up the federation protocol
    let federation_config = request_response::Config::default()
        .with_request_timeout(Duration::from_secs(30))
        .with_max_concurrent_responses(100);
        
    let federation = request_response::Behaviour::new(
        FederationCodec {},
        std::iter::once((FederationProtocol {}, federation_config)),
    );

    Ok(IcnBehaviour {
        ping,
        kademlia,
        mdns,
        identify,
        federation,
    })
}

// Handler methods
impl IcnBehaviour {
    fn on_ping(&mut self, _event: ping::Event) {
        // Pass the event to the upper layer
    }

    fn on_kademlia(&mut self, _event: kad::Event) {
        // Pass the event to the upper layer
    }

    fn on_mdns(&mut self, _event: mdns::Event) {
        // Pass the event to the upper layer
    }

    fn on_identify(&mut self, _event: identify::Event) {
        // Pass the event to the upper layer
    }
    
    fn on_federation(&mut self, _event: request_response::Event<FederationRequest, FederationResponse>) {
        // Pass the event to the upper layer
    }
}
