//! Proposal management CLI functionality for governance operations.
//!
//! This module provides the command-line interface for creating, viewing, editing,
//! and managing proposals within the governance system. It handles the lifecycle
//! of proposals from creation through deliberation, voting, and execution.
//!
//! The module includes functionality for:
//! - Creating new proposals
//! - Attaching files to proposals
//! - Adding and viewing threaded comments
//! - Transitioning proposals through various states
//! - Voting on proposals
//! - Executing proposal logic
//! - Listing and filtering proposals

use crate::compiler::parse_dsl;
use crate::compiler::parse_dsl::LifecycleConfig;
use crate::governance::comments::{self as comments};
use crate::governance::proposal::{
    Proposal, ProposalStatus, ProposalStatus as LocalProposalStatus,
};
use crate::governance::proposal_lifecycle::ExecutionStatus;
use crate::governance::proposal_lifecycle::VoteChoice;
use crate::governance::proposal_lifecycle::{Comment, ProposalLifecycle, ProposalState};
use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::{Storage, StorageBackend, StorageExtensions};
use crate::vm::Op;
use crate::vm::VMError;
use crate::vm::VM;
use chrono::{DateTime, Duration, Utc};
use clap::{arg, value_parser, Arg, ArgAction, ArgMatches, Command, Subcommand};
use hex;
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::boxed::Box;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration as StdDuration;
use uuid;
use regex::Regex;

/// Extension trait that provides proposal storage operations for VM
///
/// This trait centralizes all proposal-related storage operations, eliminating
/// direct access to VM fields like storage_backend, auth_context, and namespace.
/// Operations that mutate storage use the fork/commit pattern to ensure atomicity.
///
/// Key features:
/// - Standardized key prefixes for proposal storage
/// - Proper fork/mutation patterns for all data-changing operations
/// - Accessor methods that avoid direct field access
/// - Type-safe state transitions and error handling
trait VMProposalExtensions<S: StorageExtensions + Clone + Debug> {
    /// Get the proposal lifecycle by ID
    fn get_proposal_lifecycle(
        &self,
        proposal_id: &str,
    ) -> Result<ProposalLifecycle, Box<dyn Error>>;

    /// Get the proposal metadata by ID
    fn get_proposal(&self, proposal_id: &str) -> Result<Proposal, Box<dyn Error>>;

    /// Create a proposal in storage
    fn create_proposal(
        &mut self,
        proposal: Proposal,
        lifecycle: ProposalLifecycle,
        description: &str,
        logic: &str,
    ) -> Result<(), Box<dyn Error>>;

    /// Update a proposal's state
    fn update_proposal_state(
        &mut self,
        proposal_id: &str,
        new_state: ProposalState,
    ) -> Result<(), Box<dyn Error>>;

    /// Cast a vote on a proposal
    fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter_id: &str,
        vote_value: &str,
        delegated_by: Option<&str>,
    ) -> Result<(), Box<dyn Error>>;

    /// Get all votes for a proposal
    fn get_proposal_votes(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn Error>>;

    /// Execute a proposal
    fn execute_proposal(&mut self, proposal_id: &str) -> Result<(), Box<dyn Error>>;

    /// Add a comment to a proposal
    fn add_proposal_comment(
        &mut self,
        proposal_id: &str,
        author: &str,
        content: &str,
        parent_id: Option<&str>,
    ) -> Result<String, Box<dyn Error>>;

    /// Get proposal key prefix (for standardized key naming)
    fn proposal_key_prefix(proposal_id: &str) -> String {
        format!("governance_proposals/{}", proposal_id)
    }

    /// Get proposal lifecycle key
    fn proposal_lifecycle_key(proposal_id: &str) -> String {
        format!("{}/lifecycle", Self::proposal_key_prefix(proposal_id))
    }

    /// Get proposal description key
    fn proposal_description_key(proposal_id: &str) -> String {
        format!("{}/description", Self::proposal_key_prefix(proposal_id))
    }

    /// Get proposal logic key
    fn proposal_logic_key(proposal_id: &str) -> String {
        format!("{}/logic", Self::proposal_key_prefix(proposal_id))
    }

    /// Get proposal votes prefix
    fn proposal_votes_prefix(proposal_id: &str) -> String {
        format!("{}/votes", Self::proposal_key_prefix(proposal_id))
    }

    /// Get proposal comments prefix
    fn proposal_comments_prefix(proposal_id: &str) -> String {
        format!("{}/comments", Self::proposal_key_prefix(proposal_id))
    }
}

/// Implement the VMProposalExtensions trait for VM
impl<S> VMProposalExtensions<S> for VM<S>
where
    S: StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    fn get_proposal_lifecycle(
        &self,
        proposal_id: &str,
    ) -> Result<ProposalLifecycle, Box<dyn Error>> {
        let storage = self.get_storage_backend().ok_or("Storage not available")?;
        let auth_context_opt = self.get_auth_context();
        let namespace = self.get_namespace().unwrap_or("default");

        let lifecycle_key = Self::proposal_lifecycle_key(proposal_id);
        storage
            .get_json(auth_context_opt, &namespace, &lifecycle_key)
            .map_err(|e| format!("Failed to get proposal lifecycle: {}", e).into())
    }

    fn get_proposal(&self, proposal_id: &str) -> Result<Proposal, Box<dyn Error>> {
        let storage = self.get_storage_backend().ok_or("Storage not available")?;
        let auth_context_opt = self.get_auth_context();
        let namespace = self.get_namespace().unwrap_or("default");

        let proposal_key = Self::proposal_key_prefix(proposal_id);
        storage
            .get_json(auth_context_opt, &namespace, &proposal_key)
            .map_err(|e| format!("Failed to get proposal: {}", e).into())
    }

    fn create_proposal(
        &mut self,
        proposal: Proposal,
        lifecycle: ProposalLifecycle,
        description: &str,
        logic: &str,
    ) -> Result<(), Box<dyn Error>> {
        let proposal_id = proposal.id.clone();
        let title = lifecycle.title.clone();
        let mut forked = self.fork()?;
        let mut storage = forked
            .get_storage_backend()
            .ok_or("Storage not available")?
            .clone();
        let auth_context_opt = forked.get_auth_context();
        let namespace = forked.get_namespace().unwrap_or("default");

        // Store the proposal metadata
        let proposal_key = Self::proposal_key_prefix(&proposal_id);
        storage
            .set_json(auth_context_opt, &namespace, &proposal_key, &proposal)
            .map_err(|e| format!("Failed to store proposal: {}", e))?;

        // Store lifecycle data
        let lifecycle_key = Self::proposal_lifecycle_key(&proposal_id);
        storage
            .set_json(auth_context_opt, &namespace, &lifecycle_key, &lifecycle)
            .map_err(|e| format!("Failed to store proposal lifecycle: {}", e))?;

        // Store description
        let description_key = Self::proposal_description_key(&proposal_id);
        storage
            .set(
                auth_context_opt,
                &namespace,
                &description_key,
                description.as_bytes().to_vec(),
            )
            .map_err(|e| format!("Failed to store proposal description: {}", e))?;

        // Store logic
        let logic_key = Self::proposal_logic_key(&proposal_id);
        storage
            .set(
                auth_context_opt,
                &namespace,
                &logic_key,
                logic.as_bytes().to_vec(),
            )
            .map_err(|e| format!("Failed to store proposal logic: {}", e))?;

        // Commit the transaction
        self.commit_fork_transaction()?;

        // Log to DAG if available
        if let Some(ledger) = &mut self.dag {
            let node = icn_ledger::DagNode {
                id: String::new(), // Will be computed by the ledger
                parent_ids: vec![],
                timestamp: chrono::Utc::now().timestamp() as u64,
                data: icn_ledger::NodeData::ProposalCreated {
                    proposal_id: proposal_id.clone(),
                    title,
                },
            };
            let node_id = ledger.append(node);
            println!("🧾 DAG: Proposal {} recorded as node {}", proposal_id, node_id);
        }

        Ok(())
    }

    fn update_proposal_state(
        &mut self,
        proposal_id: &str,
        new_state: ProposalState,
    ) -> Result<(), Box<dyn Error>> {
        // Create a fork for the state update transaction
        let mut forked = self.fork()?;
        let mut storage = forked
            .get_storage_backend()
            .ok_or("Storage not available")?
            .clone();
        let auth_context_opt = forked.get_auth_context().cloned();
        let namespace = forked.get_namespace().unwrap_or("default");

        // Load the current proposal lifecycle
        let lifecycle_key = Self::proposal_lifecycle_key(proposal_id);
        let mut lifecycle = storage
            .get_json::<ProposalLifecycle>(auth_context_opt.as_ref(), &namespace, &lifecycle_key)
            .map_err(|e| format!("Failed to load proposal lifecycle: {}", e))?;

        // Update the state and add to history
        lifecycle.state = new_state.clone();
        lifecycle.history.push((chrono::Utc::now(), new_state));

        // Save the updated lifecycle
        storage
            .set_json(auth_context_opt.as_ref(), &namespace, &lifecycle_key, &lifecycle)
            .map_err(|e| format!("Failed to update proposal state: {}", e))?;

        // Commit the transaction
        self.commit_fork_transaction()?;

        Ok(())
    }

    fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter_id: &str,
        vote_value: &str,
        delegated_by: Option<&str>,
    ) -> Result<(), Box<dyn Error>> {
        // Create a fork for the vote transaction
        let mut forked = self.fork()?;
        let mut storage = forked
            .get_storage_backend()
            .ok_or("Storage not available")?
            .clone();
        let auth_context_opt = forked.get_auth_context();
        let namespace = forked.get_namespace().unwrap_or("default");

        // Check if proposal exists
        let proposal_key = Self::proposal_key_prefix(proposal_id);
        let exists = storage.contains(auth_context_opt, &namespace, &proposal_key)?;
        if !exists {
            return Err(format!("Proposal with ID '{}' not found", proposal_id).into());
        }

        // Create the vote data structure
        let vote_data = serde_json::json!({
            "voter": voter_id,
            "vote": vote_value,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "delegated_by": delegated_by,
        });

        // Create the vote key
        let vote_key = format!("{}/{}", Self::proposal_votes_prefix(proposal_id), voter_id);

        // Store the vote
        storage
            .set_json(auth_context_opt, &namespace, &vote_key, &vote_data)
            .map_err(|e| format!("Failed to store vote: {}", e))?;

        // Commit the transaction
        self.commit_fork_transaction()?;

        // Log to DAG if available
        if let Some(ledger) = &mut self.dag {
            // Convert vote value to a numeric value for the DAG
            let vote_numeric = match vote_value.to_lowercase().as_str() {
                "yes" => 1.0,
                "no" => 0.0,
                "abstain" => 0.5,
                _ => -1.0, // Invalid vote
            };

            // Find the proposal node to create the parent reference
            let parent_ids = ledger.find_proposal_node_id(proposal_id)
                .map(|id| vec![id])
                .unwrap_or_default();

            let node = icn_ledger::DagNode {
                id: String::new(), // Will be computed by the ledger
                parent_ids,
                timestamp: chrono::Utc::now().timestamp() as u64,
                data: icn_ledger::NodeData::VoteCast {
                    proposal_id: proposal_id.to_string(),
                    voter: voter_id.to_string(),
                    vote: vote_numeric,
                },
            };
            let node_id = ledger.append(node);
            println!("🗳️ DAG: Vote recorded as node {}", node_id);
        }

        Ok(())
    }

    fn get_proposal_votes(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let storage = self.get_storage_backend().ok_or("Storage not available")?;
        let auth_context_opt = self.get_auth_context();
        let namespace = self.get_namespace().unwrap_or("default");

        // Define the votes prefix
        let votes_prefix = Self::proposal_votes_prefix(proposal_id);

        // Get all vote keys for this proposal
        let vote_keys = storage.list_keys(auth_context_opt, &namespace, Some(&votes_prefix))?;

        // Load each vote
        let mut votes = Vec::new();
        for key in vote_keys {
            // Get the vote data
            let vote_data: serde_json::Value =
                storage.get_json(auth_context_opt, &namespace, &key)?;

            // Extract the vote value, defaulting to "abstain" if not found
            let vote_value = vote_data
                .get("vote")
                .and_then(|v| v.as_str())
                .unwrap_or("abstain")
                .to_string();

            // Extract the voter ID from the key
            let voter_id = key.split('/').last().unwrap_or("unknown").to_string();

            // Add to our results
            votes.push((voter_id, vote_value));
        }

        Ok(votes)
    }

    fn execute_proposal(&mut self, proposal_id: &str) -> Result<(), Box<dyn Error>> {
        // Create a fork for mutations
        let mut forked = self.fork()?;
        
        // Get and capture the auth context and namespace
        let maybe_auth_context = forked.get_auth_context().cloned();
        let namespace = forked.get_namespace().unwrap_or("default").to_string();
        
        // Get mutable storage
        let mut storage = forked
            .get_storage_backend()
            .ok_or("Storage not available")?
            .clone();

        // Load the proposal lifecycle
        let lifecycle_key = Self::proposal_lifecycle_key(proposal_id);
        let mut proposal_lifecycle: ProposalLifecycle = storage
            .get_json(maybe_auth_context.as_ref(), &namespace, &lifecycle_key)
            .map_err(|e| format!("Failed to load proposal lifecycle: {}", e))?;

        // Check if proposal has already been executed
        if matches!(proposal_lifecycle.state, ProposalState::Executed) {
            return Err(format!("Proposal '{}' has already been executed", proposal_id).into());
        }

        // Load the logic content
        let logic_key = Self::proposal_logic_key(proposal_id);
        let logic: Result<Vec<u8>, _> = storage.get(maybe_auth_context.as_ref(), &namespace, &logic_key);
        
        let success = if let Ok(logic_content) = logic {
            // Process the logic
            if let Ok(logic_str) = String::from_utf8(logic_content) {
                // Parse the DSL content
                let (ops, _) = crate::compiler::parse_dsl(&logic_str)?;
                
                // Execute the operations
                if let Err(e) = forked.execute(&ops) {
                    println!("Logic execution failed: {}", e);
                    false
                } else {
                    true
                }
            } else {
                println!("Logic content is not valid UTF-8");
                false
            }
        } else {
            println!("No logic found for proposal");
            false
        };
        
        // Update the proposal state
        proposal_lifecycle.state = ProposalState::Executed;
        proposal_lifecycle.history.push((Utc::now(), ProposalState::Executed));
        
        // Save updated lifecycle data
        storage
            .set_json(maybe_auth_context.as_ref(), &namespace, &lifecycle_key, &proposal_lifecycle)
            .map_err(|e| format!("Failed to update proposal lifecycle: {}", e))?;
        
        // Commit the transaction
        self.commit_fork_transaction()?;

        // Log to DAG if available
        if let Some(ledger) = &mut self.dag {
            // Collect parent IDs from vote nodes
            let vote_nodes = ledger.find_vote_nodes_for(proposal_id);
            let parent_ids: Vec<String> = vote_nodes.iter()
                .map(|node| node.id.clone())
                .collect();

            let node = icn_ledger::DagNode {
                id: String::new(), // Will be computed by the ledger
                parent_ids,
                timestamp: chrono::Utc::now().timestamp() as u64,
                data: icn_ledger::NodeData::ProposalExecuted {
                    proposal_id: proposal_id.to_string(),
                    success,
                },
            };
            let node_id = ledger.append(node);
            println!("⚙️ DAG: Execution recorded as node {}", node_id);
        }
        
        Ok(())
    }

    fn add_proposal_comment(
        &mut self,
        proposal_id: &str,
        author: &str,
        content: &str,
        parent_id: Option<&str>,
    ) -> Result<String, Box<dyn Error>> {
        // Create a fork for mutations
        let mut forked = self.fork()?;
        let mut storage = forked
            .get_storage_backend()
            .ok_or("Storage not available")?
            .clone();
        let auth_context = forked.get_auth_context();
        let namespace = forked.get_namespace().unwrap_or("default");

        // Check if proposal exists
        let proposal_key = Self::proposal_key_prefix(proposal_id);
        if !storage.contains(auth_context, &namespace, &proposal_key)? {
            return Err(format!("Proposal with ID '{}' not found", proposal_id).into());
        }

        // Generate a comment ID
        let comment_id = uuid::Uuid::new_v4().to_string();

        // Create the comment structure
        let comment = StoredComment {
            author: author.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            content: content.to_string(),
            parent: parent_id.map(|s| s.to_string()),
        };

        // Store the comment
        let comment_key = format!(
            "{}/{}",
            Self::proposal_comments_prefix(proposal_id),
            comment_id
        );
        storage.set_json(auth_context, &namespace, &comment_key, &comment)?;

        // Commit the changes
        self.commit_fork_transaction()?;

        Ok(comment_id)
    }
}

/// Type alias for proposal identifiers, represented as strings
pub type ProposalId = String;
/// Type alias for comment identifiers, represented as strings
type CommentId = String;

/// Default minimum time (in hours) required for the deliberation phase
const MIN_DELIBERATION_HOURS: i64 = 24;

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
    /// Text content of the comment
    pub content: String,
    /// Optional reference to parent comment if this is a reply
    pub reply_to: Option<CommentId>,
    /// Tags associated with this comment (e.g., #finance, #technical)
    pub tags: Vec<String>,
    /// Reactions to this comment, mapping emoji to count
    pub reactions: HashMap<String, u32>,
}

/// Creates the command-line interface for proposal management
///
/// Defines all subcommands and arguments for the proposal CLI, including:
/// - create: Create a new governance proposal
/// - attach: Attach a file to a proposal
/// - comment: Add a comment to a proposal
/// - edit: Edit an existing proposal
/// - publish: Move a proposal from Draft to OpenForFeedback state
/// - vote: Cast a vote on a proposal
/// - transition: Manually change a proposal's state
/// - view: View proposal details
/// - list: List and filter proposals
/// - comments: View all comments for a proposal
/// - comment-react: Add a reaction to a comment
/// - comment-tag: Add tags to an existing comment
/// - simulate: Simulate the execution of a proposal without making persistent changes
/// - summary: Get high-level summary of a proposal's activity and state
/// - execute: Execute the logic of a passed proposal
/// - view-comments: View all comments for a proposal
/// - export: Export a complete proposal and its lifecycle data to a JSON file
///
/// # Returns
/// A configured `Command` object ready to be used in a CLI application
pub fn proposal_command() -> Command {
    Command::new("proposal")
        .about("Governance proposal operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("dag-path")
                .long("dag-path")
                .value_name("PATH")
                .help("Path to the DAG ledger file for storing governance events")
                .global(true)
        )
        .subcommand(
            Command::new("create")
                .about("Create a new governance proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("ID")
                        .help("Unique identifier for the proposal")
                        .required(true),
                )
                .arg(
                    Arg::new("title")
                        .long("title")
                        .value_name("STRING")
                        .help("Title of the proposal")
                        .required(true),
                )
                .arg(
                    Arg::new("description")
                        .long("description")
                        .value_name("STRING")
                        .help("Description of the proposal")
                        .required(true),
                )
                .arg(
                    Arg::new("quorum")
                        .long("quorum")
                        .value_name("FLOAT")
                        .help("Quorum required for the proposal to pass (value between 0.0 and 1.0)")
                        .value_parser(value_parser!(f64))
                        .required(true),
                )
                .arg(
                    Arg::new("threshold")
                        .long("threshold")
                        .value_name("FLOAT")
                        .help("Threshold required for the proposal to pass (value between 0.0 and 1.0)")
                        .value_parser(value_parser!(f64))
                        .required(true),
                )
                .arg(
                    Arg::new("logic")
                        .long("logic")
                        .value_name("PATH")
                        .help("Path to the DSL logic file")
                        .required(true),
                )
                .arg(
                    Arg::new("expires-in")
                        .long("expires-in")
                        .value_name("DURATION")
                        .help("Duration until proposal expires (e.g., 7d, 24h, default: 30d)"),
                )
                .arg(
                    Arg::new("creator")
                        .long("creator")
                        .value_name("ID")
                        .help("Identity ID of the proposal creator"),
                )
                // Keep existing arguments for compatibility
                .arg(
                    Arg::new("logic-path")
                        .long("logic-path")
                        .value_name("PATH")
                        .help("Path to the proposal logic (deprecated, use --logic instead)"),
                )
                .arg(
                    Arg::new("discussion-path")
                        .long("discussion-path")
                        .value_name("PATH")
                        .help("Path to the proposal discussion thread"),
                )
                .arg(
                    Arg::new("attachments")
                        .long("attachments")
                        .value_name("ATTACHMENTS")
                        .help("Comma-separated list of attachment references"),
                )
                .arg(
                    Arg::new("min-deliberation")
                        .long("min-deliberation")
                        .value_name("HOURS")
                        .help("Minimum hours required for deliberation phase")
                        .value_parser(value_parser!(i64)),
                )
                .arg(
                    Arg::new("discussion-duration")
                        .long("discussion-duration")
                        .value_name("DURATION")
                        .help("Optional duration for the feedback/discussion phase (e.g., 7d, 48h)"),
                )
                .arg(
                    Arg::new("required-participants")
                        .long("required-participants")
                        .value_name("NUMBER")
                        .help("Minimum number of participants required for the proposal to be valid")
                        .value_parser(value_parser!(u64)),
                )
        )
        .subcommand(
            Command::new("attach")
                .about("Attach a file to a proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to attach the file to")
                        .required(true)
                        // No value_parser needed for String
                )
        .arg(
                    Arg::new("file")
                        .long("file")
                        .value_name("FILE_PATH")
                        .help("Path to the file to attach")
                .required(true)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    Arg::new("name")
                        .long("name")
                        .value_name("STRING")
                        .help("Optional name for the attachment (e.g., 'body', 'logic'). Defaults to filename stem.")
                        // Not required, handled in logic
                )
        )
        .subcommand(
            Command::new("comment")
                .about("Add a comment to a proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to comment on")
                        .required(true)
                )
                .arg(
                    Arg::new("content")
                        .long("content")
                        .value_name("TEXT")
                        .help("Text content of the comment")
                        .required(true)
                )
                .arg(
                    Arg::new("parent")
                        .long("parent")
                        .value_name("COMMENT_ID")
                        .help("ID of the parent comment (for threading)")
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .value_name("TAG")
                        .help("Add a tag to the comment (e.g., '#finance')")
                        .action(ArgAction::Append)
                )
        )
        .subcommand(
            Command::new("comment-react")
                .about("Add a reaction to a comment")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to react to")
                        .required(true)
                )
                .arg(
                    Arg::new("proposal-id")
                        .long("proposal-id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal containing the comment")
                        .required(true)
                )
                .arg(
                    Arg::new("reaction")
                        .long("reaction")
                        .value_name("EMOJI")
                        .help("Reaction emoji to add (e.g., '👍', '🔥')")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("comment-tag")
                .about("Add tags to an existing comment")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to tag")
                        .required(true)
                )
                .arg(
                    Arg::new("proposal-id")
                        .long("proposal-id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal containing the comment")
                        .required(true)
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .value_name("TAG")
                        .help("Tag to add (e.g., '#finance')")
                        .required(true)
                        .action(ArgAction::Append)
                )
        )
        .subcommand(
            Command::new("thread")
                .about("Display comments in a threaded view")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to show comments for")
                        .required(true)
                )
                .arg(
                    Arg::new("show-hidden")
                        .long("show-hidden")
                        .help("Include hidden comments in the output")
                        .action(ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("comment-edit")
                .about("Edit an existing comment")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to edit")
                        .required(true)
                )
                .arg(
                    Arg::new("proposal-id")
                        .long("proposal-id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal containing the comment")
                        .required(true)
                )
                .arg(
                    Arg::new("text")
                        .long("text")
                        .value_name("TEXT")
                        .help("New text content for the comment")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("comment-hide")
                .about("Hide a comment (soft deletion)")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to hide")
                        .required(true)
                )
                .arg(
                    Arg::new("proposal-id")
                        .long("proposal-id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal containing the comment")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("comment-history")
                .about("Show edit history of a comment")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to show history for")
                        .required(true)
                )
                .arg(
                    Arg::new("proposal-id")
                        .long("proposal-id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal containing the comment")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("edit")
                .about("Edit an existing proposal (e.g., update attachments)")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to edit")
                        .required(true)
                )
                .arg(
                    Arg::new("new-body")
                        .long("new-body")
                        .value_name("FILE_PATH")
                        .help("Path to the new proposal body file (e.g., updated markdown)")
                        .value_parser(value_parser!(PathBuf))
                )
                .arg(
                    Arg::new("new-logic")
                        .long("new-logic")
                        .value_name("FILE_PATH")
                        .help("Path to the new proposal logic file (e.g., updated CCL script)")
                        .value_parser(value_parser!(PathBuf))
                )
                // TODO: Add options for changing title, quorum, threshold? Depends on rules.
        )
        .subcommand(
            Command::new("publish")
                .about("Publish a proposal draft to make it open for feedback")
        .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to publish (move from Draft to OpenForFeedback)")
                .required(true)
                        // No value_parser needed for String
                )
        )
        .subcommand(
            Command::new("vote")
                .about("Cast a vote on an active proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to vote on")
                        .required(true)
                )
                .arg(
                    Arg::new("vote")
                        .long("vote")
                        .value_name("CHOICE")
                        .help("Your vote choice (yes, no, or abstain)")
                        .required(true)
                )
                .arg(
                    Arg::new("as")
                        .long("as")
                        .value_name("IDENTITY")
                        .help("Optional identity to vote as (for delegated voting)")
                )
        )
        .subcommand(
            Command::new("transition")
                .about("Transition proposal status")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to transition")
                        .required(true)
                )
                .arg(
                    Arg::new("state")
                        .long("state")
                        .value_name("STATE")
                        .help("New state: draft, feedback, deliberation, voting, executed, rejected, expired")
                        .required(true)
                )
                .arg(
                    Arg::new("result")
                        .long("result")
                        .value_name("RESULT")
                        .help("Optional result message for executed proposals")
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .help("Force status transition ignoring state transition rules")
                        .action(ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("view")
                .about("View detailed information about a proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to view")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("list")
                .about("List all proposals")
                .arg(
                    Arg::new("status")
                        .long("status")
                        .value_name("STATUS")
                        .help("Filter by status: draft, deliberation, active, voting, executed, rejected, expired")
                )
                .arg(
                    Arg::new("creator")
                        .long("creator")
                        .value_name("CREATOR_ID")
                        .help("Filter by creator ID")
                )
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .value_name("NUMBER")
                        .help("Limit number of proposals to display")
                        .value_parser(value_parser!(u32))
                )
        )
        .subcommand(
            Command::new("comments")
                .about("View threaded comments for a proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .required(true)
                        .help("Proposal ID to view comments for")
                )
                .arg(
                    Arg::new("sort")
                        .long("sort")
                        .value_name("SORT_BY")
                        .help("Sort comments by: time (default), author")
                )
        )
        .subcommand(
            Command::new("simulate")
                .about("Simulate execution of a proposal without actually executing it")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to simulate")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("dag-trace")
                .about("Trace a proposal's DAG path")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to trace")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("summary")
                .about("Get high-level summary of a proposal's activity and state")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to summarize")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("execute")
                .about("Execute the logic of a passed proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to execute")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("view-comments")
                .about("View all comments for a proposal")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to view comments for")
                        .required(true)
                )
                .arg(
                    Arg::new("threaded")
                        .long("threaded")
                        .action(ArgAction::SetTrue)
                        .help("Show comments in a threaded view with replies indented")
                )
        )
        .subcommand(
            Command::new("export")
                .about("Export a complete proposal and its lifecycle data to a JSON file")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to export")
                        .required(true)
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .value_name("FILE_PATH")
                        .help("File path for the exported JSON (default: proposal_<id>.json)")
                )
        )
}

/// Loads a proposal by ID from storage
pub fn load_proposal<S>(
    vm: &VM<S>,
    proposal_id: &ProposalId,
) -> Result<ProposalLifecycle, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Use the trait method to load the proposal lifecycle
    vm.get_proposal_lifecycle(proposal_id)
}

/// Converts a DID string to an Identity object
///
/// Creates a basic Identity with default values using the provided DID.
///
/// # Parameters
/// * `did` - The DID string to convert
///
/// # Returns
/// An Identity object with the given DID
fn did_to_identity(did: &str) -> Result<Identity, Box<dyn Error>> {
    // A simple DID to Identity converter for command-line use
    Identity::new(did.to_string(), None, "member".to_string(), None)
        .map_err(|e| format!("Failed to create identity from DID: {}", e).into())
}

/// Parse a DSL file from filesystem
fn parse_dsl_from_file<S>(
    vm: &mut VM<S>,
    path: &str,
) -> Result<(Vec<Op>, LifecycleConfig), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    println!("Parsing DSL from file: {}", path);

    // Read the file content
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read DSL file {}: {}", path, e))?;

    // Extract the lifecycle configuration if present
    let lifecycle_config = extract_lifecycle_config(&content)
        .map_err(|e| format!("Failed to extract lifecycle config: {}", e))?;

    // Parse the DSL content
    let (logic_ops, dsl_config) = parse_dsl(&content)
        .map_err(|e| format!("Failed to parse DSL content: {}", e))?;
        
    // Merge the configs if needed
    let mut final_config = lifecycle_config;
    if !dsl_config.required_roles.is_empty() ||
       dsl_config.quorum.is_some() ||
       dsl_config.threshold.is_some() ||
       dsl_config.min_deliberation.is_some() ||
       dsl_config.expires_in.is_some() {
        final_config.merge_from(&dsl_config);
    }

    Ok((logic_ops, final_config))
}

fn extract_lifecycle_config(content: &str) -> Result<LifecycleConfig, Box<dyn Error>> {
    let re = Regex::new(r"(?s)#\s*lifecycle\s*\{([^}]*)\}")
        .map_err(|e| format!("Regex error: {}", e))?;
    
    if let Some(caps) = re.captures(content) {
        if let Some(config_block) = caps.get(1) {
            let config_str = config_block.as_str().trim();
            let lines: Vec<&str> = config_str.split('\n').collect();
            
            let mut config = LifecycleConfig::default();
            
            for line in lines {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    
                    match key {
                        "quorum" => {
                            if let Ok(v) = value.parse::<f64>() {
                                config.quorum = Some(v);
                            } else {
                                return Err(format!("Invalid quorum value: {}", value).into());
                            }
                        }
                        "threshold" => {
                            if let Ok(v) = value.parse::<f64>() {
                                config.threshold = Some(v);
                            } else {
                                return Err(format!("Invalid threshold value: {}", value).into());
                            }
                        }
                        "min_deliberation" => {
                            if let Ok(v) = value.parse::<i64>() {
                                config.min_deliberation = Some(chrono::Duration::hours(v));
                            } else {
                                return Err(format!("Invalid min_deliberation value: {}", value).into());
                            }
                        }
                        "required_roles" => {
                            // Parse comma-separated list of roles
                            let roles: Vec<String> = value.split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                            config.required_roles = roles;
                        }
                        _ => {
                            println!("Warning: Unknown lifecycle config key: {}", key);
                        }
                    }
                }
            }
            
            return Ok(config);
        }
    }
    
    // If no lifecycle block found, return default config
    Ok(LifecycleConfig::default())
}

// Let's also fix the parse_duration_string function
fn parse_duration_string(duration_str: &str) -> Result<chrono::Duration, Box<dyn Error>> {
    let re = Regex::new(r"^(\d+)([dhm])$")
        .map_err(|e| format!("Regex error: {}", e))?;

    if let Some(caps) = re.captures(duration_str) {
        let amount = caps[1].parse::<i64>()
            .map_err(|_| format!("Invalid duration amount: {}", &caps[1]))?;
        
        match &caps[2] {
            "d" => Ok(chrono::Duration::days(amount)),
            "h" => Ok(chrono::Duration::hours(amount)),
            "m" => Ok(chrono::Duration::minutes(amount)),
            _ => Err(format!("Unknown duration unit: {}", &caps[2]).into()),
        }
    } else {
        Err(format!("Invalid duration format: {}. Expected format: <number><unit>, where unit is d (days), h (hours), or m (minutes)", duration_str).into())
    }
}

/// Main handler for proposal commands
///
/// Processes all proposal subcommands based on the CLI arguments.
/// This is the core implementation of the proposal management functionality.
///
/// # Parameters
/// * `vm` - The virtual machine with mutable access to storage and execution environment
/// * `matches` - The parsed command-line arguments
/// * `auth_context` - Authentication context for the current user
///
/// # Returns
/// * `Result<(), Box<dyn Error>>` - Success or an error
///
/// # Errors
/// Returns an error if any operation fails based on the specific subcommand.
pub fn handle_proposal_command<S>(
    vm: &mut VM<S>,
    matches: &ArgMatches,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let user_did = auth_context.identity_did(); // Get DID from auth_context parameter

    // Check for DAG path option
    if let Some(dag_path) = matches.get_one::<String>("dag-path") {
        vm.set_dag_path(PathBuf::from(dag_path));
        println!("📒 Using DAG ledger at: {}", dag_path);
    }

    match matches.subcommand() {
        Some(("create", sub_matches)) => {
            let proposal_id = sub_matches.get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            let title = sub_matches.get_one::<String>("title")
                .ok_or("Title is required")?;
            let description = sub_matches.get_one::<String>("description")
                .ok_or("Description is required")?;
            let quorum = *sub_matches.get_one::<f64>("quorum")
                .ok_or("Quorum is required")?;
            let threshold = *sub_matches.get_one::<f64>("threshold")
                .ok_or("Threshold is required")?;
            let logic_path = sub_matches
                .get_one::<String>("logic")
                .or_else(|| sub_matches.get_one::<String>("logic-path"))
                .ok_or_else(|| "No logic path provided")?;
            let discussion_path = sub_matches.get_one::<String>("discussion-path");
            let attachments = sub_matches.get_one::<String>("attachments");
            let expires_in = sub_matches.get_one::<String>("expires-in");
            let min_deliberation = sub_matches.get_one::<i64>("min-deliberation");
            let discussion_duration = sub_matches.get_one::<String>("discussion-duration");
            let required_participants = sub_matches.get_one::<u64>("required-participants");

            // Special case for creator identity
            let creator = sub_matches
                .get_one::<String>("creator")
                .map(|s| s.to_string())
                .unwrap_or_else(|| auth_context.identity_did().to_string());

            // Read and parse the DSL content
            let (logic_ops, lifecycle_config) = match parse_dsl_from_file(vm, logic_path) {
                Ok((ops, config)) => (ops, config),
                Err(e) => {
                    println!("❌ Failed to parse DSL file: {}", e);
                    return Err(format!("Failed to parse DSL file: {}", e).into());
                }
            };

            // Calculate expiry date
            let expires_at = if let Some(expires_str) = expires_in {
                match parse_duration_string(expires_str) {
                    Ok(duration) => Some(chrono::Utc::now() + duration),
                    Err(e) => {
                        println!("❌ Invalid expires-in format: {}", e);
                        return Err(e);
                    }
                }
            } else {
                // Default expiry of 30 days
                Some(chrono::Utc::now() + chrono::Duration::days(30))
            };

            // Calculate minimum deliberation period
            let min_delib_duration = if let Some(hours) = min_deliberation {
                chrono::Duration::hours(*hours)
            } else if let Some(dur_str) = discussion_duration {
                match parse_duration_string(dur_str) {
                    Ok(duration) => duration,
                    Err(e) => {
                        println!("❌ Invalid discussion-duration format: {}", e);
                        return Err(e);
                    }
                }
            } else {
                // Default 24 hours
                chrono::Duration::hours(MIN_DELIBERATION_HOURS)
            };

            // Create the proposal metadata
            let proposal = Proposal::new(
                proposal_id.to_string(),
                creator.clone(),
                Some(logic_path.to_string()),
                expires_at,
                None,       // discussion_path
                Vec::new(), // attachments
            );

            // Create identity from creator string
            let creator_identity = did_to_identity(&creator)?;

            // Create the proposal lifecycle data
            let lifecycle = ProposalLifecycle::new(
                proposal_id.to_string(),
                creator_identity,
                title.to_string(),
                (quorum * 100.0) as u64,    // Stored as percentage (0-100)
                (threshold * 100.0) as u64, // Stored as percentage (0-100)
                Some(min_delib_duration),
                required_participants.copied(),
            );

            // Read the DSL file content for storage
            let logic_content = fs::read_to_string(logic_path)
                .map_err(|e| format!("Failed to read DSL file: {}", e))?;

            // Store everything using the trait method
            vm.create_proposal(proposal, lifecycle, description, &logic_content)?;

            println!("✅ Proposal '{}' created successfully", proposal_id);

            return Ok(());
        }
        Some(("attach", attach_matches)) => {
            println!("Handling proposal attach...");

            let proposal_id = attach_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;

            let file_path = attach_matches
                .get_one::<PathBuf>("file")
                .ok_or("File path is required")?;

            // Get the custom name or use the file stem (name without extension)
            let attachment_name = attach_matches
                .get_one::<String>("name")
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    file_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "attachment".to_string())
                });

            // Check if the file exists
            if !file_path.exists() {
                return Err(format!("File not found: {}", file_path.display()).into());
            }

            // Read the file content
            let file_content =
                fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

            // Create a fork for adding the attachment
            let mut forked = vm.fork()?;
            
            // Create the attachment key path
            let attachment_key = format!(
                "{}/attachments/{}",
                VM::<S>::proposal_key_prefix(proposal_id),
                attachment_name
            );
            
            // Get values outside of storage section, clone them
            let forked_auth = forked.get_auth_context().cloned();
            let namespace = forked.get_namespace().unwrap_or("default").to_string();
            
            // Get storage and store the attachment
            {
                let storage: &mut S = forked
                    .get_storage_backend_mut()
                    .ok_or("Storage not available")?;
                
                // Store attachment bytes directly
                storage.set(forked_auth.as_ref().map(|a| a), &namespace, &attachment_key, file_content)?;
            }

            // Commit the changes
            vm.commit_fork_transaction()?;

            println!(
                "✅ Attached file '{}' to proposal '{}'",
                attachment_name, proposal_id
            );

            return Ok(());
        }
        Some(("comment", comment_matches)) => {
            let proposal_id = comment_matches.get_one::<String>("id")
                .ok_or("Proposal ID is required")?.clone();
            let content = comment_matches
                .get_one::<String>("content")
                .ok_or("Comment content is required")?.clone();
            let parent_id = comment_matches
                .get_one::<String>("parent")
                .map(|s| s.as_str());

            return handle_comment_command(vm, &proposal_id, &content, parent_id, auth_context);
        }
        Some(("view", view_matches)) => {
            let proposal_id = view_matches.get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            return handle_view_command(vm, proposal_id);
        }
        Some(("edit", edit_matches)) => {
            let proposal_id = edit_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;

            let title = edit_matches.get_one::<String>("title");
            let description = edit_matches.get_one::<String>("description");

            // Check that we have at least one field to edit
            if title.is_none() && description.is_none() {
                return Err(
                    "At least one field (title, description) must be provided for editing".into(),
                );
            }

            // Create a fork for editing
            let mut forked = vm.fork()?;
            
            // Define proposal key
            let proposal_key = VM::<S>::proposal_key_prefix(proposal_id);
            
            // Get values outside of storage section, clone them
            let auth_context_opt = forked.get_auth_context().cloned();
            let namespace = forked.get_namespace().unwrap_or("default").to_string();
            
            // Process everything in a new scope to avoid borrow issues
            {
                let storage: &mut S = forked
                    .get_storage_backend_mut()
                    .ok_or("Storage not available")?;

                // Check if proposal exists
                let exists = storage.contains(auth_context_opt.as_ref().map(|a| a), &namespace, &proposal_key)?;
                if !exists {
                    return Err(format!("Proposal with ID '{}' not found", proposal_id).into());
                }

                // Load the current proposal
                let mut proposal: Proposal = storage
                    .get_json(auth_context_opt.as_ref().map(|a| a), &namespace, &proposal_key)
                    .map_err(|e| format!("Failed to load proposal: {}", e))?;

                // Only allow editing in draft or feedback states
                if !matches!(proposal.status, ProposalStatus::Draft) {
                    return Err(format!(
                        "Cannot edit proposal '{}' in state '{:?}'. Only Draft proposals can be edited.",
                        proposal_id, proposal.status
                    ).into());
                }

                // Update fields
                if let Some(new_title) = title {
                    // The title is stored in ProposalLifecycle, not in Proposal
                    let lifecycle_key = VM::<S>::proposal_lifecycle_key(proposal_id);
                    let mut lifecycle: ProposalLifecycle = storage
                        .get_json(auth_context_opt.as_ref().map(|a| a), &namespace, &lifecycle_key)
                        .map_err(|e| format!("Failed to load proposal lifecycle: {}", e))?;

                    lifecycle.title = new_title.to_string();

                    // Save updated lifecycle
                    storage.set_json(auth_context_opt.as_ref().map(|a| a), &namespace, &lifecycle_key, &lifecycle)?;
                }

                // Save updated proposal
                storage.set_json(auth_context_opt.as_ref().map(|a| a), &namespace, &proposal_key, &proposal)?;

                // Update description if provided
                if let Some(new_description) = description {
                    let description_key = VM::<S>::proposal_description_key(proposal_id);
                    storage.set(
                        auth_context_opt.as_ref().map(|a| a),
                        &namespace,
                        &description_key,
                        new_description.as_bytes().to_vec(),
                    )?;
                }
            }

            // Commit the changes
            vm.commit_fork_transaction()?;

            println!("✅ Updated proposal '{}'", proposal_id);

            return Ok(());
        }
        Some(("publish", publish_matches)) => {
            let proposal_id = publish_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;

            // Create a fork for publishing
            let mut forked = vm.fork()?;

            // We'll use the update_proposal_state method from the trait to change the state
            vm.update_proposal_state(proposal_id, ProposalState::OpenForFeedback)?;

            println!("✅ Proposal '{}' published for feedback", proposal_id);

            return Ok(());
        }
        Some(("vote", vote_matches)) => {
            println!("Handling proposal vote...");
            let proposal_id = vote_matches.get_one::<String>("id")
                .ok_or("Proposal ID is required")?.clone();
            let vote_choice = vote_matches.get_one::<String>("vote")
                .ok_or("Vote choice is required")?.clone();
            let delegate_identity = vote_matches.get_one::<String>("as").map(|s| s.as_str());

            return handle_vote_command(
                vm,
                &proposal_id,
                &vote_choice,
                delegate_identity,
                auth_context,
            );
        }
        Some(("transition", transition_matches)) => {
            let proposal_id = transition_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;

            let state_str = transition_matches
                .get_one::<String>("state")
                .ok_or("State is required")?;

            // Parse the new state
            let new_state = match state_str.to_lowercase().as_str() {
                "draft" => ProposalState::Draft,
                "feedback" | "open_for_feedback" | "deliberation" => ProposalState::OpenForFeedback,
                "voting" => ProposalState::Voting,
                "executed" => ProposalState::Executed,
                "rejected" => ProposalState::Rejected,
                "expired" => ProposalState::Expired,
                _ => return Err(format!("Invalid state: {}", state_str).into()),
            };

            // Use the update_proposal_state method from the trait
            vm.update_proposal_state(proposal_id, new_state.clone())?;

            println!(
                "✅ Proposal '{}' transitioned to '{:?}'",
                proposal_id, new_state
            );

            return Ok(());
        }
        Some(("view", view_matches)) => {
            let proposal_id = view_matches.get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            return handle_view_command(vm, proposal_id);
        }
        Some(("list", list_matches)) => {
            // Optional status filter
            let status_filter = list_matches
                .get_one::<String>("status")
                .map(|s| s.to_string());

            // Get storage using the accessor method
            let storage = vm.get_storage_backend().ok_or("Storage not available")?;
            let auth_context_opt = vm.get_auth_context();
            let namespace = vm.get_namespace().unwrap_or("default");

            // List all proposals with our prefix
            let prefix = VM::<S>::proposal_key_prefix("");
            let keys = storage.list_keys(auth_context_opt, namespace, Some(&prefix))?;

            // Keep track of count
            let mut count = 0;

            println!("Proposals:");
            println!("----------");

            for key in keys {
                // Skip non-proposal keys (like comment keys, etc.)
                if !key.ends_with("/proposal") {
                    continue;
                }

                // Extract the ID part from the key
                let id_part = key.strip_prefix(&format!("{}/", prefix)).unwrap_or(&key);
                let id = id_part.strip_suffix("/proposal").unwrap_or(id_part);

                // Load the proposal to get its details
                match storage.get_json::<Proposal>(auth_context_opt, namespace, &key) {
                    Ok(proposal) => {
                        // Filter by status if requested
                        if let Some(ref status_str) = status_filter {
                            // Only include if status matches
                            let status_matches = match status_str.to_lowercase().as_str() {
                                "draft" => matches!(proposal.status, ProposalStatus::Draft),
                                "feedback" | "deliberation" => matches!(proposal.status, ProposalStatus::Deliberation),
                                "voting" => matches!(proposal.status, ProposalStatus::Voting),
                                "executed" => matches!(proposal.status, ProposalStatus::Executed),
                                "rejected" => matches!(proposal.status, ProposalStatus::Rejected),
                                "expired" => matches!(proposal.status, ProposalStatus::Expired),
                                "active" => matches!(proposal.status, ProposalStatus::Active),
                                "approved" => matches!(proposal.status, ProposalStatus::Approved),
                                _ => true, // Unknown status = include all
                            };

                            if !status_matches {
                                continue;
                            }
                        }

                        // Load the lifecycle to get the title
                        let lifecycle_key = VM::<S>::proposal_lifecycle_key(id);
                        let lifecycle: ProposalLifecycle = match storage.get_json(
                            auth_context_opt,
                            namespace,
                            &lifecycle_key,
                        ) {
                            Ok(lifecycle) => lifecycle,
                            Err(_) => {
                                println!("Warning: Could not load lifecycle for proposal {}", id);
                                continue;
                            }
                        };

                        println!(
                            "{}: {} - {:?}",
                            id, lifecycle.title, proposal.status
                        );
                        count += 1;
                    }
                    Err(e) => {
                        eprintln!("Error loading proposal {}: {}", id, e);
                    }
                }
            }

            if count == 0 {
                println!("No proposals found");
                if let Some(status_filter_value) = status_filter {
                    println!("(Filter: {})", status_filter_value);
                }
            } else {
                println!("\nTotal: {} proposal(s)", count);
            }

            return Ok(());
        }
        Some(("comments", comments_matches)) => {
            println!("Fetching comments for proposal...");
            let proposal_id = comments_matches.get_one::<String>("id")
                .ok_or("Proposal ID is required")?.clone();
            let sort_by = comments_matches.get_one::<String>("sort").cloned();

            // Verify the proposal exists
            let proposal = load_proposal(vm, &proposal_id)?;

            println!(
                "Comments for proposal: {} (State: {:?})",
                proposal_id, proposal.state
            );

            // Get a list of threaded comments
            println!("\nComments:");

            // Use fetch_comments_threaded to get all comments for this proposal
            let comments =
                comments::fetch_comments_threaded(vm, &proposal_id, Some(auth_context), false)?;

            if comments.is_empty() {
                println!("  No comments yet.");
            } else {
                // Find and sort root comments
                let mut roots: Vec<&comments::ProposalComment> =
                    comments.values().filter(|c| c.reply_to.is_none()).collect();

                roots.sort_by_key(|c| c.timestamp);

                // Print the threaded comments
                for root in roots {
                    print_thread(&comments, root, 0);
                    println!();
                }
            }
        }
        Some(("simulate", simulate_matches)) => {
            let proposal_id = simulate_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            return handle_simulate_command(vm, proposal_id);
        }
        Some(("dag-trace", trace_matches)) => {
            let proposal_id = trace_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            return handle_dag_trace_command(vm, proposal_id);
        }
        Some(("summary", summary_matches)) => {
            let proposal_id = summary_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            return handle_summary_command(vm, proposal_id);
        }
        Some(("execute", execute_matches)) => {
            println!("Executing proposal logic...");
            let proposal_id = execute_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?
                .clone();
            return handle_execute_command(vm, &proposal_id, auth_context);
        }
        Some(("view-comments", view_comments_matches)) => {
            let proposal_id = view_comments_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?
                .clone();
            let threaded = view_comments_matches.get_flag("threaded");

            return handle_view_comments_command(vm, &proposal_id, threaded, auth_context);
        }
        Some(("export", export_matches)) => {
            println!("Handling proposal export...");
            let proposal_id = export_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?
                .clone();
            let output_path = export_matches.get_one::<String>("output").cloned();

            return handle_export_command(vm, &proposal_id, output_path, auth_context);
        }
        Some(("comment-react", react_matches)) => {
            let comment_id = react_matches
                .get_one::<String>("id")
                .ok_or("Comment ID is required")?;
            let proposal_id = react_matches
                .get_one::<String>("proposal-id")
                .ok_or("Proposal ID is required")?;
            let reaction = react_matches
                .get_one::<String>("reaction")
                .ok_or("Reaction is required")?;

            return handle_comment_react_command(
                vm,
                comment_id,
                proposal_id,
                reaction,
                auth_context,
            );
        }
        Some(("comment-tag", tag_matches)) => {
            let comment_id = tag_matches
                .get_one::<String>("id")
                .ok_or("Comment ID is required")?;
            let proposal_id = tag_matches
                .get_one::<String>("proposal-id")
                .ok_or("Proposal ID is required")?;
            let tags: Vec<String> = if let Some(tag_values) = tag_matches.get_many::<String>("tag")
            {
                tag_values.cloned().collect()
            } else {
                Vec::new()
            };

            return handle_comment_tag_command(vm, comment_id, proposal_id, &tags, auth_context);
        }
        Some(("thread", thread_matches)) => {
            let proposal_id = thread_matches
                .get_one::<String>("id")
                .ok_or("Proposal ID is required")?;
            let show_hidden = thread_matches.get_flag("show-hidden");

            return handle_thread_command(vm, proposal_id, show_hidden, auth_context);
        }
        Some(("comment-edit", edit_matches)) => {
            let comment_id = edit_matches
                .get_one::<String>("id")
                .ok_or("Comment ID is required")?;
            let proposal_id = edit_matches
                .get_one::<String>("proposal-id")
                .ok_or("Proposal ID is required")?;
            let new_text = edit_matches
                .get_one::<String>("text")
                .ok_or("New text is required")?;

            return handle_comment_edit_command(
                vm,
                comment_id,
                proposal_id,
                new_text,
                auth_context,
            );
        }
        Some(("comment-hide", hide_matches)) => {
            let comment_id = hide_matches
                .get_one::<String>("id")
                .ok_or("Comment ID is required")?;
            let proposal_id = hide_matches
                .get_one::<String>("proposal-id")
                .ok_or("Proposal ID is required")?;

            return handle_comment_hide_command(vm, comment_id, proposal_id, auth_context);
        }
        Some(("comment-history", history_matches)) => {
            let comment_id = history_matches
                .get_one::<String>("id")
                .ok_or("Comment ID is required")?;
            let proposal_id = history_matches
                .get_one::<String>("proposal-id")
                .ok_or("Proposal ID is required")?;

            return handle_comment_history_command(vm, comment_id, proposal_id, Some(auth_context));
        }
        _ => unreachable!("Subcommand should be required"),
    }
    Ok(())
}

/// Fetch comments for a proposal in a threaded structure
///
/// # Parameters
/// * `vm` - The virtual machine with access to storage
/// * `proposal_id` - The ID of the proposal to fetch comments for
/// * `auth` - Optional authentication context
/// * `show_hidden` - Whether to include hidden comments
///
/// # Returns
/// * `Result<HashMap<String, ProposalComment>, Box<dyn Error>>` - Map of comment IDs to comments
pub fn fetch_comments_threaded<S>(
    vm: &VM<S>,
    proposal_id: &str,
    auth: Option<&AuthContext>,
    show_hidden: bool,
) -> Result<HashMap<String, ProposalComment>, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Convert from the comments::ProposalComment to our local ProposalComment
    let new_comments =
        crate::governance::comments::fetch_comments_threaded(vm, proposal_id, auth, show_hidden)?;

    let mut comments = HashMap::new();
    for (id, comment) in new_comments {
        comments.insert(
            id.clone(),
            ProposalComment {
                id: comment.id,
                author: comment.author,
                timestamp: comment.timestamp,
                content: comment.content,
                reply_to: comment.reply_to,
                tags: comment.tags,
                reactions: comment.reactions,
            },
        );
    }

    Ok(comments)
}

/// Print comments in a threaded/hierarchical display
///
/// Recursively displays a comment and all its replies with proper indentation.
///
/// # Parameters
/// * `comment_id` - ID of the comment to print
/// * `comments_map` - HashMap of all comments, keyed by comment ID
/// * `replies_map` - HashMap mapping each comment ID to a vector of its reply comment IDs
/// * `depth` - Current indentation depth (0 for top-level comments)
fn print_view_comments(
    comment_id: &CommentId,
    comments_map: &HashMap<CommentId, ProposalComment>,
    replies_map: &HashMap<Option<CommentId>, Vec<CommentId>>,
    depth: usize,
) {
    if let Some(comment) = comments_map.get(comment_id) {
        // Indent based on depth
        let indent = " ".repeat(depth * 4);

        // Format timestamp as a readable date/time
        let timestamp_str = comment
            .timestamp
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();

        println!(
            "{}└─ Comment {} (from {})",
            indent, comment.id, comment.author
        );
        println!("{}   Date: {}", indent, timestamp_str);
        println!("{}   {}", indent, comment.content);

        // Print tags if present
        if !comment.tags.is_empty() {
            println!("{}   Tags: {}", indent, comment.tags.join(", "));
        }

        // Print reactions if present
        if !comment.reactions.is_empty() {
            let reactions_str: Vec<String> = comment
                .reactions
                .iter()
                .map(|(emoji, count)| format!("{} {}", emoji, count))
                .collect();
            println!("{}   Reactions: {}", indent, reactions_str.join(", "));
        }

        // Print any replies recursively
        if let Some(replies) = replies_map.get(&Some(comment.id.clone())) {
            for reply_id in replies {
                print_view_comments(reply_id, comments_map, replies_map, depth + 1);
            }
        }
    }
}

/// Check if a proposal status matches a status string
///
/// Helper function to match status enum values with their string representations
/// for filtering proposals by status.
///
/// # Parameters
/// * `status` - The proposal status enum value to check
/// * `status_str` - The status string to match against
///
/// # Returns
/// * `bool` - True if the status matches the string representation
fn match_status(status: &LocalProposalStatus, status_str: &str) -> bool {
    match status_str.to_lowercase().as_str() {
        "draft" => matches!(status, LocalProposalStatus::Draft),
        "deliberation" => matches!(status, LocalProposalStatus::Deliberation),
        "active" => matches!(status, LocalProposalStatus::Active),
        "voting" => matches!(status, LocalProposalStatus::Voting),
        "executed" => matches!(status, LocalProposalStatus::Executed),
        "rejected" => matches!(status, LocalProposalStatus::Rejected),
        "expired" => matches!(status, LocalProposalStatus::Expired),
        _ => false,
    }
}

/// Display a summary of a proposal
///
/// Prints key information about a proposal in a concise format,
/// suitable for listing multiple proposals.
///
/// # Parameters
/// * `proposal` - The proposal to summarize
fn print_proposal_summary(proposal: &Proposal) {
    println!(
        "ID: {} | Status: {:?} | Creator: {}",
        proposal.id, proposal.status, proposal.creator
    );
    println!(
        "  Created: {} | Attachments: {}",
        proposal.created_at.to_rfc3339(),
        proposal.attachments.len()
    );
    if let Some(result) = &proposal.execution_result {
        println!("  Result: {}", result);
    }
    println!("---------------------");
}

/// Loads a proposal from storage and handles errors uniformly
pub fn load_proposal_from_governance<S>(
    vm: &VM<S>,
    proposal_id: &ProposalId,
) -> Result<Proposal, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Use our trait method to load the proposal metadata
    vm.get_proposal(proposal_id)
}

/// Count the votes for a proposal
pub fn count_votes<S>(
    vm: &VM<S>,
    proposal_id: &ProposalId,
) -> Result<(u32, u32, u32), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get all votes using our trait method
    let votes = vm.get_proposal_votes(proposal_id)?;

    // Count the votes by type
    let mut yes_votes = 0;
    let mut no_votes = 0;
    let mut abstain_votes = 0;

    for (_, vote) in votes {
        match vote.to_lowercase().as_str() {
            "yes" => yes_votes += 1,
            "no" => no_votes += 1,
            "abstain" => abstain_votes += 1,
            _ => {} // Invalid vote, ignore
        }
    }

    Ok((yes_votes, no_votes, abstain_votes))
}

/// Handle the view command to display proposal details
fn handle_view_command<S>(vm: &VM<S>, proposal_id: &str) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Load the proposal
    let proposal_id_string = proposal_id.to_string();
    let proposal = load_proposal_from_governance(vm, &proposal_id_string)?;

    // Count votes
    let (yes_votes, no_votes, abstain_votes) = count_votes(vm, &proposal_id_string)?;
    let total_votes = yes_votes + no_votes + abstain_votes;

    // Calculate participation percentage for quorum
    let quorum_percentage = if let Ok(lifecycle) = load_proposal(vm, &proposal_id_string) {
        if lifecycle.quorum > 0 {
            let quorum_percentage = (total_votes as f64 / lifecycle.quorum as f64) * 100.0;
            format!("{:.1}%", quorum_percentage)
        } else {
            "N/A".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    // Calculate threshold percentage
    let threshold_percentage = if let Ok(lifecycle) = load_proposal(vm, &proposal_id_string) {
        if lifecycle.threshold > 0 && total_votes > 0 {
            let threshold_percentage = (yes_votes as f64 / total_votes as f64) * 100.0;
            format!("{:.1}%", threshold_percentage)
        } else {
            "N/A".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    // Print formatted output
    println!("\n=== Proposal Details: {} ===", proposal_id);
    println!(
        "Title:     {}",
        load_proposal(vm, &proposal_id_string)
            .map(|p| p.title)
            .unwrap_or_else(|_| "N/A".to_string())
    );
    println!("Creator:   {}", proposal.creator);
    println!("Status:    {:?}", proposal.status);
    println!("Created:   {}", proposal.created_at);

    // Print vote counts
    println!("\n=== Voting Information ===");
    println!("Yes votes:      {}", yes_votes);
    println!("No votes:       {}", no_votes);
    println!("Abstain votes:  {}", abstain_votes);
    println!("Total votes:    {}", total_votes);
    println!("Quorum:         {}", quorum_percentage);
    println!("Threshold:      {}", threshold_percentage);

    // Print execution result if any
    if let Some(result) = &proposal.execution_result {
        println!("\n=== Execution Result ===");
        println!("{}", result);
    }

    // Print other metadata
    println!("\n=== Additional Information ===");
    if let Some(expires) = &proposal.expires_at {
        println!("Expires at: {}", expires);
    }

    if let Some(logic_path) = &proposal.logic_path {
        println!("Logic path: {}", logic_path);
    }

    Ok(())
}

/// Load a ProposalLifecycle for more information
fn load_proposal_lifecycle<S>(
    vm: &VM<S>,
    proposal_id: &str,
) -> Result<ProposalLifecycle, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Try loading the old proposal lifecycle format
    let storage_key = format!("proposals/{}", proposal_id);

    let proposal_data = vm
        .get_storage_backend()
        .ok_or_else(|| VMError::StorageUnavailable)?
        .get(None, "proposals", &storage_key)
        .map_err(|e| {
            eprintln!("Failed to read proposal lifecycle: {}", e);
            Box::new(e) as Box<dyn Error>
        })?;

    // Deserialize the proposal
    serde_json::from_slice::<ProposalLifecycle>(&proposal_data).map_err(|e| {
        eprintln!("Failed to deserialize proposal lifecycle: {}", e);
        Box::new(e) as Box<dyn Error>
    })
}

/// Handle the summary command to display a condensed overview of a proposal
#[allow(unused)]
pub fn handle_summary_command<S>(vm: &VM<S>, proposal_id: &str) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Get proposal details
    let proposal_id_string = proposal_id.to_string();
    let proposal = load_proposal_from_governance(vm, &proposal_id_string)?;

    // Get vote information
    let (yes_votes, no_votes, abstain_votes) = count_votes(vm, &proposal_id_string)?;
    let total_votes = yes_votes + no_votes + abstain_votes;

    // Count comments
    let auth_context = None; // No auth needed for summary
    let comments = fetch_comments_threaded(vm, proposal_id, auth_context, false)?;
    let comment_count = comments.len();

    // Calculate some statistics
    let top_commenters: Vec<(&String, usize)> = comments
        .iter()
        .map(|(_, comment)| &comment.author)
        .fold(HashMap::new(), |mut map, author| {
            *map.entry(author).or_insert(0) += 1;
            map
        })
        .iter()
        .map(|(author, count)| (*author, *count))
        .collect();

    // Find the last activity timestamp
    let last_activity = comments
        .values()
        .map(|c| c.timestamp)
        .max()
        .unwrap_or(proposal.created_at);

    // Print summary
    println!("\n=== Proposal Summary: {} ===", proposal_id);
    if let Ok(lifecycle) = load_proposal_lifecycle(vm, proposal_id) {
        println!("Title:      {}", lifecycle.title);
    }
    println!("Status:     {:?}", proposal.status);
    println!("Created:    {}", proposal.created_at);
    println!("Last activity: {}", last_activity);

    // Print vote summary
    println!("\n=== Vote Summary ===");
    println!(
        "Yes:     {} ({:.1}%)",
        yes_votes,
        if total_votes > 0 {
            (yes_votes as f64 / total_votes as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "No:      {} ({:.1}%)",
        no_votes,
        if total_votes > 0 {
            (no_votes as f64 / total_votes as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Abstain: {} ({:.1}%)",
        abstain_votes,
        if total_votes > 0 {
            (abstain_votes as f64 / total_votes as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("Total:   {}", total_votes);

    // Print comment summary
    println!("\n=== Comment Summary ===");
    println!("Total comments: {}", comment_count);

    if !top_commenters.is_empty() {
        println!("\nTop commenters:");
        for (author, count) in top_commenters.iter().take(5) {
            println!("  {}: {} comments", author, count);
        }
    }

    Ok(())
}

/// Handle the simulate command to test execution of a proposal without making persistent changes
#[allow(unused)]
pub fn handle_simulate_command<S>(vm: &mut VM<S>, proposal_id: &str) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Stub implementation for now
    println!("Simulating proposal execution for ID: {}", proposal_id);
    Ok(())
}

/// Handle the comment-react command to add reactions to comments
#[allow(unused)]
pub fn handle_comment_react_command<S>(
    vm: &mut VM<S>,
    comment_id: &str,
    proposal_id: &str,
    reaction: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    unimplemented!("Stub implementation")
}

/// Handle the comment-tag command to add tags to comments
#[allow(unused)]
pub fn handle_comment_tag_command<S>(
    vm: &mut VM<S>,
    comment_id: &str,
    proposal_id: &str,
    tags: &[String],
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    unimplemented!("Stub implementation")
}

/// Print comment thread with proper indentation
fn print_thread(
    comments: &HashMap<String, comments::ProposalComment>,
    comment: &comments::ProposalComment,
    depth: usize,
) {
    let indent = "  ".repeat(depth);

    // If the comment is hidden, show a placeholder
    if comment.hidden {
        println!(
            "{}└─ [{}] [HIDDEN] by {} at {}",
            indent,
            comment.id,
            comment.author,
            comment.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
    } else {
        println!(
            "{}└─ [{}] by {} at {}",
            indent,
            comment.id,
            comment.author,
            comment.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
        println!("{}   {}", indent, comment.content);

        // Print tags if available
        if !comment.tags.is_empty() {
            println!("{}   Tags: {}", indent, comment.tags.join(", "));
        }

        // Print reactions if available
        if !comment.reactions.is_empty() {
            let reaction_str = comment
                .reactions
                .iter()
                .map(|(emoji, count)| format!("{} ({})", emoji, count))
                .collect::<Vec<_>>()
                .join(", ");
            println!("{}   Reactions: {}", indent, reaction_str);
        }
    }

    // Find and sort replies to this comment
    let mut replies: Vec<&comments::ProposalComment> = comments
        .values()
        .filter(|c| c.reply_to.as_ref() == Some(&comment.id))
        .collect();

    replies.sort_by_key(|c| c.timestamp);

    for reply in replies {
        print_thread(comments, reply, depth + 1);
    }
}

/// Handle the thread command to show threaded comments
pub fn handle_thread_command<S>(
    vm: &VM<S>,
    proposal_id: &str,
    show_hidden: bool,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Fetch comments with or without hidden ones based on flag
    let comments =
        comments::fetch_comments_threaded(vm, proposal_id, Some(auth_context), show_hidden)?;

    if comments.is_empty() {
        println!("No comments found for proposal {}", proposal_id);
        return Ok(());
    }

    // Find root comments (those without a parent)
    let mut roots: Vec<&comments::ProposalComment> =
        comments.values().filter(|c| c.reply_to.is_none()).collect();

    // Sort by timestamp (oldest first)
    roots.sort_by_key(|c| c.timestamp);

    println!("Threaded comments for proposal {}:", proposal_id);
    println!(
        "{} total comments{}",
        comments.len(),
        if show_hidden {
            " (including hidden)"
        } else {
            ""
        }
    );

    for root in roots {
        print_thread(&comments, root, 0);
        println!();
    }

    Ok(())
}

/// Handle the comment-edit command
pub fn handle_comment_edit_command<S>(
    vm: &mut VM<S>,
    comment_id: &str,
    proposal_id: &str,
    new_text: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Edit the comment (this will verify authorship)
    comments::edit_comment(vm, proposal_id, comment_id, new_text, auth_context)?;

    println!("Comment {} has been edited successfully.", comment_id);
    println!("All versions remain stored and can be viewed with the comment-history command.");

    Ok(())
}

/// Handle the comment-hide command
pub fn handle_comment_hide_command<S>(
    vm: &mut VM<S>,
    comment_id: &str,
    proposal_id: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Hide the comment (this will verify authorship)
    comments::hide_comment(vm, proposal_id, comment_id, auth_context)?;

    println!("Comment {} has been hidden.", comment_id);
    println!("The comment is still stored and can be viewed with the --show-hidden flag.");

    Ok(())
}

/// Handle the comment-history command
pub fn handle_comment_history_command<S>(
    vm: &VM<S>,
    comment_id: &str,
    proposal_id: &str,
    auth_context: Option<&AuthContext>,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get the comment's edit history
    let versions = comments::get_comment_history(vm, proposal_id, comment_id, auth_context)?;

    if versions.is_empty() {
        println!("No history found for comment {}", comment_id);
        return Ok(());
    }

    println!("Comment {} edit history:", comment_id);

    for (i, version) in versions.iter().enumerate() {
        println!("Version {}:", i + 1);
        println!(
            "  Timestamp: {}",
            version.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
        println!("  Content: {}", version.content);

        // If there's a next version, show a diff (simple implementation)
        if i < versions.len() - 1 {
            let next = &versions[i + 1];

            // Very simple diff - just show if length changed
            let old_len = version.content.len();
            let new_len = next.content.len();

            if old_len != new_len {
                println!(
                    "  Changes: {} -> {} characters ({:+})",
                    old_len,
                    new_len,
                    new_len as isize - old_len as isize
                );
            }
        }

        println!();
    }

    Ok(())
}

/// Handle the vote command to cast a vote on a proposal
pub fn handle_vote_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    vote_choice: &str,
    delegate_identity: Option<&str>,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get the voter ID, which is the authenticated user's identity
    let voter_id = auth_context.identity_did().to_string();

    // Determine the effective voter (uses delegate's identity if provided)
    let delegate = if let Some(delegate_did) = delegate_identity {
        // In a real implementation, verify the delegation relationship
        // For MVP, we'll just allow it if specified
        delegate_did.to_string()
    } else {
        voter_id.clone()
    };

    // First check if the proposal exists
    if !vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not configured for proposal vote")?
        .contains(
            Some(auth_context),
            &vm.get_namespace().unwrap_or("default"),
            &VM::<S>::proposal_key_prefix(proposal_id),
        )?
    {
        return Err(format!("Proposal with ID '{}' not found", proposal_id).into());
    }

    // Load the proposal lifecycle to check deliberation period
    let proposal_lifecycle = vm.get_proposal_lifecycle(proposal_id)?;

    // Check if the minimum deliberation period has passed
    if let Some(min_deliberation) = proposal_lifecycle.discussion_duration {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(proposal_lifecycle.created_at);

        if elapsed < min_deliberation {
            // Calculate hours for both required and elapsed time
            let required_hours = min_deliberation.num_hours();
            let elapsed_hours = elapsed.num_hours();

            return Err(format!(
                "⏳ Proposal '{}' is still in deliberation.\n   Required: {} hours\n   Elapsed: {} hours",
                proposal_id, required_hours, elapsed_hours
            ).into());
        }
    }

    // Validate vote choice
    let vote_value = match vote_choice.to_lowercase().as_str() {
        "yes" => "yes",
        "no" => "no",
        "abstain" => "abstain",
        _ => {
            return Err(format!(
                "Invalid vote choice: '{}'. Must be yes, no, or abstain",
                vote_choice
            )
            .into())
        }
    };

    // Cast the vote using the trait method
    vm.cast_vote(proposal_id, &voter_id, vote_value, delegate_identity)?;

    println!(
        "✅ Vote '{}' recorded for proposal '{}' by '{}'",
        vote_value, proposal_id, voter_id
    );

    // Award reputation for participation
    let rep_dsl = format!(
        "increment_reputation \"{}\" reason=\"Voted on proposal {}\"",
        voter_id, proposal_id
    );
    let (ops, _) = parse_dsl(&rep_dsl)?;
    vm.execute(&ops)?;

    Ok(())
}

/// Handle the execute command to run proposal logic if it passed
pub fn handle_execute_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // First check if proposal exists
    if !vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not configured for proposal execution")?
        .contains(
            Some(auth_context),
            &vm.get_namespace().unwrap_or("default"),
            &VM::<S>::proposal_key_prefix(proposal_id),
        )?
    {
        return Err(format!("Proposal with ID '{}' not found", proposal_id).into());
    }

    // Tally votes
    let votes = vm.get_proposal_votes(proposal_id)?;

    let mut yes_votes = 0;
    let mut no_votes = 0;
    let mut abstain_votes = 0;

    for (_, vote) in &votes {
        match vote.to_lowercase().as_str() {
            "yes" => yes_votes += 1,
            "no" => no_votes += 1,
            "abstain" => abstain_votes += 1,
            _ => {} // Invalid vote value, ignore
        }
    }

    // Calculate totals and ratios
    let total_votes = yes_votes + no_votes + abstain_votes;
    let yes_ratio = if total_votes > 0 {
        yes_votes as f64 / total_votes as f64
    } else {
        0.0
    };

    // Load the proposal metadata to get quorum and threshold
    let proposal_lifecycle = vm.get_proposal_lifecycle(proposal_id)?;

    // Check if proposal has already been executed
    if matches!(proposal_lifecycle.state, ProposalState::Executed) {
        return Err(format!("Proposal '{}' has already been executed", proposal_id).into());
    }

    // Convert stored percentages to ratios (they're stored as integers 0-100)
    let quorum_ratio = proposal_lifecycle.quorum as f64 / 100.0;
    let threshold_ratio = proposal_lifecycle.threshold as f64 / 100.0;

    // Calculate participation rate
    let required_participants = proposal_lifecycle.required_participants.unwrap_or(1);
    let participation_rate = if required_participants > 0 {
        total_votes as f64 / required_participants as f64
    } else {
        1.0 // Avoid division by zero
    };

    // Check if proposal passed
    let quorum_met = participation_rate >= quorum_ratio;
    let threshold_met = yes_ratio >= threshold_ratio;

    // If proposal did not pass, return with message
    if !quorum_met {
        println!(
            "❌ Proposal '{}' did not meet quorum requirement.",
            proposal_id
        );
        println!(
            "   Participation: {:.1}% (Required: {:.1}%)",
            participation_rate * 100.0,
            quorum_ratio * 100.0
        );
        return Ok(());
    }

    if !threshold_met {
        println!(
            "❌ Proposal '{}' did not meet threshold requirement.",
            proposal_id
        );
        println!(
            "   Yes votes: {:.1}% (Required: {:.1}%)",
            yes_ratio * 100.0,
            threshold_ratio * 100.0
        );
        return Ok(());
    }

    // Proposal passed! Execute logic
    println!("✅ Proposal '{}' passed. Executing logic...", proposal_id);
    println!(
        "   Votes: {} yes, {} no, {} abstain",
        yes_votes, no_votes, abstain_votes
    );

    // Use the execute_proposal method from our trait
    match vm.execute_proposal(proposal_id) {
        Ok(_) => {
            println!("✅ Logic executed successfully.");
            Ok(())
        }
        Err(e) => {
            println!("⚠️ Logic execution failed: {}", e);
            Ok(()) // We still return Ok since the command itself succeeded, even if the execution failed
        }
    }
}

/// Handle the view-comments command to display all comments for a proposal
pub fn handle_view_comments_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    threaded: bool,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get reference to storage
    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not configured for viewing comments")?;

    // Get the namespace from VM
    let namespace = vm.get_namespace().unwrap_or("default");

    // Load the proposal to verify it exists
    let base_key = format!("governance_proposals/{}", proposal_id);

    // First check if proposal exists
    if !storage.contains(Some(auth_context), &namespace, &base_key)? {
        return Err(format!("Proposal with ID '{}' not found", proposal_id).into());
    }

    // List all comment keys for this proposal
    let comments_prefix = format!("{}/comments/", base_key);
    let comment_keys = storage.list_keys(Some(auth_context), &namespace, Some(&comments_prefix))?;

    if comment_keys.is_empty() {
        println!("No comments found for proposal '{}'", proposal_id);
        return Ok(());
    }

    // Load all comments
    let mut comments = Vec::new();
    for key in comment_keys {
        match storage.get_json::<StoredComment>(Some(auth_context), &namespace, &key) {
            Ok(comment) => {
                comments.push(comment);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse comment at {}: {}", key, e);
                // Continue with other comments
            }
        }
    }

    // Sort comments by timestamp
    comments.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    println!("Comments for proposal '{}':", proposal_id);
    println!();

    if threaded {
        // Display threaded comments
        display_threaded_comments(&comments);
    } else {
        // Display flat comments
        for comment in &comments {
            let author_short = shorten_did(&comment.author);
            println!("🗣️  {} at {}", author_short, comment.timestamp);
            println!("    {}", comment.content);
            println!();
        }
    }

    Ok(())
}

/// Helper function to display comments in a threaded view
fn display_threaded_comments(comments: &[StoredComment]) {
    // First, create a map of parent -> children
    let mut children_map: HashMap<Option<String>, Vec<usize>> = HashMap::new();

    // Initialize with an empty vec for root comments (no parent)
    children_map.insert(None, Vec::new());

    // Fill the map
    for (i, comment) in comments.iter().enumerate() {
        children_map
            .entry(comment.parent.clone())
            .or_insert_with(Vec::new)
            .push(i);
    }

    // Now recursively display comments, starting with the root comments
    if let Some(root_indices) = children_map.get(&None) {
        for &idx in root_indices {
            display_comment(comments, idx, &children_map, 0);
        }
    }
}

/// Helper function to display a single comment with its replies
fn display_comment(
    comments: &[StoredComment],
    index: usize,
    children_map: &HashMap<Option<String>, Vec<usize>>,
    depth: usize,
) {
    let comment = &comments[index];
    let indent = "    ".repeat(depth);
    let author_short = shorten_did(&comment.author);

    // Print the current comment
    if depth == 0 {
        println!("{}🗣️  {} at {}", indent, author_short, comment.timestamp);
    } else {
        println!("{}↳   {} replied:", indent, author_short);
    }
    println!("{}    {}", indent, comment.content);
    println!();

    // Print replies if any
    let comment_id = format!(
        "{}_{}",
        match DateTime::parse_from_rfc3339(&comment.timestamp) {
            Ok(dt) => dt.timestamp(),
            Err(_) => Utc::now().timestamp(),
        },
        comment.author
    );

    if let Some(reply_indices) = children_map.get(&Some(comment_id)) {
        for &reply_idx in reply_indices {
            display_comment(comments, reply_idx, children_map, depth + 1);
        }
    }
}

/// Helper function to shorten DIDs for display
fn shorten_did(did: &str) -> String {
    if did.starts_with("did:") {
        // For DIDs like did:coop:user123, extract just the user123 part
        if let Some(last_part) = did.split(':').last() {
            return last_part.to_string();
        }
    }
    // If not a DID or couldn't extract, just return as is
    did.to_string()
}

/// A struct to represent the complete proposal export data
#[derive(Debug, Serialize, Deserialize)]
struct ProposalExport {
    id: String,
    title: String,
    creator: String,
    state: String,
    created_at: String,
    expires_at: Option<String>,
    quorum: f64,
    threshold: f64,
    description: Option<String>,
    logic: Option<String>,
    execution_status: Option<String>,
    votes: Vec<VoteExport>,
    comments: Vec<CommentExport>,
}

/// A struct to represent a vote in the export
#[derive(Debug, Serialize, Deserialize)]
struct VoteExport {
    voter: String,
    vote: String,
    timestamp: String,
    delegated_by: Option<String>,
}

/// A struct to represent a comment in the export
#[derive(Debug, Serialize, Deserialize)]
struct CommentExport {
    author: String,
    timestamp: String,
    content: String,
    parent: Option<String>,
}

/// Handle the export command to export proposal data to a JSON file
pub fn handle_export_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    output_path: Option<String>,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Get storage backend
    let storage = vm
        .get_storage_backend()
        .ok_or_else(|| "Storage backend not configured for proposal export")?;

    // Use default namespace as in the proposal creation
    let namespace = "default";

    // First load the proposal lifecycle
    let lifecycle_key = format!("governance_proposals/{}/lifecycle", proposal_id);
    let proposal_lifecycle: ProposalLifecycle =
        match storage.get_json(Some(auth_context), namespace, &lifecycle_key) {
            Ok(lifecycle) => lifecycle,
            Err(e) => return Err(format!("Failed to load proposal lifecycle: {}", e).into()),
        };

    // Load proposal description if available
    let description_key = format!("governance_proposals/{}/description", proposal_id);
    let description = match storage.get(Some(auth_context), namespace, &description_key) {
        Ok(bytes) => Some(String::from_utf8(bytes)?),
        Err(_) => None,
    };

    // Load proposal logic if available
    let logic_key = format!("governance_proposals/{}/logic", proposal_id);
    let logic = match storage.get(Some(auth_context), namespace, &logic_key) {
        Ok(bytes) => Some(String::from_utf8(bytes)?),
        Err(_) => None,
    };

    // Load votes
    let votes_prefix = format!("governance_proposals/{}/votes/", proposal_id);
    let vote_keys = storage.list_keys(Some(auth_context), namespace, Some(&votes_prefix))?;

    let mut votes = Vec::new();
    for key in vote_keys {
        match storage.get_json::<serde_json::Value>(Some(auth_context), namespace, &key) {
            Ok(vote_data) => {
                // Extract relevant fields from the vote data
                let voter = vote_data["voter"].as_str().unwrap_or("unknown").to_string();
                let vote = vote_data["vote"].as_str().unwrap_or("unknown").to_string();
                let timestamp = vote_data["timestamp"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let delegated_by = vote_data["delegated_by"].as_str().map(|s| s.to_string());

                votes.push(VoteExport {
                    voter,
                    vote,
                    timestamp,
                    delegated_by,
                });
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse vote at {}: {}", key, e);
                // Continue with other votes
            }
        }
    }

    // Load comments
    let comments_prefix = format!("governance_proposals/{}/comments/", proposal_id);
    let comment_keys = storage.list_keys(Some(auth_context), namespace, Some(&comments_prefix))?;

    let mut comments = Vec::new();
    for key in comment_keys {
        match storage.get_json::<StoredComment>(Some(auth_context), namespace, &key) {
            Ok(comment) => {
                comments.push(CommentExport {
                    author: comment.author,
                    timestamp: comment.timestamp,
                    content: comment.content,
                    parent: comment.parent,
                });
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse comment at {}: {}", key, e);
                // Continue with other comments
            }
        }
    }

    // Build the export structure
    let export = ProposalExport {
        id: proposal_lifecycle.id.clone(),
        title: proposal_lifecycle.title.clone(),
        creator: proposal_lifecycle.creator.did().to_string(),
        state: format!("{:?}", proposal_lifecycle.state),
        created_at: proposal_lifecycle.created_at.to_rfc3339(),
        expires_at: proposal_lifecycle.expires_at.map(|dt| dt.to_rfc3339()),
        quorum: proposal_lifecycle.quorum as f64 / 100.0, // Convert from percentage to decimal
        threshold: proposal_lifecycle.threshold as f64 / 100.0, // Convert from percentage to decimal
        description,
        logic,
        execution_status: proposal_lifecycle
            .execution_status
            .map(|status| format!("{:?}", status)),
        votes,
        comments,
    };

    // Determine output file path
    let output_file_path = match output_path {
        Some(path) => path,
        None => format!("proposal_{}.json", proposal_id),
    };

    // Write to file
    let file = std::fs::File::create(&output_file_path)?;
    serde_json::to_writer_pretty(file, &export)?;

    println!(
        "✅ Exported proposal '{}' to {}",
        proposal_id, output_file_path
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::implementations::in_memory::InMemoryStorage;

    fn setup_test_vm() -> VM<InMemoryStorage> {
        let mut vm = VM::new();
        let auth = setup_test_auth();
        vm.set_auth_context(auth);
        vm.set_namespace("test_ns");
        vm.set_storage_backend(InMemoryStorage::new());
        vm
    }

    fn setup_test_auth() -> AuthContext {
        // Simple test auth
        let identity = Identity::new("test_user".to_string(), None, "member".to_string(), None)
            .expect("Failed to create test identity");
        let mut auth = AuthContext::new("test_user");
        auth.add_role("governance", "admin");
        auth
    }

    fn test_storage_set(
        storage: &mut InMemoryStorage,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        storage.set(auth, namespace, key, data)?;
        Ok(())
    }

    fn test_storage_get(
        storage: &InMemoryStorage,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(storage.get(auth, namespace, key)?)
    }

    fn create_test_proposal(
        vm: &mut VM<InMemoryStorage>,
        proposal_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Create test proposal data
        let proposal_data = r#"{"id":"test_proposal","title":"Test Proposal","creator":"alice","status":"Draft","created_at":"2023-01-01T00:00:00Z"}"#.as_bytes().to_vec();
        
        // Set the proposal data directly
        let storage_key = format!("proposals/{}", proposal_id);
        {
            let storage = vm.get_storage_backend_mut()
                .ok_or_else(|| "Storage not available".to_string())?;
            storage.set(None, "proposals", &storage_key, proposal_data)?;
        }

        // Also create a lifecycle
        let lifecycle_key = format!("proposals/{}", proposal_id);
        let lifecycle_data = r#"{"state":"Draft","quorum":2,"threshold":2}"#.as_bytes().to_vec();
        
        {
            let storage = vm.get_storage_backend_mut()
                .ok_or_else(|| "Storage not available".to_string())?;
            storage.set(None, "proposals", &lifecycle_key, lifecycle_data)?;
        }

        // Create test logic
        let test_logic = "SETSTORE key value\nACTIVATE id";
        let logic_key = format!("proposals/{}/logic", proposal_id);
        
        {
            let storage = vm.get_storage_backend_mut()
                .ok_or_else(|| "Storage not available".to_string())?;
            storage.set(None, "proposals", &logic_key, test_logic.as_bytes().to_vec())?;
        }

        Ok(())
    }
    
    // ...rest of test methods...
}

/// Simple comment structure for storage
#[derive(Debug, Serialize, Deserialize)]
struct StoredComment {
    author: String,
    timestamp: String,
    content: String,
    parent: Option<String>,
}

/// Handle the command to add a comment to a proposal
pub fn handle_comment_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    content: &str,
    parent_id: Option<&str>,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Add the comment to the proposal
    let comment = comments::create_comment(
        vm, 
        proposal_id, 
        &auth_context.current_identity_did, 
        content, 
        parent_id,
        Vec::new(), // Empty tags for now
        auth_context
    )?;

    println!("✅ Comment added successfully. Comment ID: {}", comment.id);

    // If this is a reply to another comment, mention that
    if let Some(parent_comment_id) = parent_id {
        println!("   This is a reply to comment {}", parent_comment_id);
    }

    Ok(())
}

/// Handle the dag-trace command to visualize the DAG path of a proposal
pub fn handle_dag_trace_command<S>(
    vm: &VM<S>,
    proposal_id: &str,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    if let Some(ledger) = &vm.dag {
        if let Some(start_id) = ledger.find_proposal_node_id(proposal_id) {
            let trace = ledger.trace(&start_id);
            
            println!("📜 DAG Trace for proposal '{}':", proposal_id);
            println!("Number of nodes in trace: {}", trace.len());
            
            if trace.is_empty() {
                println!("No related nodes found.");
                return Ok(());
            }
            
            // Pretty format time
            let format_time = |timestamp: u64| -> String {
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now());
                dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            };
            
            // Print the nodes in reverse chronological order (newest first)
            for node in trace.iter().rev() {
                match &node.data {
                    icn_ledger::NodeData::ProposalCreated { proposal_id, title } => {
                        println!("📝 Proposal Created [{}]", node.id);
                        println!("   ID: {}", proposal_id);
                        println!("   Title: {}", title);
                        println!("   Time: {}", format_time(node.timestamp));
                        println!("   Parents: {}", node.parent_ids.join(", "));
                    },
                    icn_ledger::NodeData::VoteCast { proposal_id, voter, vote } => {
                        let vote_str = match *vote as i32 {
                            1 => "YES",
                            0 => "NO",
                            _ => "ABSTAIN",
                        };
                        println!("🗳️ Vote Cast [{}]", node.id);
                        println!("   Proposal: {}", proposal_id);
                        println!("   Voter: {}", voter);
                        println!("   Vote: {}", vote_str);
                        println!("   Time: {}", format_time(node.timestamp));
                        println!("   Parents: {}", node.parent_ids.join(", "));
                    },
                    icn_ledger::NodeData::ProposalExecuted { proposal_id, success } => {
                        println!("⚙️ Proposal Executed [{}]", node.id);
                        println!("   ID: {}", proposal_id);
                        println!("   Success: {}", success);
                        println!("   Time: {}", format_time(node.timestamp));
                        println!("   Parents: {}", node.parent_ids.join(", "));
                    },
                    _ => {
                        println!("📄 Other Node [{}]", node.id);
                        println!("   Type: {:?}", node.data);
                        println!("   Time: {}", format_time(node.timestamp));
                        println!("   Parents: {}", node.parent_ids.join(", "));
                    }
                }
                println!();
            }
        } else {
            println!("❌ No proposal node found for ID '{}'", proposal_id);
        }
    } else {
        println!("❌ DAG ledger not available in this VM instance");
    }
    
    Ok(())
}
