use crate::storage::utils::{now_with_default, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// Version info for stored data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: u64,
    pub created_by: String,
    pub timestamp: Timestamp,
    // Stores the history recursively. This could become large.
    // Consider storing only the previous version hash/ID in a real system.
    pub prev_version: Option<Box<VersionInfo>>,
}

impl VersionInfo {
    // Create the first version
    pub fn new(created_by: &str) -> Self {
        Self {
            version: 1,
            created_by: created_by.to_string(),
            timestamp: now_with_default(),
            prev_version: None,
        }
    }

    // Create the next version based on the current one
    pub fn next_version(&self, created_by: &str) -> Self {
        Self {
            version: self.version.saturating_add(1),
            created_by: created_by.to_string(),
            timestamp: now_with_default(),
            // Clone the current version info and box it as the previous one
            prev_version: Some(Box::new(self.clone())),
        }
    }

    // Get all versions in chronological order (oldest first)
    pub fn get_version_history(&self) -> Vec<&VersionInfo> {
        let mut history = Vec::new();
        let mut current = self;

        // Travel back through version history
        loop {
            history.push(current);

            if let Some(prev) = &current.prev_version {
                current = prev;
            } else {
                break;
            }
        }

        // Reverse to get oldest first
        history.reverse();
        history
    }

    // Get a specific version by number (1-indexed)
    pub fn get_version(&self, version_number: u64) -> Option<&VersionInfo> {
        if version_number == self.version {
            return Some(self);
        }

        if version_number > self.version {
            return None; // Future version
        }

        // Travel back through history
        let mut current = self;
        while let Some(prev) = &current.prev_version {
            current = prev;
            if current.version == version_number {
                return Some(current);
            }
        }

        None // Not found
    }
}

/// A structure to compare versions and generate differences
#[derive(Debug)]
pub struct VersionDiff<T> {
    pub old_version: u64,
    pub new_version: u64,
    pub created_by: String,
    pub timestamp: Timestamp,
    pub changes: Vec<DiffChange<T>>,
}

/// Represents a single change between versions
#[derive(Debug)]
pub enum DiffChange<T> {
    // For simple values
    ValueChanged {
        path: String,
        old_value: T,
        new_value: T,
    },
    // For more complex types like collections
    Added {
        path: String,
        value: T,
    },
    Removed {
        path: String,
        value: T,
    },
    // For hierarchical structures
    Modified {
        path: String,
        changes: Vec<DiffChange<T>>,
    },
}

/// Version store for managing multiple versions of data
pub struct VersionStore<T> {
    versions: VecDeque<(VersionInfo, T)>,
    max_versions: usize,
}

impl<T: Clone> VersionStore<T> {
    /// Create a new version store with a limit on stored versions
    pub fn new(max_versions: usize) -> Self {
        Self {
            versions: VecDeque::new(),
            max_versions,
        }
    }

    /// Add a new version
    pub fn add_version(&mut self, version_info: VersionInfo, data: T) {
        self.versions.push_back((version_info, data));

        // Trim old versions if exceeding max
        while self.versions.len() > self.max_versions {
            self.versions.pop_front();
        }
    }

    /// Get the current version
    pub fn get_current(&self) -> Option<&(VersionInfo, T)> {
        self.versions.back()
    }

    /// Get a specific version by number
    pub fn get_version(&self, version_number: u64) -> Option<&(VersionInfo, T)> {
        self.versions
            .iter()
            .find(|(info, _)| info.version == version_number)
    }

    /// List available versions
    pub fn list_versions(&self) -> Vec<&VersionInfo> {
        self.versions.iter().map(|(info, _)| info).collect()
    }
}
