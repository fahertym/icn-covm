use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::sync::{Arc, Mutex};
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};

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
    path: Option<PathBuf>,
}

// Implement Debug for DagLedger
impl fmt::Debug for DagLedger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.nodes.lock() {
            Ok(nodes) => {
                f.debug_struct("DagLedger")
                    .field("nodes_count", &nodes.len())
                    .field("path", &self.path)
                    .finish()
            }
            Err(_) => {
                f.debug_struct("DagLedger")
                    .field("nodes", &"<mutex poisoned>")
                    .field("path", &self.path)
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
            path: None,
        }
    }

    /// Create a new DAG ledger with a path
    pub fn with_path(path: PathBuf) -> Self {
        match Self::load_from_file(&path) {
            Ok(mut ledger) => {
                ledger.path = Some(path);
                ledger
            },
            Err(e) => {
                eprintln!("Failed to load DAG ledger: {}, using empty DAG", e);
                DagLedger {
                    nodes: Arc::new(Mutex::new(Vec::new())),
                    path: Some(path),
                }
            }
        }
    }

    /// Set or update the path for this ledger
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    /// Append a new node to the DAG
    pub fn append(&self, mut node: DagNode) -> String {
        // If we have a path, use append_and_persist
        if let Some(path) = &self.path {
            match self.append_and_persist(node.clone(), path) {
                Ok(id) => return id,
                Err(e) => {
                    eprintln!("Failed to persist DAG node: {}, falling back to in-memory only", e);
                    // Fall back to in-memory append
                }
            }
        }
        
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
    
    /// Load a ledger from a JSONL file, one DagNode per line
    pub fn load_from_file(path: &Path) -> std::io::Result<Self> {
        let mut ledger = DagLedger::new();
        
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // If file doesn't exist, return empty ledger
        if !path.exists() {
            return Ok(ledger);
        }
        
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            
            match serde_json::from_str::<DagNode>(&line) {
                Ok(node) => {
                    let mut nodes = ledger.nodes.lock().unwrap();
                    nodes.push(node);
                },
                Err(e) => {
                    eprintln!("Error parsing DAG node: {}", e);
                }
            }
        }
        
        Ok(ledger)
    }
    
    /// Append a node and immediately persist it to disk
    pub fn append_and_persist(&self, mut node: DagNode, path: &Path) -> std::io::Result<String> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Compute a proper ID for the node
        let id = node.compute_id();
        node.id = id.clone();
        
        // Serialize the node
        let serialized = serde_json::to_string(&node)?;
        
        // Append to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
            
        file.write_all(serialized.as_bytes())?;
        file.write_all(b"\n")?;
        
        // Add to the in-memory ledger
        let mut nodes = self.nodes.lock().unwrap();
        nodes.push(node);
        
        Ok(id)
    }
    
    /// Export the entire ledger to a file
    pub fn export_to_file(&self, path: &Path) -> std::io::Result<()> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut file = File::create(path)?;
        let nodes = self.nodes.lock().unwrap();
        
        for node in nodes.iter() {
            let serialized = serde_json::to_string(node)?;
            file.write_all(serialized.as_bytes())?;
            file.write_all(b"\n")?;
        }
        
        Ok(())
    }
    
    /// Find the node ID for a proposal created event
    pub fn find_proposal_node_id(&self, proposal_id: &str) -> Option<String> {
        let nodes = self.nodes.lock().unwrap();
        nodes.iter().find_map(|node| match &node.data {
            NodeData::ProposalCreated { proposal_id: id, .. } if id == proposal_id => Some(node.id.clone()),
            _ => None,
        })
    }

    /// Find all vote nodes for a specific proposal
    pub fn find_vote_nodes_for(&self, proposal_id: &str) -> Vec<DagNode> {
        let nodes = self.nodes.lock().unwrap();
        nodes.iter()
            .filter(|node| match &node.data {
                NodeData::VoteCast { proposal_id: id, .. } if id == proposal_id => true,
                _ => false,
            })
            .cloned()
            .collect()
    }
    
    /// Trace a node and all its parents recursively
    pub fn trace(&self, node_id: &str) -> Vec<DagNode> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.trace_recursive(node_id, &mut result, &mut visited);
        result
    }
    
    /// Recursive helper for the trace method
    fn trace_recursive(&self, node_id: &str, result: &mut Vec<DagNode>, visited: &mut std::collections::HashSet<String>) {
        if visited.contains(node_id) {
            return;
        }
        
        visited.insert(node_id.to_string());
        
        if let Some(node) = self.find_by_id(node_id) {
            // Add this node to the result
            result.push(node.clone());
            
            // Recursively trace all parents
            for parent_id in &node.parent_ids {
                self.trace_recursive(parent_id, result, visited);
            }
        }
    }
} 