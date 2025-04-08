# Federation Layer

The ICN-COVM federation layer enables communication between nodes in the Intercooperative Network. This layer provides the foundation for future federated governance features, allowing cooperatives to discover each other, establish secure connections, and exchange information.

## Overview

The federation layer is implemented as a standalone module (`src/federation/`) that integrates with the core VM and storage components. It uses the libp2p networking stack to provide:

- **Peer Discovery**: Find other nodes in the network
- **Secure Channels**: Establish encrypted connections
- **Message Exchange**: Send and receive messages
- **Event Tracking**: Monitor network activity

## Core Components

### NetworkNode

The `NetworkNode` is the main component of the federation layer. It manages connections, handles network events, and provides an interface for other components to interact with the network.

```rust
pub struct NetworkNode {
    // libp2p swarm for handling network events
    swarm: Swarm<IcnBehaviour>,
    
    // Local peer ID
    local_peer_id: PeerId,
    
    // Network configuration
    config: NodeConfig,
    
    // Event channel for network events
    event_sender: mpsc::Sender<NetworkEvent>,
    event_receiver: mpsc::Receiver<NetworkEvent>,
    
    // Known peers tracking
    known_peers: Arc<Mutex<HashSet<PeerId>>>,
}
```

### Node Configuration

The `NodeConfig` structure provides configuration options for the network node:

```rust
pub struct NodeConfig {
    // Optional fixed port to listen on
    pub port: Option<u16>,
    
    // List of bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<Multiaddr>,
    
    // Node's human-readable name
    pub name: Option<String>,
    
    // Node capabilities
    pub capabilities: Vec<String>,
    
    // Protocol version
    pub protocol_version: String,
}
```

### Network Behaviors

The federation layer uses libp2p behaviors to implement various protocols:

- **Kademlia**: Distributed Hash Table (DHT) for peer discovery
- **mDNS**: Local network peer discovery
- **Ping**: Network latency measurement
- **Identify**: Exchange node information and capabilities

These behaviors are combined in the `IcnBehaviour` struct and managed by the libp2p swarm.

### Message Types

The federation layer defines several message types for communication:

- **NodeAnnouncement**: Announces a node's presence and capabilities
- **Ping**: Verifies node connectivity
- **Pong**: Response to ping messages

These messages are serialized using the Serde framework for efficient transmission.

## Network Events

The federation layer generates events to notify other components about network activity:

- **PeerDiscovered**: A new peer was found
- **PeerConnected**: Connection established with a peer
- **PeerDisconnected**: Connection lost with a peer
- **MessageReceived**: A message was received from a peer

Applications can subscribe to these events using the event channel provided by the `NetworkNode`.

## Usage

### Starting a Network Node

To start a network node:

```rust
let config = NodeConfig {
    port: Some(8000),
    bootstrap_nodes: Vec::new(),
    name: Some("node1".to_string()),
    capabilities: vec!["storage".to_string()],
    protocol_version: "1.0.0".to_string(),
};

let mut node = NetworkNode::new(config).await?;
node.start().await?;
```

### Connecting to Bootstrap Nodes

To connect to existing nodes in the network:

```rust
let bootstrap_addr = "/ip4/192.168.1.1/tcp/8000/p2p/12D3KooWX...Z9PcBJP5"
    .parse()
    .expect("Invalid multiaddress");

let config = NodeConfig {
    bootstrap_nodes: vec![bootstrap_addr],
    ..Default::default()
};

let mut node = NetworkNode::new(config).await?;
node.start().await?;
```

## Command Line Interface

The ICN-COVM CLI provides several options for federation:

- `--enable-federation`: Enable federation support
- `--federation-port PORT`: Port number for federation listening
- `--bootstrap-nodes MULTIADDR`: Multiaddresses of bootstrap nodes
- `--node-name NAME`: Human-readable name for this node
- `--capabilities CAPABILITY`: Node capabilities

Examples:

```bash
# Start a bootstrap node
cargo run -- run --enable-federation --federation-port 8000 --node-name "bootstrap-node"

# Start a node that connects to the bootstrap node
cargo run -- run --enable-federation --federation-port 8001 --bootstrap-nodes "/ip4/192.168.1.1/tcp/8000/p2p/12D3KooWX...Z9PcBJP5" --node-name "node1"
```

## Multi-Node Testing

The ICN-COVM repository includes Docker Compose configuration for testing multiple nodes:

1. Build the Docker images: `docker-compose build`
2. Start the bootstrap node: `docker-compose up node1`
3. Get the bootstrap node's peer ID from the logs
4. Update the bootstrap node information: `./update_bootstrap.sh <peer_id>`
5. Start the other nodes: `docker-compose up node2 node3`

See the [Federation Testing Environment](../README_FEDERATION.md) for more detailed instructions.

## Limitations

The current implementation has some limitations:

- Basic message types only (more sophisticated types planned for future releases)
- No built-in message routing beyond Kademlia
- No persistent peer storage (peers must be rediscovered after restart)
- No integration with governance operations (planned for future releases)

## Future Development

Planned enhancements to the federation layer include:

- Federated voting mechanisms
- Resource sharing between nodes
- Cooperative identity verification
- Distributed proposal handling
- Consensus mechanisms for shared decisions

These features will build on the core federation infrastructure established in v0.7.0. 