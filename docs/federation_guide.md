# Federation Guide for ICN-COVM

## Overview

Federation in ICN-COVM enables multiple cooperative nodes to discover, communicate, and collaborate across a distributed network. This federation layer forms the foundation for cross-cooperative governance, resource sharing, and collective decision-making.

## How Federation Works

ICN-COVM's federation is built on [libp2p](https://libp2p.io/), providing:

1. **Peer Discovery**: Nodes can discover each other through various methods (mDNS, bootstrap nodes, DHT)
2. **Secure Communication**: All inter-node communication is encrypted using TLS
3. **Distributed Identity**: Each node maintains its own identity while participating in the network
4. **Consensus Mechanisms**: For agreement on shared state across the federation

## Getting Started with Federation

### Running a Federation Node

To start a node in federation mode:

```bash
cargo run -- federation --listen /ip4/0.0.0.0/tcp/4001
```

This will:
- Generate a new node identity (or use an existing one if available)
- Start listening for connections on port 4001
- Begin discovery of other nodes on the local network

### Connecting to Other Nodes

To explicitly connect to a known node:

```bash
cargo run -- federation --connect /ip4/192.168.1.100/tcp/4001/p2p/QmNodePeerId
```

Where:
- `/ip4/192.168.1.100/tcp/4001` is the node's multiaddress
- `QmNodePeerId` is the target node's peer ID

### Federation Configuration

Create a `federation.toml` file in your project directory:

```toml
[federation]
# Node identity settings
name = "My Cooperative Node"
description = "A node for our worker cooperative"

# Network settings
listen_addresses = ["/ip4/0.0.0.0/tcp/4001"]
bootstrap_nodes = [
  "/ip4/bootstrap.icn-covm.org/tcp/4001/p2p/QmBootstrapNodeId1",
  "/ip4/bootstrap2.icn-covm.org/tcp/4001/p2p/QmBootstrapNodeId2"
]

# Discovery settings
enable_mdns = true
enable_kademlia = true

# Security settings
enable_tls = true
```

## Federated Governance

### Proposing Cross-Cooperative Actions

```bash
# Create a cross-cooperative proposal
cargo run -- federation proposal create --title "Joint Resource Sharing" --description "Proposal to share computing resources between cooperatives" --action "resource_share" --params '{"resource_type": "compute", "allocation": 25}'

# Get the proposal ID from the output
PROPOSAL_ID=<proposal_id>

# Broadcast the proposal to the federation
cargo run -- federation broadcast --proposal $PROPOSAL_ID
```

### Voting on Federation Proposals

```bash
# View active federation proposals
cargo run -- federation proposals list

# Vote on a proposal
cargo run -- federation vote --proposal $PROPOSAL_ID --decision "approve" --reason "Aligns with our resource sharing goals"
```

## Federation Security

### Identity and Trust

Each node in the ICN-COVM federation maintains a cryptographic identity. Trust relationships between nodes can be established through:

1. **Pre-shared keys**: Exchanged out-of-band between cooperatives
2. **Web of Trust**: Nodes can vouch for other nodes they trust
3. **Certificate Authorities**: For larger federated networks

### Access Control

Federation participants can define access policies for their resources:

```bash
# Set access policy for local resources
cargo run -- federation acl set --resource "compute_resources" --access "read,execute" --for QmTrustedNodeId1,QmTrustedNodeId2
```

## Advanced Topics

### Custom Federation Protocols

ICN-COVM allows defining custom protocols for specialized inter-cooperative communication:

```rust
// In your Rust code
use icn_covm::federation::{Protocol, ProtocolHandler};

struct MyCustomProtocol;

impl ProtocolHandler for MyCustomProtocol {
    fn handle_message(&self, message: &[u8]) -> Result<Vec<u8>, Error> {
        // Custom message handling logic
    }
}

// Register the protocol
federation_node.register_protocol("/mycoop/custom/1.0.0", MyCustomProtocol);
```

### Federation Events

Subscribe to federation events to react to network changes:

```bash
# Stream federation events
cargo run -- federation events --subscribe "node_join,node_leave,proposal_new,vote_cast"
```

## Troubleshooting

### Common Issues

1. **Connection Refused**: Check firewall settings and ensure the port is open
2. **Node Not Discovered**: Verify that both nodes have compatible discovery methods enabled
3. **Permission Denied**: Ensure the node has proper access rights in the federation

### Diagnostic Commands

```bash
# Check federation status
cargo run -- federation status

# View connected peers
cargo run -- federation peers list

# Test connectivity to a specific node
cargo run -- federation ping /ip4/192.168.1.100/tcp/4001/p2p/QmNodePeerId
```

## Future Federation Features

The following federation features are planned for upcoming releases:

1. **Federated Governance Operations** (v0.8.0)
2. **Resource Sharing Mechanisms** (v0.8.0)
3. **Inter-Cooperative Economic Tools** (v0.9.0)
4. **Global Federation Directory** (v1.0.0)

See the [ICN-COVM Roadmap](roadmap.md) for more details on upcoming features. 