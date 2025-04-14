pub mod auth;
pub mod errors;
pub mod events;
pub mod implementations;
pub mod namespaces;
pub mod resource;
pub mod traits;
pub mod utils;
pub mod versioning;

pub use auth::*;
pub use errors::*;
pub use events::*;
pub use namespaces::*;
pub use resource::*;
pub use traits::*;
pub use versioning::*;
// We might want to be more specific about what's exported from implementations
// For now, let's export the in-memory implementation directly
pub use implementations::in_memory::InMemoryStorage;
pub use utils::{now, Timestamp};

use std::collections::HashMap;
use serde_json::Value;

/// Represents a proposal in the system
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub votes_for: Option<i32>,
    pub votes_against: Option<i32>,
    pub votes_abstain: Option<i32>,
    pub attachments: Option<Vec<ProposalAttachment>>,
}

/// Represents an attachment to a proposal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProposalAttachment {
    pub id: String,
    pub proposal_id: String,
    pub name: String,
    pub content_type: String,
    pub url: String,
    pub size: i64,
}

/// Represents a vote on a proposal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Vote {
    pub id: String,
    pub proposal_id: String,
    pub user_id: String,
    pub vote_type: String,
    pub created_at: String,
}

/// Represents a comment on a proposal
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Comment {
    pub id: String,
    pub proposal_id: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub reactions: Option<HashMap<String, i32>>,
}

/// Represents a DSL macro definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MacroDefinition {
    /// Unique identifier for the macro
    pub id: String,
    /// Name of the macro
    pub name: String,
    /// The DSL code contents of the macro
    pub code: String,
    /// Optional description of the macro's purpose
    pub description: Option<String>,
    /// Creation timestamp in ISO 8601 format
    pub created_at: String,
    /// Last update timestamp in ISO 8601 format
    pub updated_at: String,
    /// Category for grouping macros (e.g., "economic", "governance")
    pub category: Option<String>,
    /// User who created the macro
    pub created_by: Option<String>,
    /// Additional metadata for the macro (including visual representation)
    pub metadata: Option<Value>,
}

// Add extension methods to InMemoryStorage for proposals
impl InMemoryStorage {
    pub async fn list_proposals(&self) -> StorageResult<Vec<Proposal>> {
        // In a real implementation, this would query the database
        // For this demo, we'll return an empty list
        Ok(Vec::new())
    }
    
    pub async fn get_proposal(&self, id: &str) -> StorageResult<Proposal> {
        // For demo purposes, return a dummy proposal
        // In a real implementation, this would query the database
        Err(StorageError::NotImplemented { 
            feature: format!("Getting proposal with id: {}", id)
        })
    }
    
    pub async fn save_proposal(&mut self, proposal: &Proposal) -> StorageResult<()> {
        // In a real implementation, this would save to the database
        Ok(())
    }
    
    pub async fn save_proposal_attachment(&mut self, attachment: &ProposalAttachment) -> StorageResult<()> {
        // In a real implementation, this would save to the database
        Ok(())
    }
    
    pub async fn get_proposal_comments(&self, proposal_id: &str) -> StorageResult<Vec<Comment>> {
        // In a real implementation, this would query the database
        Ok(Vec::new())
    }
    
    pub async fn save_comment(&mut self, comment: &Comment) -> StorageResult<()> {
        // In a real implementation, this would save to the database
        Ok(())
    }
    
    pub async fn save_vote(&mut self, vote: &Vote) -> StorageResult<()> {
        // In a real implementation, this would save to the database
        Ok(())
    }
}
