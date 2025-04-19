use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::sync::{Arc, Mutex};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub timestamp: u64,
    pub data: NodeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeData {
    ProposalCreated { proposal_id: String, title: String },
    VoteCast { proposal_id: String, voter: String, vote: f64 },
    ProposalExecuted { proposal_id: String, success: bool },
    TokenMinted { resource: String, recipient: String, amount: f64 },
}

impl DagNode {
    pub fn compute_id(&self) -> String {
        let serialized = serde_json::to_vec(self).unwrap();
        let hash = Sha256::digest(&serialized);
        hex::encode(hash)
    }
}

/// The DagLedger stores and manages a collection of DagNodes
#[derive(Clone)]
pub struct DagLedger {
    nodes: Arc<Mutex<Vec<DagNode>>>,
}

// Implement Debug for DagLedger
impl fmt::Debug for DagLedger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.nodes.lock() {
            Ok(nodes) => {
                f.debug_struct("DagLedger")
                    .field("nodes_count", &nodes.len())
                    .finish()
            }
            Err(_) => {
                f.debug_struct("DagLedger")
                    .field("nodes", &"<mutex poisoned>")
                    .finish()
            }
        }
    }
}

impl DagLedger {
    /// Create a new empty DAG ledger
    pub fn new() -> Self {
        DagLedger {
            nodes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Append a new node to the DAG
    pub fn append(&self, mut node: DagNode) -> String {
        // Compute a proper ID for the node
        let id = node.compute_id();
        node.id = id.clone();
        
        // Add to the ledger
        let mut nodes = self.nodes.lock().unwrap();
        nodes.push(node);
        
        id
    }
    
    /// Retrieve all nodes in the DAG
    pub fn trace_all(&self) -> Vec<DagNode> {
        let nodes = self.nodes.lock().unwrap();
        nodes.clone()
    }
    
    /// Find a node by its ID
    pub fn find_by_id(&self, id: &str) -> Option<DagNode> {
        let nodes = self.nodes.lock().unwrap();
        nodes.iter().find(|node| node.id == id).cloned()
    }
} 