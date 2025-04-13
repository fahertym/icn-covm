use crate::storage::{
    Proposal, ProposalAttachment, Comment, Vote,
    Storage, StorageResult, StorageBackend, StorageExtensions
};
use std::sync::Arc;

/// Extension trait to add async proposal methods for use with Warp
pub trait AsyncStorage {
    async fn get_proposal(&self, id: &str) -> StorageResult<Proposal>;
    async fn save_proposal(&self, proposal: &Proposal) -> StorageResult<()>;
    async fn list_proposals(&self) -> StorageResult<Vec<Proposal>>;
    async fn save_proposal_attachment(&self, attachment: &ProposalAttachment) -> StorageResult<()>;
    async fn get_proposal_attachments(&self, proposal_id: &str) -> StorageResult<Vec<ProposalAttachment>>;
    async fn save_vote(&self, vote: &Vote) -> StorageResult<()>;
    async fn get_proposal_votes(&self, proposal_id: &str) -> StorageResult<Vec<Vote>>;
    async fn save_comment(&self, comment: &Comment) -> StorageResult<()>;
    async fn get_proposal_comments(&self, proposal_id: &str) -> StorageResult<Vec<Comment>>;
    async fn get_proposal_logic_path(&self, proposal_id: &str) -> StorageResult<String>;
    async fn get_proposal_logic(&self, logic_path: &str) -> StorageResult<String>;
    async fn save_proposal_execution_result(&self, proposal_id: &str, result: &str) -> StorageResult<()>;
    async fn get_proposal_execution_result(&self, proposal_id: &str) -> StorageResult<String>;
    async fn get_proposal_execution_logs(&self, proposal_id: &str) -> StorageResult<String>;
}

// Implement the async trait for Arc<Storage> (which is what we're using in Warp handlers)
impl AsyncStorage for Arc<Storage> {
    async fn get_proposal(&self, id: &str) -> StorageResult<Proposal> {
        let namespace = "governance";
        let key = format!("proposals/{}", id);
        self.get_json(None, namespace, &key)
    }
    
    async fn save_proposal(&self, proposal: &Proposal) -> StorageResult<()> {
        // TODO: This requires mutable access - either handle this differently
        // or consider using an interior mutable pattern like RwLock
        let namespace = "governance";
        let key = format!("proposals/{}", proposal.id);
        self.get_json(None, namespace, &key)
    }
    
    async fn list_proposals(&self) -> StorageResult<Vec<Proposal>> {
        let namespace = "governance";
        let prefix = "proposals/";
        let keys = self.list_keys(None, namespace, Some(prefix))?;
        
        // Filter out keys that have subpaths
        let proposal_keys = keys.into_iter()
            .filter(|k| !k.contains("/votes/") && !k.contains("/comments/") && 
                   !k.contains("/attachments/") && !k.contains("/execution_"))
            .collect::<Vec<_>>();
        
        let mut proposals = Vec::new();
        for key in proposal_keys {
            match self.get_json::<Proposal>(None, namespace, &key) {
                Ok(proposal) => proposals.push(proposal),
                Err(_) => continue, // Skip any keys that don't deserialize to proposals
            }
        }
        
        Ok(proposals)
    }
    
    async fn save_proposal_attachment(&self, attachment: &ProposalAttachment) -> StorageResult<()> {
        let namespace = "governance";
        let key = format!("proposals/{}/attachments/{}", attachment.proposal_id, attachment.id);
        // TODO: Handle mutable access issue
        Ok(())
    }
    
    async fn get_proposal_attachments(&self, proposal_id: &str) -> StorageResult<Vec<ProposalAttachment>> {
        let namespace = "governance";
        let prefix = format!("proposals/{}/attachments/", proposal_id);
        let keys = self.list_keys(None, namespace, Some(&prefix))?;
        
        let mut attachments = Vec::new();
        for key in keys {
            let attachment = self.get_json::<ProposalAttachment>(None, namespace, &key)?;
            attachments.push(attachment);
        }
        
        Ok(attachments)
    }
    
    async fn save_vote(&self, vote: &Vote) -> StorageResult<()> {
        // TODO: Handle mutable access issue
        Ok(())
    }
    
    async fn get_proposal_votes(&self, proposal_id: &str) -> StorageResult<Vec<Vote>> {
        let namespace = "governance";
        let prefix = format!("proposals/{}/votes/", proposal_id);
        let keys = self.list_keys(None, namespace, Some(&prefix))?;
        
        let mut votes = Vec::new();
        for key in keys {
            let vote = self.get_json::<Vote>(None, namespace, &key)?;
            votes.push(vote);
        }
        
        Ok(votes)
    }
    
    async fn save_comment(&self, comment: &Comment) -> StorageResult<()> {
        // TODO: Handle mutable access issue
        Ok(())
    }
    
    async fn get_proposal_comments(&self, proposal_id: &str) -> StorageResult<Vec<Comment>> {
        let namespace = "governance";
        let prefix = format!("proposals/{}/comments/", proposal_id);
        let keys = self.list_keys(None, namespace, Some(&prefix))?;
        
        let mut comments = Vec::new();
        for key in keys {
            let comment = self.get_json::<Comment>(None, namespace, &key)?;
            comments.push(comment);
        }
        
        Ok(comments)
    }
    
    async fn get_proposal_logic_path(&self, proposal_id: &str) -> StorageResult<String> {
        // Get the logic path from the proposal metadata
        let namespace = "governance";
        let meta_key = format!("proposals/{}/metadata", proposal_id);
        let bytes = self.get(None, namespace, &meta_key)?;
        
        // Parse metadata to extract logic_path
        let metadata: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Failed to parse proposal metadata: {}", e),
            }
        })?;
        
        // Extract logic path from metadata
        match metadata.get("logic_path") {
            Some(path) => match path.as_str() {
                Some(path_str) => Ok(path_str.to_string()),
                None => Err(crate::storage::errors::StorageError::InvalidValue {
                    details: "logic_path is not a string".to_string(),
                }),
            },
            None => Err(crate::storage::errors::StorageError::KeyNotFound {
                key: "logic_path".to_string(),
            }),
        }
    }
    
    async fn get_proposal_logic(&self, logic_path: &str) -> StorageResult<String> {
        // Get the DSL code from the specified path
        let namespace = "governance";
        let bytes = self.get(None, namespace, logic_path)?;
        
        // Convert bytes to string
        String::from_utf8(bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Invalid UTF-8 in DSL code: {}", e),
            }
        })
    }
    
    async fn save_proposal_execution_result(&self, proposal_id: &str, result: &str) -> StorageResult<()> {
        // TODO: Handle mutable access issue
        Ok(())
    }
    
    async fn get_proposal_execution_result(&self, proposal_id: &str) -> StorageResult<String> {
        // Create the key for retrieving execution result
        let namespace = "governance";
        let result_key = format!("proposals/{}/execution_result", proposal_id);
        
        // Try to get execution result, return error if not found
        match self.get(None, namespace, &result_key) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
                crate::storage::errors::StorageError::SerializationError {
                    details: format!("Invalid UTF-8 in execution result: {}", e),
                }
            }),
            Err(e) => Err(e),
        }
    }
    
    async fn get_proposal_execution_logs(&self, proposal_id: &str) -> StorageResult<String> {
        // Create the key for retrieving execution logs
        let namespace = "governance";
        let logs_key = format!("proposals/{}/execution_logs", proposal_id);
        
        // Try to get logs, return empty string if not found
        match self.get(None, namespace, &logs_key) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
                crate::storage::errors::StorageError::SerializationError {
                    details: format!("Invalid UTF-8 in execution logs: {}", e),
                }
            }),
            Err(crate::storage::errors::StorageError::KeyNotFound { .. }) => Ok(String::new()),
            Err(e) => Err(e),
        }
    }
} 