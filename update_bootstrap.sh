#!/bin/bash

# This script helps update the docker-compose.yml file with bootstrap node information

# Check if we have the bootstrap node ID
if [ $# -ne 1 ]; then
  echo "Usage: $0 <bootstrap_peer_id>"
  echo "Example: $0 12D3KooWAbCdEfGhIjKlMnOpQrStUvXz123456789"
  exit 1
fi

# Get the bootstrap peer ID from command line
BOOTSTRAP_PEER_ID=$1

# Update the docker-compose.yml file
sed -i "s/BOOTSTRAP_PEER_ID_PLACEHOLDER/$BOOTSTRAP_PEER_ID/g" docker-compose.yml

echo "Updated docker-compose.yml with bootstrap peer ID: $BOOTSTRAP_PEER_ID"
echo "Run 'docker-compose up node1' first to start the bootstrap node"
echo "Then in another terminal, run 'docker-compose up node2 node3' to start the other nodes" 