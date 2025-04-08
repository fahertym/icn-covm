# ICN-COVM Federation Testing Environment

This directory contains files for testing the ICN-COVM federation functionality using Docker Compose.

## Overview

The federation layer enables ICN-COVM nodes to discover each other, establish connections, and communicate over the P2P network. This test environment sets up multiple containerized ICN-COVM instances to validate the core federation capabilities.

## Setup

### Prerequisites

- Docker and Docker Compose installed
- Rust environment for building the project (if not using pre-built images)

### Components

1. **Dockerfile**: Builds the ICN-COVM binary
2. **docker-compose.yml**: Defines three nodes (node1, node2, node3)
3. **update_bootstrap.sh**: Helper script to update bootstrap node information

## Running the Tests

### Building the Environment

```bash
# Build all images
docker-compose build
```

### Start the Bootstrap Node

```bash
# Start only the bootstrap node
docker-compose up node1

# In the logs, look for the PeerId, which will look something like:
# "Local peer ID: 12D3KooWAbCdEfGhIjKlMnOpQrStUvXz123456789"
```

### Update the Bootstrap Node Information

Once you have the bootstrap node's PeerId, update the docker-compose.yml file using the helper script:

```bash
./update_bootstrap.sh 12D3KooWAbCdEfGhIjKlMnOpQrStUvXz123456789
```

### Start Other Nodes

In a separate terminal:

```bash
docker-compose up node2 node3
```

### Verify Connectivity

Check the logs for connection messages such as:

- "Connected to 12D3KooWAbCdEfGhIjKlMnOpQrStUvXz123456789"
- "mDNS discovered peer ... at ..."
- "Kademlia query ... found ... peers"

## CLI Options for Federation

The following CLI options control federation behavior:

- `--enable-federation`: Enable federation support
- `--federation-port PORT`: Port number for federation listening
- `--bootstrap-nodes MULTIADDR`: Multiaddresses of bootstrap nodes (can be used multiple times)
- `--node-name NAME`: Human-readable name for this node
- `--capabilities CAPABILITY`: Node capabilities (can be used multiple times)
- `--log-level LEVEL`: Log level (error, warn, info, debug, trace)

## Example Commands

```bash
# Run a node with federation enabled on port 8000
icn-covm run --enable-federation --federation-port 8000 --node-name "node1" --log-level debug

# Run a node that connects to a bootstrap node
icn-covm run --enable-federation --federation-port 8001 --bootstrap-nodes "/ip4/192.168.1.10/tcp/8000/p2p/12D3KooWAbCdEfGhIjKlMnOpQrStUvXz123456789" --node-name "node2" --log-level debug
``` 