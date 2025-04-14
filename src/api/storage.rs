use crate::storage::{
    Proposal, ProposalAttachment, Comment, Vote,
    Storage, StorageResult, StorageBackend, StorageExtensions, AsyncStorageExtensions, 
};
use crate::api::v1::models;
use std::sync::Arc;
use log::warn;

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
    
    // DSL macro methods
    async fn get_macro(&self, id: &str) -> StorageResult<crate::storage::MacroDefinition>;
    async fn save_macro(&self, macro_def: &crate::storage::MacroDefinition) -> StorageResult<()>;
    async fn delete_macro(&self, id: &str) -> StorageResult<()>;
    async fn list_macros(&self, page: usize, page_size: usize, sort_by: Option<&str>, category: Option<&str>) 
        -> StorageResult<crate::api::v1::models::MacroListResponse>;
}

// Implement the async trait for Arc<T> where T implements Storage
impl<T> AsyncStorage for Arc<tokio::sync::Mutex<T>> 
where T: StorageBackend + StorageExtensions + AsyncStorageExtensions + Send + Sync + 'static {
    async fn get_proposal(&self, id: &str) -> StorageResult<Proposal> {
        let namespace = "governance";
        let key = format!("proposals/{}", id);
        let guard = self.lock().await;
        guard.get_json(None, namespace, &key)
    }
    
    async fn save_proposal(&self, proposal: &Proposal) -> StorageResult<()> {
        let namespace = "governance";
        let key = format!("proposals/{}", proposal.id);
        let mut guard = self.lock().await;
        guard.set_json(None, namespace, &key, proposal)
    }
    
    async fn list_proposals(&self) -> StorageResult<Vec<Proposal>> {
        let namespace = "governance";
        let prefix = "proposals/";
        let guard = self.lock().await;
        let keys = guard.list_keys(None, namespace, Some(prefix))?;
        
        // Filter out keys that have subpaths
        let proposal_keys = keys.into_iter()
            .filter(|k| !k.contains("/votes/") && !k.contains("/comments/") && 
                   !k.contains("/attachments/") && !k.contains("/execution_"))
            .collect::<Vec<_>>();
        
        let mut proposals = Vec::new();
        for key in &proposal_keys {
            if let Ok(proposal) = guard.get_json::<Proposal>(None, namespace, key) {
                proposals.push(proposal);
            }
        }
        
        Ok(proposals)
    }
    
    async fn save_proposal_attachment(&self, attachment: &ProposalAttachment) -> StorageResult<()> {
        let namespace = "governance";
        let key = format!("proposals/{}/attachments/{}", attachment.proposal_id, attachment.id);
        let mut guard = self.lock().await;
        guard.set_json(None, namespace, &key, attachment)
    }
    
    async fn get_proposal_attachments(&self, proposal_id: &str) -> StorageResult<Vec<ProposalAttachment>> {
        let namespace = "governance";
        let prefix = format!("proposals/{}/attachments/", proposal_id);
        let guard = self.lock().await;
        let keys = guard.list_keys(None, namespace, Some(&prefix))?;
        
        let mut attachments = Vec::new();
        for key in &keys {
            if let Ok(attachment) = guard.get_json::<ProposalAttachment>(None, namespace, key) {
                attachments.push(attachment);
            }
        }
        
        Ok(attachments)
    }
    
    async fn save_vote(&self, vote: &Vote) -> StorageResult<()> {
        let namespace = "governance";
        let key = format!("proposals/{}/votes/{}", vote.proposal_id, vote.id);
        let mut guard = self.lock().await;
        guard.set_json(None, namespace, &key, vote)
    }
    
    async fn get_proposal_votes(&self, proposal_id: &str) -> StorageResult<Vec<Vote>> {
        let namespace = "governance";
        let prefix = format!("proposals/{}/votes/", proposal_id);
        let guard = self.lock().await;
        let keys = guard.list_keys(None, namespace, Some(&prefix))?;
        
        let mut votes = Vec::new();
        for key in &keys {
            if let Ok(vote) = guard.get_json::<Vote>(None, namespace, key) {
                votes.push(vote);
            }
        }
        
        Ok(votes)
    }
    
    async fn save_comment(&self, comment: &Comment) -> StorageResult<()> {
        let namespace = "governance";
        let key = format!("proposals/{}/comments/{}", comment.proposal_id, comment.id);
        let mut guard = self.lock().await;
        guard.set_json(None, namespace, &key, comment)
    }
    
    async fn get_proposal_comments(&self, proposal_id: &str) -> StorageResult<Vec<Comment>> {
        let namespace = "governance";
        let prefix = format!("proposals/{}/comments/", proposal_id);
        let guard = self.lock().await;
        let keys = guard.list_keys(None, namespace, Some(&prefix))?;
        
        let mut comments = Vec::new();
        for key in &keys {
            if let Ok(comment) = guard.get_json::<Comment>(None, namespace, key) {
                comments.push(comment);
            }
        }
        
        Ok(comments)
    }
    
    async fn get_proposal_logic_path(&self, proposal_id: &str) -> StorageResult<String> {
        let guard = self.lock().await;
        guard.get_proposal_logic_path(proposal_id)
    }
    
    async fn get_proposal_logic(&self, logic_path: &str) -> StorageResult<String> {
        let guard = self.lock().await;
        guard.get_proposal_logic(logic_path)
    }
    
    async fn save_proposal_execution_result(&self, proposal_id: &str, result: &str) -> StorageResult<()> {
        let namespace = "governance";
        let key = format!("proposals/{}/execution_result", proposal_id);
        let mut guard = self.lock().await;
        guard.set(None, namespace, &key, result.as_bytes().to_vec())
    }
    
    async fn get_proposal_execution_result(&self, proposal_id: &str) -> StorageResult<String> {
        let namespace = "governance";
        let key = format!("proposals/{}/execution_result", proposal_id);
        let guard = self.lock().await;
        let bytes = guard.get(None, namespace, &key)?;
        String::from_utf8(bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Failed to parse execution result: {}", e),
            }
        })
    }
    
    async fn get_proposal_execution_logs(&self, proposal_id: &str) -> StorageResult<String> {
        let namespace = "governance";
        let key = format!("proposals/{}/execution_logs", proposal_id);
        let guard = self.lock().await;
        let bytes = guard.get(None, namespace, &key)?;
        String::from_utf8(bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Failed to parse execution logs: {}", e),
            }
        })
    }
    
    // DSL macro methods that delegate to AsyncStorageExtensions
    async fn get_macro(&self, id: &str) -> StorageResult<crate::storage::MacroDefinition> {
        let guard = self.lock().await;
        AsyncStorageExtensions::get_macro(&*guard, id).await
    }
    
    async fn save_macro(&self, macro_def: &crate::storage::MacroDefinition) -> StorageResult<()> {
        let mut guard = self.lock().await;
        AsyncStorageExtensions::save_macro(&mut *guard, macro_def).await
    }
    
    async fn delete_macro(&self, id: &str) -> StorageResult<()> {
        let mut guard = self.lock().await;
        AsyncStorageExtensions::delete_macro(&mut *guard, id).await
    }
    
    async fn list_macros(&self, page: usize, page_size: usize, sort_by: Option<&str>, category: Option<&str>) 
        -> StorageResult<crate::api::v1::models::MacroListResponse> {
        let guard = self.lock().await;
        AsyncStorageExtensions::list_macros(&*guard, page, page_size, sort_by, category).await
    }
} 