use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Management of hierarchical namespaces for cooperative storage
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NamespaceRegistry {
    // Map of namespace to its metadata
    namespaces: HashMap<String, NamespaceMetadata>,
}

/// Metadata for each namespace
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamespaceMetadata {
    /// Unique path for this namespace
    pub path: String,

    /// Owner of this namespace
    pub owner: String,

    /// Resource quota for this namespace (in bytes)
    pub quota_bytes: u64,

    /// Current resource usage (in bytes)
    pub used_bytes: u64,

    /// Optional parent namespace
    pub parent: Option<String>,

    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

impl NamespaceRegistry {
    /// Create a new namespace registry
    pub fn new() -> Self {
        Self {
            namespaces: HashMap::new(),
        }
    }

    /// Register a new namespace
    pub fn register_namespace(
        &mut self,
        path: &str,
        owner: &str,
        quota_bytes: u64,
        parent: Option<&str>,
    ) -> Result<(), String> {
        if self.namespaces.contains_key(path) {
            return Err(format!("Namespace {} already exists", path));
        }

        // Check if parent exists when specified
        if let Some(parent_path) = parent {
            if !self.namespaces.contains_key(parent_path) {
                return Err(format!("Parent namespace {} does not exist", parent_path));
            }
        }

        let metadata = NamespaceMetadata {
            path: path.to_string(),
            owner: owner.to_string(),
            quota_bytes,
            used_bytes: 0,
            parent: parent.map(|p| p.to_string()),
            attributes: HashMap::new(),
        };

        self.namespaces.insert(path.to_string(), metadata);
        Ok(())
    }

    /// Get metadata for a namespace
    pub fn get_namespace(&self, path: &str) -> Option<&NamespaceMetadata> {
        self.namespaces.get(path)
    }

    /// Check if a user has permission to access a namespace
    pub fn has_permission(&self, user: &str, action: &str, path: &str) -> bool {
        // Find the namespace or any parent
        match self.find_namespace_or_parent(path) {
            Some(metadata) => {
                // Owner has all permissions
                if metadata.owner == user {
                    return true;
                }

                // TODO: More sophisticated permission model based on roles
                // For now, just a simple check
                match action {
                    "read" => true,                    // Anyone can read (simplistic)
                    "write" => metadata.owner == user, // Only owner can write
                    _ => false,
                }
            }
            None => false,
        }
    }

    /// Track resource usage for a namespace
    pub fn update_resource_usage(&mut self, path: &str, bytes_delta: i64) -> Result<(), String> {
        match self.namespaces.get_mut(path) {
            Some(metadata) => {
                // Handle increase or decrease in usage
                if bytes_delta >= 0 {
                    let new_usage = metadata.used_bytes.saturating_add(bytes_delta as u64);

                    // Check if it exceeds quota
                    if new_usage > metadata.quota_bytes {
                        return Err(format!(
                            "Quota exceeded for namespace {}: {} of {} bytes",
                            path, new_usage, metadata.quota_bytes
                        ));
                    }

                    metadata.used_bytes = new_usage;
                } else {
                    metadata.used_bytes = metadata.used_bytes.saturating_sub((-bytes_delta) as u64);
                }

                Ok(())
            }
            None => Err(format!("Namespace {} does not exist", path)),
        }
    }

    /// List child namespaces
    pub fn list_children(&self, parent_path: &str) -> Vec<&NamespaceMetadata> {
        self.namespaces
            .values()
            .filter(|metadata| metadata.parent.as_ref().map_or(false, |p| p == parent_path))
            .collect()
    }

    /// Check if a namespace exists
    pub fn exists(&self, path: &str) -> bool {
        self.namespaces.contains_key(path)
    }

    /// Find a namespace or its closest parent that exists
    pub fn find_namespace_or_parent(&self, path: &str) -> Option<&NamespaceMetadata> {
        let mut current_path = path.to_string();

        loop {
            // Check if current path exists
            if let Some(metadata) = self.namespaces.get(&current_path) {
                return Some(metadata);
            }

            // Try parent path
            let parent_path = parent_namespace(&current_path);
            if parent_path == current_path {
                // No more parents (root level)
                return None;
            }

            current_path = parent_path;
        }
    }
}

/// Extract the parent namespace from a path
fn parent_namespace(path: &str) -> String {
    match path.rfind('/') {
        Some(idx) if idx > 0 => path[..idx].to_string(),
        _ => "".to_string(), // Root namespace
    }
}
