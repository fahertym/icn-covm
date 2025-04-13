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

/// Represents a proposal in the system
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct ProposalAttachment {
    pub id: String,
    pub proposal_id: String,
    pub name: String,
    pub content_type: String,
    pub url: String,
    pub size: i64,
}

/// Represents a vote on a proposal
#[derive(Debug, Clone)]
pub struct Vote {
    pub id: String,
    pub proposal_id: String,
    pub user_id: String,
    pub vote_type: String,
    pub created_at: String,
}

/// Represents a comment on a proposal
#[derive(Debug, Clone)]
pub struct Comment {
    pub id: String,
    pub proposal_id: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub reactions: Option<HashMap<String, i32>>,
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
        Err(StorageError::KeyNotFound { key: id.to_string() })
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
