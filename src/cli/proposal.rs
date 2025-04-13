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
use clap::ArgMatches;
use clap::{arg, value_parser, Arg, ArgAction, Command};
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
/// - create: Create a new proposal
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
///
/// # Returns
/// A configured `Command` object ready to be used in a CLI application
pub fn proposal_command() -> Command {
    Command::new("proposal")
        .about("Manage governance proposal lifecycle")
        .subcommand_required(true)
        .arg_required_else_help(true)
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
                    Arg::new("creator")
                        .long("creator")
                        .value_name("ID")
                        .help("Identity ID of the proposal creator"),
                )
                .arg(
                    Arg::new("logic-path")
                        .long("logic-path")
                        .value_name("PATH")
                        .help("Path to the proposal logic"),
                )
                .arg(
                    Arg::new("expires-in")
                        .long("expires-in")
                        .value_name("DURATION")
                        .help("Duration until proposal expires (e.g., 7d, 24h)"),
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
                // Keep existing arguments for backward compatibility
                .arg(
                    Arg::new("title")
                        .long("title")
                        .value_name("STRING")
                        .help("Title of the proposal"),
                )
                .arg(
                    Arg::new("quorum")
                        .long("quorum")
                        .value_name("NUMBER")
                        .help("Quorum required for the proposal to pass (e.g., number of votes)")
                        .value_parser(value_parser!(u64)),
                )
                .arg(
                    Arg::new("threshold")
                        .long("threshold")
                        .value_name("NUMBER")
                        .help("Threshold required for the proposal to pass (e.g., percentage or fixed number)")
                        .value_parser(value_parser!(u64)),
                )
                .arg(
                    Arg::new("discussion-duration")
                        .long("discussion-duration")
                        .value_name("DURATION")
                        .help("Optional duration for the feedback/discussion phase (e.g., 7d, 48h)"),
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
                    Arg::new("text")
                        .long("text")
                        .value_name("TEXT")
                        .help("Text content of the comment")
                        .required(true)
                )
                .arg(
                    Arg::new("reply-to")
                        .long("reply-to")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to reply to")
                        // Not required, optional for replies
                        // No value_parser needed for String
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
                    Arg::new("choice")
                        .long("choice")
                        .value_name("VOTE")
                        .help("Your vote choice (yes, no, or abstain)")
                        .required(true)
                        .value_parser(value_parser!(VoteChoice))
                )
                // TODO: Add identity/signing argument if needed
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
                    Arg::new("status")
                        .long("status")
                        .value_name("STATUS")
                        .help("New status: deliberation, active, voting, executed, rejected, expired")
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
                .about("Simulate the execution of a proposal without making persistent changes")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to simulate")
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
}

/// Helper function to load a proposal from storage
///
/// Retrieves a proposal's lifecycle information from storage using its ID.
///
/// # Parameters
/// * `vm` - The virtual machine with access to storage
/// * `proposal_id` - The ID of the proposal to load
///
/// # Returns
/// * `Result<ProposalLifecycle, Box<dyn Error>>` - The loaded proposal on success, or an error
///
/// # Errors
/// Returns an error if:
/// * Storage backend is not configured
/// * Proposal can't be found
/// * Deserialization fails
pub fn load_proposal<S>(
    vm: &VM<S>,
    proposal_id: &ProposalId,
) -> Result<ProposalLifecycle, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm
        .storage_backend
        .as_ref()
        .ok_or_else(|| "Storage backend not configured for load_proposal")?;
    let namespace = "governance";
    let key = format!("governance/proposals/{}/lifecycle", proposal_id);
    // Need to handle potential deserialization issues if ProposalLifecycle still expects u64 ID
    storage
        .get_json::<ProposalLifecycle>(vm.auth_context.as_ref(), namespace, &key)
        .map_err(|e| format!("Failed to load proposal {} lifecycle: {}", proposal_id, e).into())
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
fn did_to_identity(did: &str) -> Identity {
    // Create a basic Identity with just the DID and default values
    Identity::new(did.to_string(), None, "member".to_string(), None)
        .expect("Failed to create identity from DID")
}

/// Parses DSL code from a file or storage path
///
/// Loads DSL code from either a filesystem path or a storage path,
/// then parses it into a vector of operations.
///
/// # Parameters
/// * `vm` - The virtual machine with access to storage
/// * `path` - Path to the DSL content, either a filesystem path or a storage path
///
/// # Returns
/// * `Result<Vec<Op>, Box<dyn Error>>` - The parsed operations on success, or an error
///
/// # Errors
/// Returns an error if:
/// * Storage backend is not configured
/// * File can't be read
/// * Content can't be parsed as DSL
fn parse_dsl_from_file<S>(vm: &VM<S>, path: &str) -> Result<Vec<Op>, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm
        .storage_backend
        .as_ref()
        .ok_or_else(|| "Storage backend not configured for loading logic")?;

    // Check if this is a storage path or filesystem path
    let contents = if path.starts_with("governance/") {
        // It's a storage path - load from storage
        let auth_context = vm.auth_context.as_ref();
        match storage.get(auth_context, "governance", path) {
            Ok(bytes) => String::from_utf8(bytes)
                .map_err(|e| format!("Invalid UTF-8 in stored logic: {}", e))?,
            Err(e) => return Err(format!("Failed to load logic from storage: {}", e).into()),
        }
    } else {
        // It's a filesystem path - load from file
        match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to read logic file {}: {}", path, e).into()),
        }
    };

    // Parse the DSL content
    match parse_dsl(&contents) {
        Ok(ops) => Ok(ops),
        Err(e) => Err(format!("Failed to parse DSL: {}", e).into()),
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

    match matches.subcommand() {
        Some(("create", create_matches)) => {
            // Check for new command format
            if let Some(id) = create_matches.get_one::<String>("id") {
                let creator = create_matches
                    .get_one::<String>("creator")
                    .map(|s| s.clone())
                    .unwrap_or_else(|| user_did.to_string());
                let logic_path = create_matches.get_one::<String>("logic-path").cloned();
                let discussion_path = create_matches.get_one::<String>("discussion-path").cloned();

                // Handle expires-in parameter
                let expires_at =
                    if let Some(expires_in) = create_matches.get_one::<String>("expires-in") {
                        // Simple implementation - assume values like "7d", "24h", etc.
                        let duration = parse_duration_string(expires_in)?;
                        Some(Utc::now() + duration)
                    } else {
                        None
                    };

                // Parse attachments if provided
                let attachments = if let Some(attachments_str) =
                    create_matches.get_one::<String>("attachments")
                {
                    attachments_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                } else {
                    Vec::new()
                };

                // Create the new proposal
                let mut proposal = Proposal::new(
                    id.clone(),
                    creator,
                    logic_path,
                    expires_at,
                    discussion_path,
                    attachments,
                );

                // Set minimum deliberation hours if provided
                if let Some(min_hours) = create_matches.get_one::<i64>("min-deliberation") {
                    proposal.min_deliberation_hours = Some(*min_hours);
                }

                // Store the proposal using JSON storage API
                let storage = vm
                    .storage_backend
                    .as_mut()
                    .ok_or_else(|| "Storage backend not configured for proposal creation")?;

                storage.set_json(
                    Some(auth_context),
                    "governance",
                    &proposal.storage_key(),
                    &proposal,
                )?;

                println!("Proposal {} created successfully.", id);
                return Ok(());
            }

            // Existing code for other format
            println!("Handling proposal create...");
            // 1. Parse args
            let title = create_matches.get_one::<String>("title").unwrap();
            let quorum = *create_matches.get_one::<u64>("quorum").unwrap();
            let threshold = *create_matches.get_one::<u64>("threshold").unwrap();

            // Parse optional args
            let discussion_duration_str = create_matches.get_one::<String>("discussion-duration");
            let required_participants = create_matches
                .get_one::<u64>("required-participants")
                .copied(); // Get Option<u64>

            // Parse duration string (need helper)
            let discussion_duration = discussion_duration_str
                .map(|s| parse_duration_string(s))
                .transpose() // Convert Result<Option<Duration>, _> to Option<Result<Duration, _>> then handle error?
                .map_err(|e| format!("Invalid discussion duration: {}", e))?;

            // Use user_did for ID generation
            let timestamp_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let mut hasher = Sha256::new();
            hasher.update(user_did.as_bytes()); // Use DID
            hasher.update(title.as_bytes());
            hasher.update(&timestamp_nanos.to_le_bytes());
            let hash_result = hasher.finalize();
            let proposal_id = hex::encode(&hash_result[..16]);

            println!("Generated Proposal ID: {}", proposal_id);

            // 3. Create ProposalLifecycle instance
            let proposal = ProposalLifecycle::new(
                proposal_id.clone(),
                did_to_identity(user_did), // Convert DID string to Identity
                title.clone(),
                quorum,
                threshold,
                discussion_duration,
                required_participants,
            );

            // 4. Get storage backend MUTABLY for set_json
            let storage = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal create")?;

            // 5. Store lifecycle object
            // Assuming lifecycle is stored in "governance" namespace
            let namespace = "governance";
            let key = format!("governance/proposals/{}/lifecycle", proposal_id);
            storage.set_json(Some(auth_context), namespace, &key, &proposal)?;
            println!("Proposal {} lifecycle stored.", proposal_id);

            // 6. Emit reputation hook
            let rep_dsl = format!(
                "increment_reputation \"{}\" reason=\"Created proposal {}\"",
                user_did, proposal_id
            );
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;

            println!("Proposal {} created with title '{}'.", proposal_id, title);
            // Explicitly print the ID for easy copying
            println!("Proposal ID: {}", proposal_id);
        }
        Some(("attach", attach_matches)) => {
            println!("Handling proposal attach...");
            let proposal_id = attach_matches.get_one::<ProposalId>("id").unwrap().clone(); // Clone String ID
            let file_path = attach_matches.get_one::<PathBuf>("file").unwrap();
            let attachment_name_opt = attach_matches.get_one::<String>("name");

            if !file_path.exists() || !file_path.is_file() {
                return Err(format!(
                    "Attachment file not found or is not a file: {:?}",
                    file_path
                )
                .into());
            }
            let file_content_bytes = fs::read(file_path)?;

            let attachment_name = attachment_name_opt.map(|s| s.clone()).unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "attachment".to_string())
            });
            let sanitized_attachment_name = attachment_name.replace('/', "_").replace('\\', "_");

            // Store attachment bytes directly using storage trait
            let storage = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal attach")?;

            // Assuming attachments stored in "governance" namespace
            let namespace = "governance";
            let key = format!(
                "governance/proposals/{}/attachments/{}",
                proposal_id, sanitized_attachment_name
            );

            storage.set(Some(auth_context), namespace, &key, file_content_bytes)?;
            println!(
                "Attachment '{}' stored directly for proposal {}.",
                sanitized_attachment_name, proposal_id
            );

            // Emit reputation hook
            let rep_dsl = format!(
                "increment_reputation \"{}\" reason=\"Attached {} to proposal {}\"",
                user_did, sanitized_attachment_name, proposal_id
            );
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("comment", comment_matches)) => {
            println!("Adding comment to proposal...");
            let proposal_id = comment_matches.get_one::<String>("id").unwrap().clone();
            let comment_text = comment_matches.get_one::<String>("text").unwrap().clone();
            let reply_to = comment_matches.get_one::<String>("reply-to").cloned();

            // Get tags if provided
            let tags = if let Some(tag_values) = comment_matches.get_many::<String>("tag") {
                tag_values.cloned().collect()
            } else {
                Vec::new()
            };

            // Create a new comment with tags
            let comment_id = uuid::Uuid::new_v4().to_string();
            let comment = ProposalComment {
                id: comment_id.clone(),
                author: user_did.to_string(),
                timestamp: Utc::now(),
                content: comment_text.clone(),
                reply_to: reply_to.clone(),
                tags,
                reactions: HashMap::new(),
            };

            // Get storage backend
            let storage = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for adding comment")?;

            // First, verify the proposal exists
            let namespace = "governance";
            let proposal_key = format!("governance/proposals/{}", proposal_id);

            let proposal: Proposal = storage
                .get_json(Some(auth_context), namespace, &proposal_key)
                .map_err(|_| format!("Proposal with ID {} not found", proposal_id))?;

            // Store the comment
            let comment_key = format!(
                "governance/proposals/{}/comments/{}",
                proposal_id, comment_id
            );
            storage.set_json(Some(auth_context), namespace, &comment_key, &comment)?;

            println!("Added comment {} to proposal {}", comment_id, proposal_id);

            // Award reputation for contribution
            let rep_dsl = format!(
                "increment_reputation \"{}\" reason=\"Commented on proposal {}\"",
                user_did, proposal_id
            );
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("view", view_matches)) => {
            let proposal_id = view_matches.get_one::<String>("id").unwrap();
            return handle_view_command(vm, proposal_id);
        }
        Some(("edit", edit_matches)) => {
            println!("Handling proposal edit...");
            // 1. Parse args
            let proposal_id = edit_matches.get_one::<ProposalId>("id").unwrap().clone(); // Clone String ID
            let new_body_path = edit_matches.get_one::<PathBuf>("new-body");
            let new_logic_path = edit_matches.get_one::<PathBuf>("new-logic");

            // 2. Load proposal
            let mut proposal = load_proposal(vm, &proposal_id)?;

            // 3. Check state
            if !matches!(
                proposal.state,
                ProposalState::Draft | ProposalState::OpenForFeedback
            ) {
                return Err(format!(
                    "Proposal {} cannot be edited in its current state: {:?}",
                    proposal_id, proposal.state
                )
                .into());
            }
            // Check permissions using DID
            if proposal.creator.did != user_did {
                return Err(format!(
                    "User {} does not have permission to edit proposal {}",
                    user_did, proposal_id
                )
                .into());
            }

            let mut edited = false;
            let namespace = "governance"; // Namespace for attachments

            // Get mutable storage backend reference once
            let storage = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal edit")?;

            // 4. Handle new body
            if let Some(path) = new_body_path {
                println!("Updating body from {:?}...", path);
                if !path.exists() || !path.is_file() {
                    return Err(format!("New body file not found: {:?}", path).into());
                }
                let content_bytes = fs::read(path)?;
                let key = format!("governance/proposals/{}/attachments/body", proposal_id);
                // Assuming common attachment names like "body.md" or "body"
                storage.set(Some(auth_context), namespace, &key, content_bytes)?;
                edited = true;
            }

            // 5. Handle new logic
            if let Some(path) = new_logic_path {
                println!("Updating logic from {:?}...", path);
                if !path.exists() || !path.is_file() {
                    return Err(format!("New logic file not found: {:?}", path).into());
                }
                let content_bytes = fs::read(path)?;
                let key = format!("governance/proposals/{}/attachments/logic", proposal_id);
                // Assuming common attachment names like "logic.ccl" or "logic"
                storage.set(Some(auth_context), namespace, &key, content_bytes)?;
                edited = true;
            }

            if edited {
                // 6. & 7. Update version and potentially state
                proposal.update_version(); // Call the lifecycle method
                                           // Decide if state should change, e.g., back to Draft
                                           // proposal.state = ProposalState::Draft;
                proposal.history.push((Utc::now(), proposal.state.clone())); // Record the edit/version bump

                // 8. Save updated lifecycle
                let lifecycle_key = format!("governance/proposals/{}/lifecycle", proposal_id);
                storage.set_json(Some(auth_context), namespace, &lifecycle_key, &proposal)?;
                println!(
                    "Proposal {} edited. New version: {}.",
                    proposal_id, proposal.current_version
                );

                // 9. Emit reputation hook
                let rep_dsl = format!(
                    "increment_reputation \"{}\" reason=\"Edited proposal {}\"",
                    user_did, proposal_id
                );
                let ops = parse_dsl(&rep_dsl)?;
                vm.execute(&ops)?;
            } else {
                println!("No changes specified for proposal {}.", proposal_id);
            }
        }
        Some(("publish", publish_matches)) => {
            println!("Handling proposal publish...");
            let proposal_id = publish_matches.get_one::<ProposalId>("id").unwrap().clone(); // Clone String ID

            let mut proposal = load_proposal(vm, &proposal_id)?;
            proposal.open_for_feedback(); // Call the state transition method

            // Save the updated proposal
            let storage = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal publish")?;
            let namespace = "governance";
            let key = format!("governance/proposals/{}/lifecycle", proposal_id);
            // Use direct method call
            storage.set_json(Some(auth_context), namespace, &key, &proposal)?;
            println!(
                "Proposal {} published (state: {:?}).",
                proposal_id, proposal.state
            );

            // TODO: Add reputation hook?
        }
        Some(("vote", vote_matches)) => {
            println!("Handling proposal vote...");
            let proposal_id = vote_matches.get_one::<ProposalId>("id").unwrap().clone();
            let choice_enum = vote_matches
                .get_one::<VoteChoice>("choice")
                .unwrap()
                .clone();

            let choice_str = match choice_enum {
                VoteChoice::Yes => "yes",
                VoteChoice::No => "no",
                VoteChoice::Abstain => "abstain",
            }
            .to_string();

            let storage_ref_mut = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal vote")?;
            let namespace = "governance";
            let key = format!("governance/proposals/{}/votes/{}", proposal_id, user_did); // Use DID
            storage_ref_mut.set(
                Some(auth_context),
                namespace,
                &key,
                choice_str.clone().into_bytes(),
            )?;
            println!(
                "Vote '{}' recorded for proposal {} by {}.",
                choice_str, proposal_id, user_did
            );

            let mut proposal = load_proposal(vm, &proposal_id)?;

            if let Err(e) = proposal.transition_to_executed(vm, Some(auth_context)) {
                eprintln!(
                    "Error during execution check/transition for proposal {}: {}",
                    proposal_id, e
                );
            }

            let rep_dsl = format!(
                "increment_reputation \"{}\" reason=\"Voted on proposal {}\"",
                user_did, proposal_id
            );
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("transition", transition_matches)) => {
            println!("Handling proposal transition...");
            let proposal_id = transition_matches.get_one::<String>("id").unwrap().clone();
            let status_str = transition_matches
                .get_one::<String>("status")
                .unwrap()
                .clone();
            let result = transition_matches.get_one::<String>("result").cloned();
            let force = transition_matches.get_flag("force");

            // Get storage backend
            let storage = vm
                .storage_backend
                .as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal transition")?;

            // Load the proposal
            let namespace = "governance";
            let key = format!("governance/proposals/{}", proposal_id);

            let mut proposal: Proposal = storage.get_json(Some(auth_context), namespace, &key)?;

            // Check permissions - only creator or admin can transition
            if proposal.creator != user_did && !auth_context.has_role("governance", "admin") {
                return Err(format!(
                    "User {} does not have permission to transition proposal {}",
                    user_did, proposal_id
                )
                .into());
            }

            // Apply transition based on the status string
            match status_str.to_lowercase().as_str() {
                "deliberation" => {
                    if !matches!(proposal.status, ProposalStatus::Draft) && !force {
                        return Err(format!(
                            "Cannot transition proposal from {:?} to Deliberation without --force flag",
                            proposal.status
                        ).into());
                    }
                    proposal.mark_deliberation();
                }
                "active" => {
                    if matches!(proposal.status, ProposalStatus::Deliberation) {
                        let started_at = proposal
                            .deliberation_started_at
                            .ok_or("Missing deliberation start timestamp")?;
                        let now = Utc::now();
                        let elapsed = now.signed_duration_since(started_at);
                        let min_required = proposal
                            .min_deliberation_hours
                            .unwrap_or(MIN_DELIBERATION_HOURS);

                        if elapsed.num_hours() < min_required && !force {
                            return Err(format!(
                                "Deliberation phase must last at least {} hours (elapsed: {}). Use --force to override.",
                                min_required,
                                elapsed.num_hours()
                            ).into());
                        }
                    } else if !matches!(proposal.status, ProposalStatus::Draft) && !force {
                        return Err(format!(
                            "Cannot transition proposal from {:?} to Active without --force flag",
                            proposal.status
                        )
                        .into());
                    }
                    proposal.mark_active();
                }
                "voting" => {
                    if !matches!(proposal.status, ProposalStatus::Active) && !force {
                        return Err(format!(
                            "Cannot transition proposal from {:?} to Voting without --force flag",
                            proposal.status
                        )
                        .into());
                    }
                    proposal.mark_voting();
                }
                "executed" => {
                    // Check if current status is Voting
                    if !matches!(proposal.status, ProposalStatus::Voting) && !force {
                        return Err(format!(
                            "Cannot execute proposal from {:?} state. Must be in Voting state or use --force flag.",
                            proposal.status
                        ).into());
                    }

                    // For custom result, use that instead of executing logic
                    if let Some(custom_result) = result {
                        proposal.mark_executed(custom_result);
                    } else {
                        // Try to execute logic if available
                        if let Some(logic_path) = &proposal.logic_path.clone() {
                            println!("Executing proposal logic from: {}", logic_path);

                            // First, save the proposal with updated status
                            storage.set_json(Some(auth_context), namespace, &key, &proposal)?;

                            // Clone the path first to avoid borrowing issues
                            let logic_path_clone = logic_path.clone();

                            // Get the logic content directly
                            let logic_content = match storage.get(
                                Some(auth_context),
                                "governance",
                                &logic_path_clone,
                            ) {
                                Ok(bytes) => match String::from_utf8(bytes) {
                                    Ok(content) => content,
                                    Err(e) => {
                                        let error_msg =
                                            format!("Invalid UTF-8 in logic file: {}", e);
                                        println!("{}", error_msg);
                                        proposal.mark_executed(error_msg);
                                        storage.set_json(
                                            Some(auth_context),
                                            namespace,
                                            &key,
                                            &proposal,
                                        )?;
                                        return Ok(());
                                    }
                                },
                                Err(e) => {
                                    let error_msg = format!("Failed to read logic file: {}", e);
                                    println!("{}", error_msg);
                                    proposal.mark_executed(error_msg);
                                    storage.set_json(
                                        Some(auth_context),
                                        namespace,
                                        &key,
                                        &proposal,
                                    )?;
                                    return Ok(());
                                }
                            };

                            // Parse the DSL directly
                            let ops = match parse_dsl(&logic_content) {
                                Ok(ops) => ops,
                                Err(e) => {
                                    let error_msg =
                                        format!("Failed to parse proposal logic: {}", e);
                                    println!("{}", error_msg);
                                    proposal.mark_executed(error_msg);
                                    storage.set_json(
                                        Some(auth_context),
                                        namespace,
                                        &key,
                                        &proposal,
                                    )?;
                                    return Ok(());
                                }
                            };

                            // Store temporary variables for what we need after vm.execute
                            let proposal_id_for_result = proposal_id.clone();
                            let logic_path_for_result = logic_path.clone();

                            // Release the storage borrow before executing
                            // We no longer need the storage reference until after execute
                            let _ = storage;

                            // Execute the operations
                            let execution_result = match vm.execute(&ops) {
                                Ok(_) => format!(
                                    "Successfully executed logic at {}",
                                    logic_path_for_result
                                ),
                                Err(e) => format!("Logic execution failed: {}", e),
                            };

                            println!("Execution result: {}", execution_result);

                            // Get a fresh storage reference
                            let storage = vm.storage_backend.as_mut().ok_or_else(|| {
                                "Storage backend not configured for proposal execution"
                            })?;

                            // Reload the proposal (it might have been modified during execution)
                            let mut updated_proposal: Proposal =
                                match storage.get_json(Some(auth_context), namespace, &key) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        let error_msg = format!(
                                            "Failed to reload proposal after execution: {}",
                                            e
                                        );
                                        println!("{}", error_msg);
                                        // Create a fresh proposal as fallback (we can't use the old one since we dropped it)
                                        let mut p = Proposal::new(
                                            proposal_id_for_result,
                                            user_did.to_string(),
                                            Some(logic_path_for_result),
                                            None,
                                            None,
                                            Vec::new(),
                                        );
                                        p.mark_executed(format!(
                                            "{} - {}",
                                            execution_result, error_msg
                                        ));
                                        p
                                    }
                                };

                            updated_proposal.mark_executed(execution_result);

                            // Save again with the execution result
                            storage.set_json(
                                Some(auth_context),
                                namespace,
                                &key,
                                &updated_proposal,
                            )?;

                            // Early return since we've already saved
                            return Ok(());
                        } else {
                            // No logic path provided
                            let msg = "No logic path defined for proposal.".to_string();
                            println!("{}", msg);
                            proposal.mark_executed(msg);
                        }
                    }
                }
                "rejected" => {
                    if !matches!(proposal.status, ProposalStatus::Voting) && !force {
                        return Err(format!(
                            "Cannot reject proposal from {:?} state. Must be in Voting state or use --force flag.",
                            proposal.status
                        ).into());
                    }
                    proposal.mark_rejected();
                }
                "expired" => proposal.mark_expired(),
                _ => return Err(format!("Invalid status: {}", status_str).into()),
            }

            // Save the updated proposal
            storage.set_json(Some(auth_context), namespace, &key, &proposal)?;

            println!(
                "Proposal {} transitioned to {} status.",
                proposal_id, status_str
            );

            // Emit reputation hook
            let rep_dsl = format!(
                "increment_reputation \"{}\" reason=\"Transitioned proposal {}\"",
                user_did, proposal_id
            );
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("list", list_matches)) => {
            println!("Listing proposals...");

            // Get filter parameters
            let status_filter = list_matches
                .get_one::<String>("status")
                .map(|s| s.to_lowercase());
            let creator_filter = list_matches.get_one::<String>("creator").cloned();
            let limit = list_matches.get_one::<u32>("limit").copied().unwrap_or(100);

            // Get storage backend
            let storage = vm
                .storage_backend
                .as_ref()
                .ok_or_else(|| "Storage backend not configured for listing proposals")?;

            // List all proposal keys
            let namespace = "governance";
            let prefix = "governance/proposals/";
            let keys = storage.list_keys(vm.auth_context.as_ref(), namespace, Some(prefix))?;

            println!("--- Proposals ---");
            let mut count = 0;

            for key in keys {
                if count >= limit {
                    break;
                }

                // Try to load proposal
                match storage.get_json::<Proposal>(vm.auth_context.as_ref(), namespace, &key) {
                    Ok(proposal) => {
                        // Apply filters
                        let status_match = status_filter
                            .as_ref()
                            .map(|s| match_status(&proposal.status, s))
                            .unwrap_or(true);

                        let creator_match = creator_filter
                            .as_ref()
                            .map(|c| proposal.creator == *c)
                            .unwrap_or(true);

                        if status_match && creator_match {
                            // Display proposal summary
                            print_proposal_summary(&proposal);
                            count += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading proposal from {}: {}", key, e);
                    }
                }
            }

            if count == 0 {
                println!("No proposals found matching the criteria.");
            }
        }
        Some(("comments", comments_matches)) => {
            println!("Fetching comments for proposal...");
            let proposal_id = comments_matches.get_one::<String>("id").unwrap().clone();
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
            let proposal_id = simulate_matches.get_one::<String>("id").unwrap();
            return handle_simulate_command(vm, proposal_id);
        }
        Some(("summary", summary_matches)) => {
            let proposal_id = summary_matches.get_one::<String>("id").unwrap();
            return handle_summary_command(vm, proposal_id);
        }
        Some(("comment-react", react_matches)) => {
            let comment_id = react_matches.get_one::<String>("id").unwrap();
            let proposal_id = react_matches.get_one::<String>("proposal-id").unwrap();
            let reaction = react_matches.get_one::<String>("reaction").unwrap();

            return handle_comment_react_command(
                vm,
                comment_id,
                proposal_id,
                reaction,
                auth_context,
            );
        }
        Some(("comment-tag", tag_matches)) => {
            let comment_id = tag_matches.get_one::<String>("id").unwrap();
            let proposal_id = tag_matches.get_one::<String>("proposal-id").unwrap();
            let tags: Vec<String> = if let Some(tag_values) = tag_matches.get_many::<String>("tag")
            {
                tag_values.cloned().collect()
            } else {
                Vec::new()
            };

            return handle_comment_tag_command(vm, comment_id, proposal_id, &tags, auth_context);
        }
        Some(("thread", thread_matches)) => {
            let proposal_id = thread_matches.get_one::<String>("id").unwrap();
            let show_hidden = thread_matches.get_flag("show-hidden");

            return handle_thread_command(vm, proposal_id, show_hidden, auth_context);
        }
        Some(("comment-edit", edit_matches)) => {
            let comment_id = edit_matches.get_one::<String>("id").unwrap();
            let proposal_id = edit_matches.get_one::<String>("proposal-id").unwrap();
            let new_text = edit_matches.get_one::<String>("text").unwrap();

            return handle_comment_edit_command(
                vm,
                comment_id,
                proposal_id,
                new_text,
                auth_context,
            );
        }
        Some(("comment-hide", hide_matches)) => {
            let comment_id = hide_matches.get_one::<String>("id").unwrap();
            let proposal_id = hide_matches.get_one::<String>("proposal-id").unwrap();

            return handle_comment_hide_command(vm, comment_id, proposal_id, auth_context);
        }
        Some(("comment-history", history_matches)) => {
            let comment_id = history_matches.get_one::<String>("id").unwrap();
            let proposal_id = history_matches.get_one::<String>("proposal-id").unwrap();

            return handle_comment_history_command(vm, comment_id, proposal_id, Some(auth_context));
        }
        _ => unreachable!("Subcommand should be required"),
    }
    Ok(())
}

/// Parse a duration string into a chrono Duration
///
/// Parses strings like "7d", "24h", "30m", "60s" into corresponding Duration values.
///
/// # Parameters
/// * `duration_str` - A string representation of duration (e.g., "7d", "24h")
///
/// # Returns
/// * `Result<Duration, Box<dyn Error>>` - A chrono Duration on success, or an error
///
/// # Errors
/// Returns an error if:
/// * Format is invalid
/// * Unit is not one of d (days), h (hours), m (minutes), s (seconds)
///
/// # Examples
/// ```
/// let duration = parse_duration_string("7d")?; // 7 days
/// let duration = parse_duration_string("24h")?; // 24 hours
/// ```
fn parse_duration_string(duration_str: &str) -> Result<Duration, Box<dyn Error>> {
    // Get the numeric part and unit
    let (num_str, unit) = duration_str.split_at(duration_str.len() - 1);
    let num = num_str
        .parse::<i64>()
        .map_err(|_| format!("Invalid duration format: {}", duration_str))?;

    // Convert to Duration based on unit
    match unit {
        "d" => Ok(Duration::days(num)),
        "h" => Ok(Duration::hours(num)),
        "m" => Ok(Duration::minutes(num)),
        "s" => Ok(Duration::seconds(num)),
        _ => Err(format!("Invalid duration unit: {}. Expected d, h, m, or s", unit).into()),
    }
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
    // Load the proposal from storage
    let storage_key = format!("governance/proposals/{}", proposal_id);

    let proposal_data = vm
        .storage_backend
        .as_ref()
        .ok_or_else(|| VMError::StorageUnavailable)?
        .get(None, "proposals", &storage_key)
        .map_err(|e| {
            eprintln!("Failed to read proposal: {}", e);
            Box::new(e) as Box<dyn Error>
        })?;

    // Deserialize the proposal
    serde_json::from_slice::<Proposal>(&proposal_data).map_err(|e| {
        eprintln!("Failed to deserialize proposal: {}", e);
        Box::new(e) as Box<dyn Error>
    })
}

/// Count votes for a proposal from storage
pub fn count_votes<S>(
    vm: &VM<S>,
    proposal_id: &ProposalId,
) -> Result<(u32, u32, u32), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let votes_path = format!("votes/{}", proposal_id);
    let mut yes_votes = 0;
    let mut no_votes = 0;
    let mut abstain_votes = 0;

    // Try to list all files in the votes directory
    match vm
        .storage_backend
        .as_ref()
        .ok_or_else(|| VMError::StorageUnavailable)?
        .list_keys(None, "votes", Some(&votes_path))
    {
        Ok(voter_items) => {
            // Process each voter's vote
            for voter_item in voter_items {
                let voter_id = voter_item.split('/').last().unwrap_or_default();
                let vote_key = format!("{}/{}", votes_path, voter_id);

                match vm
                    .storage_backend
                    .as_ref()
                    .unwrap()
                    .get(None, "votes", &vote_key)
                {
                    Ok(vote_data) => {
                        // Try to deserialize as VoteChoice
                        if let Ok(vote) = serde_json::from_slice::<VoteChoice>(&vote_data) {
                            match vote {
                                VoteChoice::Yes => yes_votes += 1,
                                VoteChoice::No => no_votes += 1,
                                VoteChoice::Abstain => abstain_votes += 1,
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read vote from {}: {}", voter_id, e);
                        // Continue with other votes
                    }
                }
            }
        }
        Err(e) => {
            // If directory doesn't exist, it might mean no votes yet
            if let StorageError::NotFound { .. } = e {
                // This is fine - no votes yet
                println!("No votes found for proposal {}", proposal_id);
            } else {
                // Other errors should be reported
                eprintln!("Error accessing votes: {}", e);
                return Err(Box::new(e));
            }
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
        .storage_backend
        .as_ref()
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
        .filter(|c| c.reply_to.as_deref() == Some(&comment.id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::implementations::in_memory::InMemoryStorage;
    use crate::storage::traits::{Storage, StorageBackend, StorageExtensions};
    use chrono::Duration;

    // Helper function to create a test VM
    fn setup_test_vm() -> VM<InMemoryStorage> {
        let storage = InMemoryStorage::new();
        VM::with_storage_backend(storage)
    }

    // Helper function to create a test auth context
    fn setup_test_auth() -> AuthContext {
        AuthContext {
            current_identity_did: "test_user_1".to_string(),
            identity_registry: HashMap::new(),
            roles: HashMap::new(),
            memberships: Vec::new(),
            delegations: Vec::new(),
        }
    }

    // Helper function for storage operations in tests
    fn test_storage_set(
        storage: &mut InMemoryStorage,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        // Direct use of the storage traits
        <InMemoryStorage as StorageBackend>::set(storage, auth, namespace, key, data)?;
        Ok(())
    }

    // Helper function for storage operations in tests
    fn test_storage_get(
        storage: &InMemoryStorage,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        // Direct use of the storage traits
        let data = <InMemoryStorage as StorageBackend>::get(storage, auth, namespace, key)?;
        Ok(data)
    }

    /// Create a test proposal with sample data
    fn create_test_proposal(
        vm: &mut VM<InMemoryStorage>,
        proposal_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Create a basic proposal
        let storage_key = format!("governance/proposals/{}", proposal_id);

        let proposal = Proposal {
            id: proposal_id.to_string(),
            creator: "test_creator".to_string(),
            status: LocalProposalStatus::Draft,
            created_at: Utc::now(),
            expires_at: None,
            logic_path: Some("test_logic.dsl".to_string()),
            discussion_path: Some("test_discussion.dsl".to_string()),
            votes_path: Some(format!("votes/{}", proposal_id)),
            attachments: Vec::new(),
            execution_result: None,
            deliberation_started_at: None,
            min_deliberation_hours: None,
        };

        let proposal_data = serde_json::to_vec(&proposal)?;

        // Set the proposal data directly
        vm.storage_backend
            .as_mut()
            .unwrap()
            .set(None, "proposals", &storage_key, proposal_data)?;

        // Also create a lifecycle
        let lifecycle_key = format!("proposals/{}", proposal_id);

        let lifecycle = ProposalLifecycle::new(
            proposal_id.to_string(),
            Identity::new("test_creator".to_string(), None, "member".to_string(), None)?,
            "Test Proposal Title".to_string(),
            10, // quorum
            51, // threshold
            Some(Duration::days(7)),
            Some(5), // required_participants
        );

        let lifecycle_data = serde_json::to_vec(&lifecycle)?;

        vm.storage_backend.as_mut().unwrap().set(
            None,
            "proposals",
            &lifecycle_key,
            lifecycle_data,
        )?;

        // Create test logic
        let test_logic = "SETSTORE key value\nACTIVATE id";
        let logic_key = "test_logic.dsl";

        vm.storage_backend.as_mut().unwrap().set(
            None,
            "proposals",
            logic_key,
            test_logic.as_bytes().to_vec(),
        )?;

        Ok(())
    }

    // Test adding and retrieving a comment with tags
    #[test]
    fn test_comment_with_tags() -> Result<(), Box<dyn Error>> {
        let mut vm = setup_test_vm();
        let auth = setup_test_auth();
        let proposal_id = "test-proposal";
        let comment_id = "comment1";

        // Create test proposal
        create_test_proposal(&mut vm, proposal_id)?;

        // Create a comment with tags
        let comment_key = format!("comments/{}/{}", proposal_id, comment_id);

        let comment = ProposalComment {
            id: comment_id.to_string(),
            author: auth.current_identity_did.clone(),
            timestamp: Utc::now(),
            content: "This is a test comment".to_string(),
            reply_to: None,
            tags: vec!["test".to_string(), "feature".to_string()],
            reactions: HashMap::new(),
        };

        let comment_data = serde_json::to_vec(&comment)?;

        vm.storage_backend.as_mut().unwrap().set(
            Some(&auth),
            "comments",
            &comment_key,
            comment_data,
        )?;

        // Retrieve the comment
        let retrieved_data =
            vm.storage_backend
                .as_ref()
                .unwrap()
                .get(Some(&auth), "comments", &comment_key)?;

        let retrieved_comment: ProposalComment = serde_json::from_slice(&retrieved_data)?;

        // Verify tags are present
        assert_eq!(retrieved_comment.tags.len(), 2);
        assert!(retrieved_comment.tags.contains(&"test".to_string()));
        assert!(retrieved_comment.tags.contains(&"feature".to_string()));

        Ok(())
    }

    // Test adding reactions to a comment
    #[test]
    fn test_comment_reactions() -> Result<(), Box<dyn Error>> {
        let mut vm = setup_test_vm();
        let auth = setup_test_auth();
        let proposal_id = "test-proposal";
        let comment_id = "comment1";

        // Create test proposal
        create_test_proposal(&mut vm, proposal_id)?;

        // Create a comment with reactions
        let comment_key = format!("comments/{}/{}", proposal_id, comment_id);

        let mut comment = ProposalComment {
            id: comment_id.to_string(),
            author: auth.current_identity_did.clone(),
            timestamp: Utc::now(),
            content: "This is a test comment for reactions".to_string(),
            reply_to: None,
            tags: Vec::new(),
            reactions: HashMap::new(),
        };

        // Add some reactions
        comment.reactions.insert("👍".to_string(), 1);
        comment.reactions.insert("❤️".to_string(), 2);

        let comment_data = serde_json::to_vec(&comment)?;

        vm.storage_backend.as_mut().unwrap().set(
            Some(&auth),
            "comments",
            &comment_key,
            comment_data,
        )?;

        // Add another reaction through the utility function
        handle_comment_react_command(&mut vm, comment_id, proposal_id, "👍", &auth)?;

        // Retrieve the comment
        let retrieved_data =
            vm.storage_backend
                .as_ref()
                .unwrap()
                .get(Some(&auth), "comments", &comment_key)?;

        let retrieved_comment: ProposalComment = serde_json::from_slice(&retrieved_data)?;

        // Verify reactions updated
        assert_eq!(retrieved_comment.reactions.get("👍"), Some(&2));
        assert_eq!(retrieved_comment.reactions.get("❤️"), Some(&2));

        Ok(())
    }

    // Test the simulation of a proposal
    #[test]
    fn test_proposal_simulation() -> Result<(), Box<dyn Error>> {
        let mut vm = setup_test_vm();
        let auth = setup_test_auth();
        let proposal_id = "test-proposal-3";

        // Create a test proposal with logic
        create_test_proposal(&mut vm, proposal_id)?;

        // Run simulation
        handle_simulate_command(&mut vm, proposal_id)?;

        // Verify the original VM wasn't modified
        // This is a basic test - in a real test we'd check more specific behavior

        Ok(())
    }

    #[test]
    fn test_comment_migration() -> Result<(), Box<dyn Error>> {
        let mut vm = setup_test_vm();
        let auth = setup_test_auth();
        let proposal_id = "test-proposal-migration";

        // Create a test proposal
        create_test_proposal(&mut vm, proposal_id)?;

        // Create an "old format" comment (without hidden or edit_history)
        let old_comment = ProposalComment {
            id: "test-comment-old".to_string(),
            author: auth.current_identity_did.clone(),
            timestamp: Utc::now(),
            content: "This is an old-format comment".to_string(),
            reply_to: None,
            tags: vec!["legacy".to_string()],
            reactions: HashMap::new(),
        };

        // Store the old-format comment
        let comment_key = format!(
            "governance/proposals/{}/comments/{}",
            proposal_id, old_comment.id
        );
        vm.storage_backend.as_mut().unwrap().set_json(
            Some(&auth),
            "governance",
            &comment_key,
            &old_comment,
        )?;

        // Now try to fetch the comment using the new system
        // This should automatically convert/migrate it
        let migrated_comment =
            comments::get_comment(&vm, proposal_id, &old_comment.id, Some(&auth))?;

        // Verify the comment has been properly migrated with default values
        assert_eq!(migrated_comment.content, old_comment.content);
        assert_eq!(migrated_comment.author, old_comment.author);

        // Verify the new fields have appropriate default values
        assert_eq!(migrated_comment.hidden, false);
        assert!(migrated_comment.edit_history.len() > 0);
        assert_eq!(
            migrated_comment.edit_history[0].content,
            old_comment.content
        );

        Ok(())
    }
}
