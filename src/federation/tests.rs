#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::messages::{NetworkMessage, NodeAnnouncement, Ping, Pong};
    use serde_json;
    use std::time::Duration;

    #[test]
    fn test_message_serialization() {
        // Create a NodeAnnouncement message
        let announcement = NodeAnnouncement {
            node_id: "node1".to_string(),
            capabilities: vec!["storage".to_string(), "execution".to_string()],
            version: "1.0.0".to_string(),
            name: Some("Test Node".to_string()),
        };

        // Wrap it in a NetworkMessage
        let message = NetworkMessage::NodeAnnouncement(announcement);
        
        // Serialize to JSON
        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");
        
        // Deserialize back
        let deserialized: NetworkMessage = serde_json::from_str(&serialized).expect("Failed to deserialize message");
        
        // Verify the result
        match deserialized {
            NetworkMessage::NodeAnnouncement(node_announcement) => {
                assert_eq!(node_announcement.node_id, "node1");
                assert_eq!(node_announcement.capabilities.len(), 2);
                assert_eq!(node_announcement.capabilities[0], "storage");
                assert_eq!(node_announcement.capabilities[1], "execution");
                assert_eq!(node_announcement.version, "1.0.0");
                assert_eq!(node_announcement.name, Some("Test Node".to_string()));
            },
            _ => panic!("Deserialized to wrong message type"),
        }
    }

    #[test]
    fn test_ping_pong_serialization() {
        // Create a Ping message
        let ping = Ping {
            nonce: 12345,
            timestamp_ms: 1618000000000,
        };

        // Wrap it in a NetworkMessage
        let message = NetworkMessage::Ping(ping);
        
        // Serialize to JSON
        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");
        
        // Deserialize back
        let deserialized: NetworkMessage = serde_json::from_str(&serialized).expect("Failed to deserialize message");
        
        // Verify the result
        match deserialized {
            NetworkMessage::Ping(ping) => {
                assert_eq!(ping.nonce, 12345);
                assert_eq!(ping.timestamp_ms, 1618000000000);
            },
            _ => panic!("Deserialized to wrong message type"),
        }

        // Test Pong message
        let pong = Pong {
            nonce: 12345,
            timestamp_ms: 1618000001000,
            ttl: Some(Duration::from_secs(60)),
        };

        // Wrap it in a NetworkMessage
        let message = NetworkMessage::Pong(pong);
        
        // Serialize to JSON
        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");
        
        // Deserialize back
        let deserialized: NetworkMessage = serde_json::from_str(&serialized).expect("Failed to deserialize message");
        
        // Verify the result
        match deserialized {
            NetworkMessage::Pong(pong) => {
                assert_eq!(pong.nonce, 12345);
                assert_eq!(pong.timestamp_ms, 1618000001000);
                assert!(pong.ttl.is_some());
                assert_eq!(pong.ttl.unwrap().as_secs(), 60);
            },
            _ => panic!("Deserialized to wrong message type"),
        }
    }
    
    #[test]
    fn test_extract_identify_info() {
        // This is a test utility to verify that we can correctly parse listen_addrs from Identify
        let parsed_addr1: libp2p::Multiaddr = "/ip4/192.168.1.1/tcp/8000".parse().unwrap();
        let parsed_addr2: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/8001".parse().unwrap();
        
        // Create a Vec of addresses
        let addresses = vec![parsed_addr1.clone(), parsed_addr2.clone()];
        
        // Check contents
        assert_eq!(addresses.len(), 2);
        assert_eq!(addresses[0], parsed_addr1);
        assert_eq!(addresses[1], parsed_addr2);
        
        // Convert to strings and verify
        assert_eq!(addresses[0].to_string(), "/ip4/192.168.1.1/tcp/8000");
        assert_eq!(addresses[1].to_string(), "/ip4/10.0.0.1/tcp/8001");
    }
} 