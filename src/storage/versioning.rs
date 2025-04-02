use serde::{Serialize, Deserialize};
use crate::storage::utils::{Timestamp, now};

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
            timestamp: now(),
            prev_version: None,
        }
    }

    // Create the next version based on the current one
    pub fn next_version(&self, created_by: &str) -> Self {
        Self {
            version: self.version.saturating_add(1),
            created_by: created_by.to_string(),
            timestamp: now(),
            // Clone the current version info and box it as the previous one
            prev_version: Some(Box::new(self.clone())),
        }
    }
}
