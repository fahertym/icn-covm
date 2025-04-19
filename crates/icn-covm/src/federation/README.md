# Federation Module

## Overview

The Federation module enables peer-to-peer communication between network nodes in the Cooperative Value Network. It provides mechanisms for data synchronization, message passing, and distributed consensus, allowing multiple nodes to operate as a cohesive network.

## Core Features

1. **Peer Discovery and Connection**
   - Automatic peer discovery using mDNS and DHT
   - Secure connections with noise protocol
   - NAT traversal capabilities
   - Connection management with retry and backoff

2. **Message Passing**
   - Typed message protocol
   - Reliable delivery with acknowledgments
   - Gossip-based broadcast capabilities
   - Direct peer-to-peer messaging

3. **Data Synchronization**
   - Distributed data replication
   - Conflict detection and resolution
   - Delta-based synchronization for efficiency
   - Support for eventual consistency

4. **Security**
   - Identity-based authentication
   - Message signing and verification
   - Encrypted communications
   - Peer authorization and access control

## Architecture

The federation module uses libp2p as its networking foundation, with several layers of abstraction for ease of use:

```
┌──────────────────────────────────────────────┐
│                Application                   │
└───────────────────────┬──────────────────────┘
                        │
┌───────────────────────▼──────────────────────┐
│              Federation Service               │
│                                              │
│  ┌─────────────┐    ┌─────────────────────┐  │
│  │ PeerManager │    │ MessageDispatcher   │  │
│  └─────────────┘    └─────────────────────┘  │
│                                              │
│  ┌─────────────┐    ┌─────────────────────┐  │
│  │ SyncManager │    │ GossipProtocol      │  │
│  └─────────────┘    └─────────────────────┘  │
└───────────────────────┬──────────────────────┘
                        │
┌───────────────────────▼──────────────────────┐
│                  libp2p                      │
│                                              │
│  ┌─────────────┐    ┌─────────────────────┐  │
│  │ Transport   │    │ Protocol Handlers   │  │
│  └─────────────┘    └─────────────────────┘  │
│                                              │
│  ┌─────────────┐    ┌─────────────────────┐  │
│  │ Discovery   │    │ Security            │  │
│  └─────────────┘    └─────────────────────┘  │
└──────────────────────────────────────────────┘
```

### Key Components

1. **FederationService**: Main entry point that coordinates all federation activities
2. **PeerManager**: Handles discovery, connection, and heartbeating of peers
3. **MessageDispatcher**: Routes messages to appropriate handlers
4. **SyncManager**: Coordinates data synchronization between peers
5. **GossipProtocol**: Implements efficient broadcast of messages

## Core APIs

### Federation Service

The `FederationService` provides the main API for interacting with the federation system:

```rust
// Create a new federation service
let federation = FederationService::new(identity, config)?;

// Start the federation service
federation.start()?;

// Subscribe to messages of a specific type
federation.subscribe("governance.proposal", |msg| {
    // Handle message
    println!("Received proposal: {}", msg);
});

// Publish a message to the network
federation.publish("governance.proposal", proposal_data)?;

// Send a direct message to a specific peer
federation.send_direct(peer_id, "private.message", message_data)?;

// Get connected peers
let peers = federation.connected_peers();

// Stop the federation service
federation.stop()?;
```

### Peer Management

```rust
// Get peer information
let peer_info = federation.get_peer_info(peer_id)?;

// Connect to a specific peer
federation.connect_peer(peer_address)?;

// Disconnect from a peer
federation.disconnect_peer(peer_id)?;

// Ban a peer temporarily
federation.ban_peer(peer_id, Duration::from_secs(300))?;
```

### Data Synchronization

```rust
// Register a data type for synchronization
federation.register_sync_type("governance.proposals", proposals_store)?;

// Request sync for a specific key
federation.request_sync("governance.proposals", proposal_id)?;

// Get sync status
let status = federation.get_sync_status("governance.proposals")?;
```

## Message Types

The federation module uses a typed message system for safety and clarity:

### System Messages

- **Heartbeat**: Regular messages to verify peer connectivity
- **SyncRequest**: Request for data synchronization
- **SyncResponse**: Response containing requested data
- **PeerInfo**: Information about a peer's capabilities and state

### Application Messages

- **Governance.Proposal**: Proposal creation and updates
- **Governance.Vote**: Votes on proposals
- **Resource.Transfer**: Resource transfer notifications
- **Identity.Update**: Identity information updates

## Configuration

Federation behavior can be customized through a configuration object:

```rust
let config = FederationConfig {
    listen_addresses: vec!["/ip4/0.0.0.0/tcp/9000".parse()?],
    enable_mdns: true,
    enable_kademlia: true,
    bootstrap_peers: vec![],
    max_connections: 50,
    heartbeat_interval: Duration::from_secs(30),
    sync_interval: Duration::from_secs(300),
    message_timeout: Duration::from_secs(60),
    connection_retry: ConnectionRetryConfig {
        max_retries: 5,
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(60),
    },
};
```

## CLI Integration

The federation module provides CLI commands for management and visibility:

```
# List connected peers
covm federation peers

# Show gossip log
covm federation gossip-log

# Send a direct message to a peer
covm federation send-msg <peer_id> <message_type> <message>

# Show federation statistics
covm federation stats

# Connect to a specific peer
covm federation connect <multiaddr>
```

## Example: Proposal Propagation

```rust
// Node A: Create and publish a proposal
let proposal = Proposal {
    id: "proposal-123".to_string(),
    title: "Funding allocation".to_string(),
    description: "Allocate funds to project X".to_string(),
    // ...
};

federation.publish("governance.proposal", serde_json::to_string(&proposal)?)?;

// Node B: Receive and handle the proposal
federation.subscribe("governance.proposal", |msg| {
    let proposal: Proposal = serde_json::from_str(msg)?;
    
    // Store the proposal
    proposal_store.store(&proposal.id, &proposal)?;
    
    // Notify users
    println!("New proposal received: {}", proposal.title);
    
    Ok(())
});
```

## Robustness Features

### Connection Retry with Exponential Backoff

```rust
// Example of connection retry logic
fn connect_with_retry(peer_id: PeerId, config: &ConnectionRetryConfig) -> Result<(), Error> {
    let mut retry_count = 0;
    let mut delay = config.base_delay;
    
    while retry_count < config.max_retries {
        match connect_peer(peer_id) {
            Ok(_) => return Ok(()),
            Err(e) => {
                log::warn!("Connection attempt {} failed: {}", retry_count + 1, e);
                retry_count += 1;
                
                if retry_count >= config.max_retries {
                    return Err(e.into());
                }
                
                // Exponential backoff
                std::thread::sleep(delay);
                delay = std::cmp::min(delay * 2, config.max_delay);
            }
        }
    }
    
    Err(Error::ConnectionFailed)
}
```

### Error Handling

All operations in the federation module return proper `Result` types to enable error handling:

```rust
match federation.publish("topic", data) {
    Ok(_) => {
        log::info!("Message published successfully");
    },
    Err(FederationError::NetworkError(e)) => {
        log::error!("Network error: {}", e);
        // Handle network errors specifically
    },
    Err(FederationError::MessageTooLarge) => {
        log::warn!("Message too large, splitting into chunks");
        // Handle by splitting message
    },
    Err(e) => {
        log::error!("Failed to publish message: {}", e);
        // General error handling
    }
}
``` 