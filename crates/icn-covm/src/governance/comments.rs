use crate::storage::auth::AuthContext;
use crate::storage::traits::{Storage, StorageExtensions};
use crate::vm::VM;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use uuid::Uuid;

/// Type alias for comment identifiers, represented as strings
pub type CommentId = String;

/// Represents a comment version with its content and timestamp
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentVersion {
    /// Content of this version of the comment
    pub content: String,
    /// When this version was created
    pub timestamp: DateTime<Utc>,
}

/// Represents a comment on a proposal
///
/// Comments can be hierarchical, with the `reply_to` field pointing to the parent comment
/// if this comment is a reply. Top-level comments have `reply_to` set to `None`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposalComment {
    /// Unique identifier for the comment
    pub id: CommentId,
    /// Author of the comment, represented by their DID
    pub author: String,
    /// When the comment was created
    pub timestamp: DateTime<Utc>,
    /// Current content of the comment
    pub content: String,
    /// Optional reference to parent comment if this is a reply
    pub reply_to: Option<CommentId>,
    /// Tags associated with this comment (e.g., #finance, #technical)
    pub tags: Vec<String>,
    /// Reactions to this comment, mapping emoji to count
    pub reactions: HashMap<String, u32>,
    /// Whether this comment is hidden (soft deleted)
    pub hidden: bool,
    /// History of versions of this comment
    pub edit_history: Vec<CommentVersion>,
}

impl ProposalComment {
    /// Create a new comment with initial content
    pub fn new(
        author: String,
        content: String,
        reply_to: Option<CommentId>,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Self {
            id,
            author,
            timestamp: now.clone(),
            content: content.clone(),
            reply_to,
            tags,
            reactions: HashMap::new(),
            hidden: false,
            edit_history: vec![CommentVersion {
                content: content.clone(),
                timestamp: now,
            }],
        }
    }

    /// Add a new version of the comment content
    pub fn add_version(&mut self, content: String) {
        let now = Utc::now();
        // Update the current content
        self.content = content.clone();
        // Add to history
        self.edit_history.push(CommentVersion {
            content,
            timestamp: now,
        });
    }

    /// Hide the comment (soft deletion)
    pub fn hide(&mut self) {
        self.hidden = true;
    }

    /// Show the comment (undo soft deletion)
    pub fn unhide(&mut self) {
        self.hidden = false;
    }

    /// Get all comment versions
    pub fn get_versions(&self) -> &[CommentVersion] {
        &self.edit_history
    }

    /// Add a reaction to the comment
    pub fn add_reaction(&mut self, reaction: &str) {
        *self.reactions.entry(reaction.to_string()).or_insert(0) += 1;
    }

    /// Add tags to the comment
    pub fn add_tags(&mut self, new_tags: &[String]) {
        for tag in new_tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }
    }
}

/// Fetch all comments for a proposal, organized in a thread structure
pub fn fetch_comments_threaded<S>(
    vm: &VM<S>,
    proposal_id: &str,
    auth: Option<&AuthContext>,
    show_hidden: bool,
) -> Result<HashMap<String, ProposalComment>, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Check that the proposal exists
    let proposal_path = format!("governance/proposals/{}", proposal_id);

    // Ensure the proposal exists before fetching comments
    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| format!("Storage backend not available"))?;
    let _ = storage
        .get(auth, "governance", &proposal_path)
        .map_err(|_| format!("Proposal {} does not exist", proposal_id))?;

    // Fetch all comments stored under governance/proposals/{proposal_id}/comments/
    let comment_path = format!("governance/proposals/{}/comments", proposal_id);
    let comments_refs = storage.list_keys(auth, "governance", Some(&comment_path))?;

    let mut comments = HashMap::new();

    for comment_ref in comments_refs {
        match storage.get_json::<ProposalComment>(auth, "governance", &comment_ref) {
            Ok(comment) => {
                // Only include non-hidden comments unless show_hidden is true
                if !comment.hidden || show_hidden {
                    comments.insert(comment.id.clone(), comment);
                }
            }
            Err(_) => continue, // Skip invalid comments
        }
    }

    Ok(comments)
}

/// Create a new comment on a proposal
pub fn create_comment<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    author: &str,
    content: &str,
    reply_to: Option<&str>,
    tags: Vec<String>,
    auth_context: &AuthContext,
) -> Result<ProposalComment, Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Check that the proposal exists
    let proposal_path = format!("governance/proposals/{}", proposal_id);

    // Ensure the proposal exists
    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not available")?;
    let _ = storage
        .get(Some(auth_context), "governance", &proposal_path)
        .map_err(|_| format!("Proposal {} does not exist", proposal_id))?;

    // Create the comment
    let comment = ProposalComment::new(
        author.to_string(),
        content.to_string(),
        reply_to.map(|r| r.to_string()),
        tags,
    );

    // Store the comment
    let comment_path = format!(
        "governance/proposals/{}/comments/{}",
        proposal_id, comment.id
    );

    // Clone the storage to get a mutable version
    let mut storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not available")?
        .clone();
    storage.set_json(Some(auth_context), "governance", &comment_path, &comment)?;

    Ok(comment)
}

/// Get a single comment by ID
pub fn get_comment<S>(
    vm: &VM<S>,
    proposal_id: &str,
    comment_id: &str,
    auth_context: Option<&AuthContext>,
) -> Result<ProposalComment, Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let comment_path = format!(
        "governance/proposals/{}/comments/{}",
        proposal_id, comment_id
    );

    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not available")?;
    let comment_data = storage.get(auth_context, "governance", &comment_path)?;

    // Try to deserialize as the new format
    match serde_json::from_slice::<ProposalComment>(&comment_data) {
        Ok(comment) => {
            // Successfully deserialized as new format
            Ok(comment)
        }
        Err(_) => {
            // Might be in old format without hidden and edit_history fields
            // Try to deserialize as a simplified format
            #[derive(Debug, Serialize, Deserialize)]
            struct LegacyComment {
                id: String,
                author: String,
                timestamp: DateTime<Utc>,
                content: String,
                reply_to: Option<String>,
                tags: Vec<String>,
                reactions: HashMap<String, u32>,
            }

            // Try to deserialize as legacy format
            let legacy_comment = serde_json::from_slice::<LegacyComment>(&comment_data)?;

            // Convert to new format
            let now = Utc::now();
            let migrated_comment = ProposalComment {
                id: legacy_comment.id,
                author: legacy_comment.author,
                timestamp: legacy_comment.timestamp,
                content: legacy_comment.content.clone(),
                reply_to: legacy_comment.reply_to,
                tags: legacy_comment.tags,
                reactions: legacy_comment.reactions,
                hidden: false, // Default: not hidden
                edit_history: vec![CommentVersion {
                    content: legacy_comment.content,
                    timestamp: legacy_comment.timestamp, // Use original timestamp
                }],
            };

            // Save the migrated comment back to storage with the new format
            // This is a read-only operation, so we'll need to clone the VM and get a mutable reference
            if let Some(mut vm_clone) = vm.try_clone() {
                if let Some(mut storage_mut) = vm_clone.get_storage_backend().cloned() {
                    let _ = storage_mut.set_json(
                        auth_context,
                        "governance",
                        &comment_path,
                        &migrated_comment,
                    );
                }
            }

            Ok(migrated_comment)
        }
    }
}

/// Edit an existing comment, creating a new version
pub fn edit_comment<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    comment_id: &str,
    new_content: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get the comment
    let comment_path = format!(
        "governance/proposals/{}/comments/{}",
        proposal_id, comment_id
    );

    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not available")?;
    let mut comment =
        storage.get_json::<ProposalComment>(Some(auth_context), "governance", &comment_path)?;

    // Verify the author is the same as the current user
    if comment.author != auth_context.current_identity_did {
        return Err(format!("Only the original author can edit a comment").into());
    }

    // Add the new version
    comment.add_version(new_content.to_string());

    // Save the updated comment
    let mut storage_mut = storage.clone();
    storage_mut.set_json(Some(auth_context), "governance", &comment_path, &comment)?;

    // Also save the version history
    let version_id = comment.edit_history.len() - 1;
    let version_path = format!(
        "governance/proposals/{}/comments/versions/{}/{}",
        proposal_id, comment_id, version_id
    );

    storage_mut.set_json(
        Some(auth_context),
        "governance",
        &version_path,
        &comment.edit_history.last()
            .ok_or_else(|| "Comment edit history is empty".to_string())?,
    )?;

    Ok(())
}

/// Hide a comment (soft deletion)
pub fn hide_comment<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    comment_id: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get the comment
    let comment_path = format!(
        "governance/proposals/{}/comments/{}",
        proposal_id, comment_id
    );

    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not available")?;
    let mut comment =
        storage.get_json::<ProposalComment>(Some(auth_context), "governance", &comment_path)?;

    // Verify the author is the same as the current user
    if comment.author != auth_context.current_identity_did {
        return Err(format!("Only the original author can hide a comment").into());
    }

    // Hide the comment
    comment.hide();

    // Save the updated comment
    let mut storage_mut = storage.clone();
    storage_mut.set_json(Some(auth_context), "governance", &comment_path, &comment)?;

    Ok(())
}

/// Get the version history of a comment
pub fn get_comment_history<S>(
    vm: &VM<S>,
    proposal_id: &str,
    comment_id: &str,
    auth_context: Option<&AuthContext>,
) -> Result<Vec<CommentVersion>, Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get the comment
    let comment_path = format!(
        "governance/proposals/{}/comments/{}",
        proposal_id, comment_id
    );

    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not available")?;
    let comment = storage.get_json::<ProposalComment>(auth_context, "governance", &comment_path)?;

    Ok(comment.edit_history.clone())
}
