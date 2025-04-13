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
use crate::governance::proposal_lifecycle::ExecutionStatus;
use crate::governance::proposal_lifecycle::VoteChoice;
use crate::governance::proposal_lifecycle::{Comment, ProposalLifecycle, ProposalState};
use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::Storage;
use crate::storage::traits::StorageExtensions;
use crate::vm::VM;
use crate::vm::Op;
use chrono::{DateTime, Duration, Utc};
use clap::ArgMatches;
use clap::{arg, value_parser, Arg, ArgAction, Command};
use hex;
use serde_json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::boxed::Box;
use std::str::FromStr;
use crate::governance::proposal::{Proposal, ProposalStatus};
use serde::{Serialize, Deserialize};
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
                        .value_name("COMMENT_TEXT")
                        .help("The text of the comment")
                        .required(true)
                )
                .arg(
                    Arg::new("reply-to")
                        .long("reply-to")
                        .value_name("COMMENT_ID")
                        .help("ID of the comment to reply to")
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
                        // No value_parser needed for String
                )
                .arg(
                    Arg::new("new-body")
                        .long("new-body")
                        .value_name("FILE_PATH")
                        .help("Path to the new proposal body file (e.g., updated markdown)")
                        .value_parser(value_parser!(PathBuf)), // Not required
                )
                .arg(
                    Arg::new("new-logic")
                        .long("new-logic")
                        .value_name("FILE_PATH")
                        .help("Path to the new proposal logic file (e.g., updated CCL script)")
                        .value_parser(value_parser!(PathBuf)), // Not required
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
                        // No value_parser needed for String
        )
        .arg(
                    Arg::new("choice")
                        .long("choice")
                        .value_name("VOTE")
                        .help("Your vote choice (yes, no, or abstain)")
                .required(true)
                        .value_parser(value_parser!(VoteChoice)),
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
                .about("View the details and status of a proposal")
                 .arg(
                    Arg::new("id")
                        .long("id")
                        .value_name("PROPOSAL_ID")
                        .help("ID of the proposal to view")
                        .required(true)
                        // No value_parser needed for String
                )
                .arg(
                    Arg::new("version")
                        .long("version")
                        .value_name("VERSION_NUMBER")
                        .help("Optionally specify a version to view")
                        .value_parser(value_parser!(u64)), // Not required
                )
                .arg(
                    Arg::new("comments")
                        .long("comments")
                        .help("Flag to also view comments")
                        .action(ArgAction::SetTrue), // Not required
        )
        .arg(
                    Arg::new("history")
                        .long("history")
                        .help("Flag to also view history")
                        .action(ArgAction::SetTrue), // Not required
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
    let key = format!("proposals/{}/lifecycle", proposal_id);
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
fn parse_dsl_from_file<S>(
    vm: &VM<S>, 
    path: &str
) -> Result<Vec<Op>, Box<dyn Error>> 
where 
    S: Storage + Send + Sync + Clone + Debug + 'static
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
                let expires_at = if let Some(expires_in) = create_matches.get_one::<String>("expires-in") {
                    // Simple implementation - assume values like "7d", "24h", etc.
                    let duration = parse_duration_string(expires_in)?;
                    Some(Utc::now() + duration)
                } else {
                    None
                };
                
                // Parse attachments if provided
                let attachments = if let Some(attachments_str) = create_matches.get_one::<String>("attachments") {
                    attachments_str.split(',').map(|s| s.trim().to_string()).collect()
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
                let storage = vm.storage_backend.as_mut().ok_or_else(||
                    "Storage backend not configured for proposal creation")?;
                
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
            let key = format!("proposals/{}/lifecycle", proposal_id);
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
                "proposals/{}/attachments/{}",
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
            
            // Generate a unique comment ID
            let comment_id = format!("comment-{}", uuid::Uuid::new_v4().to_string());
            
            // Create the comment
            let comment = ProposalComment {
                id: comment_id.clone(),
                author: user_did.to_string(),
                timestamp: Utc::now(),
                content: comment_text,
                reply_to,
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
            let comment_key = format!("comments/{}/{}", proposal_id, comment_id);
            storage.set_json(
                Some(auth_context),
                namespace,
                &comment_key,
                &comment,
            )?;
            
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
            println!("Handling proposal view...");
            let proposal_id = view_matches.get_one::<ProposalId>("id").unwrap().clone();
            let specific_version = view_matches.get_one::<u64>("version").copied();
            let show_comments = view_matches.get_flag("comments");
            let show_history = view_matches.get_flag("history");

            let storage = vm
                .storage_backend
                .as_ref()
                .ok_or_else(|| "Storage backend not configured for proposal view")?;
            let namespace = "governance";

            // --- Load Proposal Lifecycle (specific version or latest) ---
            let proposal: ProposalLifecycle = if let Some(version) = specific_version {
                let key = format!("proposals/{}/lifecycle", proposal_id);
                storage
                    .get_version_json(vm.auth_context.as_ref(), namespace, &key, version)
                    .map_err(|e| {
                        format!(
                            "Failed to load version {} of proposal {}: {}",
                            version, proposal_id, e
                        )
                    })?
                    .ok_or_else(|| {
                        format!("Version {} not found for proposal {}", version, proposal_id)
                    })?
            } else {
                // Use existing helper for latest version
                load_proposal(vm, &proposal_id)?
            };

            // --- Print Core Details ---
            println!("--- Proposal Details ---");
            println!("ID:          {}", proposal.id);
            println!("Title:       {}", proposal.title);
            println!("Creator:     {:?}", proposal.creator); // Using debug format instead of Display
            println!("State:       {:?}", proposal.state);
            println!("Version:     {}", proposal.current_version);
            println!("Created At:  {}", proposal.created_at.to_rfc3339());
            if let Some(expires) = proposal.expires_at {
                println!("Expires At:  {}", expires.to_rfc3339());
            } else {
                println!("Expires At:  N/A");
            }
            println!("Quorum:      {}", proposal.quorum);
            println!("Threshold:   {}", proposal.threshold);

            // --- Vote Tally (if Voting) ---
            if proposal.state == ProposalState::Voting {
                // Get a clone of the storage reference to avoid borrowing vm
                let storage = vm
                    .storage_backend
                    .as_ref()
                    .ok_or_else(|| "Storage backend not configured for tallying votes")?;

                // Build a list of keys for votes
                let vote_prefix = format!("proposals/{}/votes/", proposal_id);
                let mut yes_votes = 0;
                let mut no_votes = 0;
                let mut abstain_votes = 0;

                // Count votes directly
                if let Ok(vote_keys) =
                    storage.list_keys(vm.auth_context.as_ref(), "governance", Some(&vote_prefix))
                {
                    for key in vote_keys {
                        if let Ok(vote_bytes) =
                            storage.get(vm.auth_context.as_ref(), "governance", &key)
                        {
                            if let Ok(vote_str) = String::from_utf8(vote_bytes) {
                                match vote_str.as_str() {
                                    "yes" => yes_votes += 1,
                                    "no" => no_votes += 1,
                                    "abstain" => abstain_votes += 1,
                                    _ => { /* Invalid vote */ }
                                }
                            }
                        }
                    }

                    // Build a votes map
                    let mut votes = HashMap::new();
                    votes.insert("yes".to_string(), yes_votes);
                    votes.insert("no".to_string(), no_votes);
                    votes.insert("abstain".to_string(), abstain_votes);

                    println!("--- Current Votes ---");
                    println!("  Yes:       {}", yes_votes);
                    println!("  No:        {}", no_votes);
                    println!("  Abstain:   {}", abstain_votes);

                    // Display quorum and threshold status
                    let total_votes = yes_votes + no_votes;
                    println!("  Quorum:    {}/{} votes", total_votes, proposal.quorum);
                    println!(
                        "  Threshold: {}/{} yes votes",
                        yes_votes, proposal.threshold
                    );
                } else {
                    println!("Error reading votes.");
                }
            }

            // --- List Attachments ---
            let attachment_prefix = format!("proposals/{}/attachments/", proposal.id);
            match storage.list_keys(
                vm.auth_context.as_ref(),
                namespace,
                Some(&attachment_prefix),
            ) {
                Ok(keys) if !keys.is_empty() => {
                    println!("--- Attachments ---");
                    for key in keys {
                        if let Some(name) = key.split('/').last() {
                            println!("  - {}", name);
                        }
                    }
                }
                Ok(_) => { /* No attachments found */ }
                Err(e) => {
                    println!("Error listing attachments: {}", e);
                }
            }

            // --- Execution Status ---
            if let Some(status) = &proposal.execution_status {
                println!("--- Execution Status ---");
                match status {
                    ExecutionStatus::Success => println!("  Status: Success"),
                    ExecutionStatus::Failure(reason) => println!("  Status: Failure - {}", reason),
                }
            }

            // --- History ---
            if show_history {
                println!("--- History ---");
                for (timestamp, state) in &proposal.history {
                    println!("  [{}] -> {:?}", timestamp.to_rfc3339(), state);
                }
            }

            // --- Comments ---
            if show_comments {
                println!("--- Comments ---");
                let comment_prefix = format!("proposals/{}/comments/", proposal.id);
                match storage.list_keys(vm.auth_context.as_ref(), namespace, Some(&comment_prefix))
                {
                    Ok(comment_keys) if !comment_keys.is_empty() => {
                        let mut all_comments = HashMap::new();
                        let mut root_comments = Vec::new();
                        let mut replies_map: HashMap<Option<CommentId>, Vec<CommentId>> =
                            HashMap::new();

                        for key in comment_keys {
                            match storage.get_json::<Comment>(
                                vm.auth_context.as_ref(),
                                namespace,
                                &key,
                            ) {
                                Ok(comment) => {
                                    let comment_id = comment.id.clone();
                                    let reply_to = comment.reply_to.clone();
                                    all_comments.insert(comment_id.clone(), comment);
                                    replies_map.entry(reply_to).or_default().push(comment_id);
                                }
                                Err(e) => println!("Error loading comment {}: {}", key, e),
                            }
                        }

                        // Find root comments (those not replying to anything)
                        if let Some(roots) = replies_map.get(&None) {
                            root_comments = roots.clone();
                            
                            // Sort root comments by timestamp (default)
                            root_comments.sort_by_key(|id| all_comments.get(id).map(|c| c.timestamp));
                        }

                        println!("\n--- Threaded Comments ---");
                        
                        if root_comments.is_empty() {
                            println!("No comments found.");
                        } else {
                            for root_id in root_comments {
                                print_threaded_comments(&root_id, &all_comments, &replies_map, 0);
                                println!(); // Add a blank line between top-level comments
                            }
                        }
                        
                        println!("Total comments: {}", all_comments.len());
                    }
                    Ok(_) => {
                        println!("No comments found.");
                    }
                    Err(e) => {
                        println!("Error listing comments: {}", e);
                    }
                }
            }
            println!("-----------------------");
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
                let key = format!("proposals/{}/attachments/body", proposal_id);
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
                let key = format!("proposals/{}/attachments/logic", proposal_id);
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
                let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
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
            let key = format!("proposals/{}/lifecycle", proposal_id);
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
            let key = format!("proposals/{}/votes/{}", proposal_id, user_did); // Use DID
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
            let status_str = transition_matches.get_one::<String>("status").unwrap().clone();
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
            
            let mut proposal: Proposal = storage
                .get_json(Some(auth_context), namespace, &key)?;
            
            // Check permissions - only creator or admin can transition
            if proposal.creator != user_did && !auth_context.has_role("governance", "admin") {
                return Err(format!(
                    "User {} does not have permission to transition proposal {}",
                    user_did, proposal_id
                ).into());
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
                },
                "active" => {
                    if matches!(proposal.status, ProposalStatus::Deliberation) {
                        let started_at = proposal.deliberation_started_at.ok_or("Missing deliberation start timestamp")?;
                        let now = Utc::now();
                        let elapsed = now.signed_duration_since(started_at);
                        let min_required = proposal.min_deliberation_hours.unwrap_or(MIN_DELIBERATION_HOURS);
                        
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
                        ).into());
                    }
                    proposal.mark_active();
                },
                "voting" => {
                    if !matches!(proposal.status, ProposalStatus::Active) && !force {
                        return Err(format!(
                            "Cannot transition proposal from {:?} to Voting without --force flag",
                            proposal.status
                        ).into());
                    }
                    proposal.mark_voting();
                },
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
                            storage.set_json(
                                Some(auth_context),
                                namespace,
                                &key,
                                &proposal,
                            )?;
                            
                            // Clone the path first to avoid borrowing issues
                            let logic_path_clone = logic_path.clone();
                            
                            // Get the logic content directly
                            let logic_content = match storage.get(Some(auth_context), "governance", &logic_path_clone) {
                                Ok(bytes) => match String::from_utf8(bytes) {
                                    Ok(content) => content,
                                    Err(e) => {
                                        let error_msg = format!("Invalid UTF-8 in logic file: {}", e);
                                        println!("{}", error_msg);
                                        proposal.mark_executed(error_msg);
                                        storage.set_json(Some(auth_context), namespace, &key, &proposal)?;
                                        return Ok(());
                                    }
                                },
                                Err(e) => {
                                    let error_msg = format!("Failed to read logic file: {}", e);
                                    println!("{}", error_msg);
                                    proposal.mark_executed(error_msg);
                                    storage.set_json(Some(auth_context), namespace, &key, &proposal)?;
                                    return Ok(());
                                }
                            };
                            
                            // Parse the DSL directly
                            let ops = match parse_dsl(&logic_content) {
                                Ok(ops) => ops,
                                Err(e) => {
                                    let error_msg = format!("Failed to parse proposal logic: {}", e);
                                    println!("{}", error_msg);
                                    proposal.mark_executed(error_msg);
                                    storage.set_json(Some(auth_context), namespace, &key, &proposal)?;
                                    return Ok(());
                                }
                            };
                            
                            // Store temporary variables for what we need after vm.execute
                            let proposal_id_for_result = proposal_id.clone();
                            let logic_path_for_result = logic_path.clone();
                            
                            // Release the storage borrow before executing
                            // We no longer need the storage reference until after execute
                            drop(storage);
                            
                            // Execute the operations
                            let execution_result = match vm.execute(&ops) {
                                Ok(_) => format!("Successfully executed logic at {}", logic_path_for_result),
                                Err(e) => format!("Logic execution failed: {}", e),
                            };
                            
                            println!("Execution result: {}", execution_result);
                            
                            // Get a fresh storage reference
                            let storage = vm
                                .storage_backend
                                .as_mut()
                                .ok_or_else(|| "Storage backend not configured for proposal execution")?;
                            
                            // Reload the proposal (it might have been modified during execution)
                            let mut updated_proposal: Proposal = match storage.get_json(Some(auth_context), namespace, &key) {
                                Ok(p) => p,
                                Err(e) => {
                                    let error_msg = format!("Failed to reload proposal after execution: {}", e);
                                    println!("{}", error_msg);
                                    // Create a fresh proposal as fallback (we can't use the old one since we dropped it)
                                    let mut p = Proposal::new(
                                        proposal_id_for_result,
                                        user_did.to_string(),
                                        Some(logic_path_for_result),
                                        None,
                                        None,
                                        Vec::new()
                                    );
                                    p.mark_executed(format!("{} - {}", execution_result, error_msg));
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
                },
                "rejected" => {
                    if !matches!(proposal.status, ProposalStatus::Voting) && !force {
                        return Err(format!(
                            "Cannot reject proposal from {:?} state. Must be in Voting state or use --force flag.",
                            proposal.status
                        ).into());
                    }
                    proposal.mark_rejected();
                },
                "expired" => proposal.mark_expired(),
                _ => return Err(format!("Invalid status: {}", status_str).into()),
            }
            
            // Save the updated proposal
            storage.set_json(
                Some(auth_context),
                namespace,
                &key,
                &proposal,
            )?;
            
            println!("Proposal {} transitioned to {} status.", proposal_id, status_str);
            
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
            let status_filter = list_matches.get_one::<String>("status").map(|s| s.to_lowercase());
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
            
            println!("Comments for proposal: {} (State: {:?})", proposal_id, proposal.state);
            
            // Use fetch_comments_threaded to get all comments for this proposal
            let comments = fetch_comments_threaded(vm, &proposal_id, Some(auth_context))?;
            
            if comments.is_empty() {
                println!("No comments found for this proposal.");
                return Ok(());
            }
            
            // Find root comments (those with no parent)
            let mut roots: Vec<&ProposalComment> = comments
                .values()
                .filter(|c| c.reply_to.is_none())
                .collect();
            
            // Sort root comments based on the sort_by parameter or default to timestamp
            match sort_by.as_deref() {
                Some("author") => roots.sort_by(|a, b| a.author.cmp(&b.author)),
                _ => roots.sort_by_key(|c| c.timestamp), // Default sort by time
            }
            
            println!("\n--- Threaded Comments ---");
            
            if roots.is_empty() {
                println!("No top-level comments found.");
                return Ok(());
            }
            
            // Print threaded comments recursively
            fn print_thread(comments: &HashMap<String, ProposalComment>, comment: &ProposalComment, depth: usize) {
                let indent = "  ".repeat(depth);
                println!("{}└─ [{}] by {} at {}", 
                    indent, 
                    comment.id,
                    comment.author,
                    comment.timestamp.format("%Y-%m-%d %H:%M:%S")
                );
                println!("{}   {}", indent, comment.content);
                
                // Find and sort replies to this comment
                let mut replies: Vec<&ProposalComment> = comments
                    .values()
                    .filter(|c| c.reply_to.as_deref() == Some(&comment.id))
                    .collect();
                
                replies.sort_by_key(|c| c.timestamp);
                
                for reply in replies {
                    print_thread(comments, reply, depth + 1);
                }
            }
            
            for root in roots {
                print_thread(&comments, root, 0);
                println!();
            }
            
            println!("Total comments: {}", comments.len());
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
    let num = num_str.parse::<i64>().map_err(|_| {
        format!("Invalid duration format: {}", duration_str)
    })?;
    
    // Convert to Duration based on unit
    match unit {
        "d" => Ok(Duration::days(num)),
        "h" => Ok(Duration::hours(num)),
        "m" => Ok(Duration::minutes(num)),
        "s" => Ok(Duration::seconds(num)),
        _ => Err(format!("Invalid duration unit: {}. Expected d, h, m, or s", unit).into()),
    }
}

/// Fetch all comments for a proposal
///
/// Retrieves comments from storage and returns them as a HashMap keyed by comment ID.
/// This function checks multiple storage paths to ensure all comments are found,
/// including both the newer and legacy storage locations.
///
/// # Parameters
/// * `vm` - The virtual machine with access to storage
/// * `proposal_id` - ID of the proposal to fetch comments for
/// * `auth` - Optional authentication context
///
/// # Returns
/// * `Result<HashMap<String, ProposalComment>, Box<dyn Error>>` - Comments on success, or an error
///
/// # Errors
/// Returns an error if the storage backend is not available
pub fn fetch_comments_threaded<S>(
    vm: &VM<S>,
    proposal_id: &str,
    auth: Option<&AuthContext>,
) -> Result<HashMap<String, ProposalComment>, Box<dyn Error>> 
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm
        .storage_backend
        .as_ref()
        .ok_or("Storage backend not available")?;

    let namespace = "governance";
    let mut comments = HashMap::new();
    
    // Check both possible storage paths for comments
    
    // Path 1: "comments/{proposal_id}/"
    let prefix1 = format!("comments/{}/", proposal_id);
    if let Ok(keys) = storage.list_keys(auth, namespace, Some(&prefix1)) {
        for key in keys {
            if let Ok(comment) = storage.get_json::<ProposalComment>(auth, namespace, &key) {
                comments.insert(comment.id.clone(), comment);
            }
        }
    }
    
    // Path 2: "proposals/{proposal_id}/comments/"
    let prefix2 = format!("proposals/{}/comments/", proposal_id);
    if let Ok(keys) = storage.list_keys(auth, namespace, Some(&prefix2)) {
        for key in keys {
            // Try to parse as ProposalComment first
            if let Ok(comment) = storage.get_json::<ProposalComment>(auth, namespace, &key) {
                comments.insert(comment.id.clone(), comment);
            } 
            // If that fails, try to convert from Comment
            else if let Ok(comment) = storage.get_json::<Comment>(auth, namespace, &key) {
                // Convert to ProposalComment format
                let proposal_comment = ProposalComment {
                    id: comment.id,
                    author: comment.author.did.clone(),
                    timestamp: comment.timestamp,
                    content: comment.content,
                    reply_to: comment.reply_to,
                };
                comments.insert(proposal_comment.id.clone(), proposal_comment);
            }
        }
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
fn print_threaded_comments(
    comment_id: &CommentId,
    comments_map: &HashMap<CommentId, Comment>,
    replies_map: &HashMap<Option<CommentId>, Vec<CommentId>>,
    depth: usize,
) {
    if let Some(comment) = comments_map.get(comment_id) {
        let indent = "  ".repeat(depth);
        println!("{}[{}] by {}:", indent, &comment.id, &comment.author.did);
        println!("{}  {}", indent, &comment.content);
        println!("{}  @{}", indent, comment.timestamp.to_rfc3339());

        if let Some(replies) = replies_map.get(&Some(comment.id.clone())) {
            // Sort replies by timestamp before printing
            let mut sorted_replies = replies.clone();
            sorted_replies.sort_by_key(|id| comments_map.get(id).map(|c| c.timestamp));
            for reply_id in sorted_replies {
                print_threaded_comments(&reply_id, comments_map, replies_map, depth + 1);
            }
        }
    } else {
        println!("Error: Comment {} not found in map.", comment_id);
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
fn match_status(status: &ProposalStatus, status_str: &str) -> bool {
    match (status, status_str) {
        (ProposalStatus::Draft, "draft") => true,
        (ProposalStatus::Deliberation, "deliberation") => true,
        (ProposalStatus::Active, "active") => true,
        (ProposalStatus::Voting, "voting") => true,
        (ProposalStatus::Executed, "executed") => true,
        (ProposalStatus::Rejected, "rejected") => true,
        (ProposalStatus::Expired, "expired") => true,
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
    println!("ID: {} | Status: {:?} | Creator: {}", 
        proposal.id, 
        proposal.status,
        proposal.creator
    );
    println!("  Created: {} | Attachments: {}", 
        proposal.created_at.to_rfc3339(), 
        proposal.attachments.len()
    );
    if let Some(result) = &proposal.execution_result {
        println!("  Result: {}", result);
    }
    println!("---------------------");
}
