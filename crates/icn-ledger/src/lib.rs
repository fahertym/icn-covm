use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
// Only include OS-specific imports when needed
#[cfg(target_os = "windows")]
use std::os::windows::prelude::OsStrExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub parent_ids: Vec<String>,
    pub timestamp: u64,
    pub namespace: String,
    pub data: NodeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NodeData {
    ProposalCreated {
        proposal_id: String,
        title: String,
    },
    VoteCast {
        proposal_id: String,
        voter: String,
        vote: f64,
    },
    ProposalExecuted {
        proposal_id: String,
        success: bool,
    },
    TokenMinted {
        resource: String,
        recipient: String,
        amount: f64,
    },
}

impl DagNode {
    pub fn compute_id(&self) -> String {
        let serialized = serde_json::to_vec(self).unwrap();
        let hash = Sha256::digest(&serialized);
        hex::encode(hash)
    }

    // Add a helper method to create a node with default namespace
    pub fn with_default_namespace(parent_ids: Vec<String>, data: NodeData, timestamp: u64) -> Self {
        Self {
            id: String::new(), // Will be set by compute_id later
            parent_ids,
            timestamp,
            namespace: "default".to_string(),
            data,
        }
    }

    // Add a helper method to create a node with specified namespace
    pub fn with_namespace(
        parent_ids: Vec<String>,
        data: NodeData,
        timestamp: u64,
        namespace: String,
    ) -> Self {
        Self {
            id: String::new(), // Will be set by compute_id later
            parent_ids,
            timestamp,
            namespace,
            data,
        }
    }
}

/// The DagLedger stores and manages a collection of DagNodes
#[derive(Clone)]
pub struct DagLedger {
    nodes: Vec<DagNode>,
    file_path: Option<PathBuf>,
}

// Implement Debug for DagLedger
impl fmt::Debug for DagLedger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DagLedger")
            .field("nodes_count", &self.nodes.len())
            .field("path", &self.file_path)
            .finish()
    }
}

/// Result of a diff operation between two DAG ledgers
#[derive(Debug, Clone)]
pub struct DagDiff {
    pub added: Vec<DagNode>,
    pub removed: Vec<DagNode>,
    pub common: Vec<String>, // IDs of nodes in both DAGs
}

impl DagLedger {
    /// Create a new empty DAG ledger
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            file_path: None,
        }
    }

    /// Create a new DAG ledger with a path
    pub fn with_path(path: PathBuf) -> Self {
        match Self::load_from_file(&path) {
            Ok(mut ledger) => {
                ledger.file_path = Some(path);
                ledger
            }
            Err(e) => {
                eprintln!("Failed to load DAG ledger: {}, using empty DAG", e);
                DagLedger {
                    nodes: Vec::new(),
                    file_path: Some(path),
                }
            }
        }
    }

    /// Set or update the path for this ledger
    pub fn set_path(&mut self, path: PathBuf) {
        self.file_path = Some(path);
    }

    /// Append a new node to the DAG
    pub fn append(&mut self, mut node: DagNode) -> Result<String, String> {
        // Auto-generate ID
        node.id = node.compute_id();
        self.nodes.push(node.clone());
        Ok(node.id)
    }

    pub fn nodes(&self) -> &Vec<DagNode> {
        &self.nodes
    }

    pub fn find_by_id(&self, id: &str) -> Option<&DagNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    // New method to filter nodes by namespace
    pub fn nodes_by_namespace(&self, namespace: &str) -> Vec<&DagNode> {
        self.nodes
            .iter()
            .filter(|n| n.namespace == namespace)
            .collect()
    }

    pub fn trace_all(&self) -> Result<String, String> {
        let mut result = String::new();
        for node in &self.nodes {
            result.push_str(&format!("{}\n", self.trace(node)?));
        }
        Ok(result)
    }

    // New method to trace all nodes in a specific namespace
    pub fn trace_namespace(&self, namespace: &str) -> Result<String, String> {
        let mut result = String::new();
        for node in self.nodes_by_namespace(namespace) {
            result.push_str(&format!("{}\n", self.trace(node)?));
        }
        Ok(result)
    }

    /// Retrieve all nodes in the DAG
    pub fn trace_all_nodes(&self) -> Vec<DagNode> {
        self.nodes.clone()
    }

    /// Find a node by its ID
    pub fn find_by_id_nodes(&self, id: &str) -> Option<DagNode> {
        self.nodes.iter().find(|node| node.id == id).cloned()
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
                    ledger.nodes.push(node);
                }
                Err(e) => {
                    eprintln!("Error parsing DAG node: {}", e);
                }
            }
        }

        Ok(ledger)
    }

    /// Append a node and immediately persist it to disk
    pub fn append_and_persist(&mut self, node: DagNode) -> Result<String, String> {
        if self.file_path.is_none() {
            return Err("File path is not set".to_string());
        }

        let node_id = self.append(node)?;
        self.export_to_file().map_err(|e| e.to_string())?;
        Ok(node_id)
    }

    /// Export the entire ledger to a file
    pub fn export_to_file(&self) -> std::io::Result<()> {
        if let Some(path) = &self.file_path {
            let mut file = File::create(path)?;
            let nodes = self.nodes.iter();

            for node in nodes {
                let serialized = serde_json::to_string(node)?;
                file.write_all(serialized.as_bytes())?;
                file.write_all(b"\n")?;
            }

            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "File path is not set"))
        }
    }

    /// Find the node ID for a proposal created event
    pub fn find_proposal_node_id(&self, proposal_id: &str) -> Option<String> {
        self.nodes.iter().find_map(|node| match &node.data {
            NodeData::ProposalCreated {
                proposal_id: id, ..
            } if id == proposal_id => Some(node.id.clone()),
            _ => None,
        })
    }

    /// Find all vote nodes for a specific proposal
    pub fn find_vote_nodes_for(&self, proposal_id: &str) -> Vec<DagNode> {
        self.nodes
            .iter()
            .filter(|node| match &node.data {
                NodeData::VoteCast {
                    proposal_id: id, ..
                } if id == proposal_id => true,
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// Trace a node and all its parents recursively
    pub fn trace(&self, node: &DagNode) -> Result<String, String> {
        let mut result = String::new();
        let mut visited = std::collections::HashSet::new();
        self.trace_recursive(node, &mut result, &mut visited)?;
        Ok(result)
    }

    /// Recursive helper for the trace method
    fn trace_recursive(
        &self,
        node: &DagNode,
        result: &mut String,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        if visited.contains(&node.id) {
            return Ok(());
        }

        visited.insert(node.id.clone());

        // Add this node to the result
        result.push_str(&format!("{:?}\n", node));

        // Recursively trace all parents
        for parent_id in &node.parent_ids {
            if let Some(parent_node) = self.find_by_id(parent_id) {
                self.trace_recursive(parent_node, result, visited)?;
            } else {
                return Err(format!("Parent node {} not found", parent_id));
            }
        }

        Ok(())
    }

    /// Export nodes matching the provided list of IDs
    pub fn export_nodes(&self, ids: &[String]) -> Vec<DagNode> {
        self.nodes
            .iter()
            .filter(|node| ids.contains(&node.id))
            .cloned()
            .collect()
    }

    /// Return a list of all node IDs in the DAG
    pub fn all_node_ids(&self) -> Vec<String> {
        self.nodes.iter().map(|node| node.id.clone()).collect()
    }

    /// Import nodes from a JSONL file (only missing ones)
    pub fn import_from_file(&mut self, path: &Path) -> std::io::Result<usize> {
        // Only proceed if the file exists
        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut added = 0;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<DagNode>(&line) {
                Ok(node) => {
                    // Check if this node is already in our collection
                    if !self.nodes.iter().any(|existing| existing.id == node.id) {
                        self.nodes.push(node);
                        added += 1;
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing DAG node: {}", e);
                }
            }
        }

        Ok(added)
    }

    /// Export all nodes as a Vec
    pub fn export_all(&self) -> Vec<DagNode> {
        self.nodes.clone()
    }

    /// Export selected nodes and their reachable parent nodes
    pub fn export_selected(&self, start_ids: &[String]) -> Vec<DagNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();

        // Process each starting node
        for start_id in start_ids {
            self.export_recursive(start_id, &mut result, &mut visited);
        }

        result
    }

    /// Recursive helper for exporting selected nodes
    fn export_recursive(
        &self,
        node_id: &str,
        result: &mut Vec<DagNode>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(node_id) {
            return;
        }

        visited.insert(node_id.to_string());

        if let Some(node) = self.find_by_id_nodes(node_id) {
            // Add this node to the result
            result.push(node.clone());

            // Recursively process all parent nodes
            for parent_id in &node.parent_ids {
                self.export_recursive(parent_id, result, visited);
            }
        }
    }

    /// Export selected nodes to a file
    pub fn export_selected_to_file(
        &self,
        start_ids: &[String],
        path: &Path,
    ) -> std::io::Result<usize> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let nodes = self.export_selected(start_ids);
        let mut file = File::create(path)?;

        for node in &nodes {
            let serialized = serde_json::to_string(node)?;
            file.write_all(serialized.as_bytes())?;
            file.write_all(b"\n")?;
        }

        Ok(nodes.len())
    }

    /// Find differences between this DAG and another DAG from a file
    pub fn diff_with_file(&self, other_path: &Path) -> std::io::Result<DagDiff> {
        let other_ledger = Self::load_from_file(other_path)?;
        Ok(self.diff_with(&other_ledger))
    }

    /// Find differences between this DAG and another DAG
    pub fn diff_with(&self, other: &DagLedger) -> DagDiff {
        let this_nodes = &self.nodes;
        let other_nodes = &other.nodes;

        // Build HashSets of node IDs for more efficient lookup
        let this_ids: HashSet<String> = this_nodes.iter().map(|node| node.id.clone()).collect();
        let other_ids: HashSet<String> = other_nodes.iter().map(|node| node.id.clone()).collect();

        // Find nodes in this DAG but not in other
        let added: Vec<DagNode> = this_nodes
            .iter()
            .filter(|node| !other_ids.contains(&node.id))
            .cloned()
            .collect();

        // Find nodes in other DAG but not in this
        let removed: Vec<DagNode> = other_nodes
            .iter()
            .filter(|node| !this_ids.contains(&node.id))
            .cloned()
            .collect();

        // Find nodes in both DAGs
        let common: Vec<String> = this_ids.intersection(&other_ids).cloned().collect();

        DagDiff {
            added,
            removed,
            common,
        }
    }

    /// Export a diff to a file (exports only the added nodes)
    pub fn export_diff_to_file(&self, diff: &DagDiff, path: &Path) -> std::io::Result<()> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;

        for node in &diff.added {
            let serialized = serde_json::to_string(node)?;
            file.write_all(serialized.as_bytes())?;
            file.write_all(b"\n")?;
        }

        Ok(())
    }

    /// Find all nodes related to a specific proposal
    pub fn find_proposal_related_nodes(&self, proposal_id: &str) -> Vec<DagNode> {
        self.nodes
            .iter()
            .filter(|node| match &node.data {
                NodeData::ProposalCreated {
                    proposal_id: id, ..
                } if id == proposal_id => true,
                NodeData::VoteCast {
                    proposal_id: id, ..
                } if id == proposal_id => true,
                NodeData::ProposalExecuted {
                    proposal_id: id, ..
                } if id == proposal_id => true,
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// Get a summary of nodes by type (counts)
    pub fn get_node_type_summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();

        for node in &self.nodes {
            let type_name = match &node.data {
                NodeData::ProposalCreated { .. } => "ProposalCreated",
                NodeData::VoteCast { .. } => "VoteCast",
                NodeData::ProposalExecuted { .. } => "ProposalExecuted",
                NodeData::TokenMinted { .. } => "TokenMinted",
            };

            *summary.entry(type_name.to_string()).or_insert(0) += 1;
        }

        summary
    }

    // New method to get a file path with namespace
    pub fn get_namespaced_file_path(&self, namespace: &str) -> Result<String, String> {
        if let Some(file_path) = &self.file_path {
            let path = Path::new(file_path);
            let file_stem = path
                .file_stem()
                .ok_or_else(|| "Invalid file path".to_string())?;
            let extension = path.extension().unwrap_or_else(|| std::ffi::OsStr::new(""));

            let mut new_file_name = file_stem.to_os_string();
            new_file_name.push("_");
            new_file_name.push(namespace);

            if !extension.is_empty() {
                new_file_name.push(".");
                new_file_name.push(extension);
            }

            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let new_path = parent.join(new_file_name);

            Ok(new_path.to_string_lossy().to_string())
        } else {
            Err("File path is not set".to_string())
        }
    }

    // New method to export only nodes from a specific namespace
    pub fn export_namespace_to_file(&self, namespace: &str) -> Result<(), String> {
        let namespaced_file_path = self.get_namespaced_file_path(namespace)?;
        let nodes = self.nodes_by_namespace(namespace);

        let json = serde_json::to_string_pretty(&nodes.iter().collect::<Vec<_>>())
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        fs::write(&namespaced_file_path, json)
            .map_err(|e| format!("Failed to write to {}: {}", namespaced_file_path, e))?;

        Ok(())
    }
}
