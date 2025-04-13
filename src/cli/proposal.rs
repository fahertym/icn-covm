use clap::{arg, Command, Arg, ArgAction, value_parser};
use std::error::Error;
use crate::vm::VM;
use crate::compiler::parse_dsl;
use crate::storage::auth::AuthContext;
use std::path::PathBuf;
use chrono::Utc;
use serde_json;
use crate::governance::proposal_lifecycle::Comment;
use std::fs;
use std::path::Path;
use crate::governance::proposal_lifecycle::ProposalLifecycle;
use crate::storage::traits::StorageExtensions;
use crate::governance::proposal_lifecycle::ProposalState;
use crate::governance::proposal_lifecycle::ExecutionStatus;
use crate::governance::proposal_lifecycle::VoteChoice;
use sha2::{Sha256, Digest};
use hex;
use chrono::Duration;
use std::collections::HashMap;

// Use String for IDs
type ProposalId = String;
type CommentId = String;

pub fn proposal_command() -> Command {
    Command::new("proposal")
        .about("Manage governance proposal lifecycle")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
    Command::new("create")
                .about("Create a new governance proposal")
        .arg(
                    Arg::new("title")
                        .long("title")
                        .value_name("STRING")
                        .help("Title of the proposal")
                        .required(true),
        )
        .arg(
                    Arg::new("quorum")
                        .long("quorum")
                        .value_name("NUMBER")
                        .help("Quorum required for the proposal to pass (e.g., number of votes)")
                        .required(true)
                        .value_parser(value_parser!(u64)),
        )
        .arg(
                     Arg::new("threshold")
                        .long("threshold")
                        .value_name("NUMBER")
                        .help("Threshold required for the proposal to pass (e.g., percentage or fixed number)")
                        .required(true)
                        .value_parser(value_parser!(u64)), // TODO: Adjust parser based on final type (f64 for percentage?)
        )
        .arg(
                     Arg::new("discussion-duration")
                        .long("discussion-duration")
                        .value_name("DURATION") // e.g., 7d, 24h, 30m
                        .help("Optional duration for the feedback/discussion phase (e.g., 7d, 48h)")
                        .required(false) // Optional
                        // No specific value_parser needed, parse string later
        )
        .arg(
                     Arg::new("required-participants")
                        .long("required-participants")
                        .value_name("NUMBER")
                        .help("Optional minimum number of participants required before voting can start")
                        .required(false) // Optional
                        .value_parser(value_parser!(u64))
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
                        // No value_parser needed for String
                )
                .arg(
                    Arg::new("text")
                        .long("text")
                        .value_name("STRING")
                        .help("The text content of the comment")
                        .required(true),
                )
                .arg(
                    Arg::new("reply-to")
                        .long("reply-to")
                        .value_name("COMMENT_ID")
                        .help("Optional ID of the comment to reply to")
                        // No value_parser needed for String
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
}

// Update signature to use String
pub fn load_proposal(vm: &VM, proposal_id: &ProposalId) -> Result<ProposalLifecycle, Box<dyn Error>> {
    let storage = vm.storage_backend.as_ref()
        .ok_or_else(|| "Storage backend not configured for load_proposal")?;
    let namespace = "governance";
    let key = format!("proposals/{}/lifecycle", proposal_id);
    // Need to handle potential deserialization issues if ProposalLifecycle still expects u64 ID
    storage.get_json::<ProposalLifecycle>(vm.auth_context.as_ref(), namespace, &key)
        .map_err(|e| format!("Failed to load proposal {} lifecycle: {}", proposal_id, e).into())
}

pub fn handle_proposal_command(
    matches: &clap::ArgMatches,
    vm: &mut VM,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>> {
    let user_did = auth_context.identity_did(); // Get DID once

    match matches.subcommand() {
        Some(("create", create_matches)) => {
            println!("Handling proposal create...");
            // 1. Parse args
            let title = create_matches.get_one::<String>("title").unwrap();
            let quorum = *create_matches.get_one::<u64>("quorum").unwrap();
            let threshold = *create_matches.get_one::<u64>("threshold").unwrap();

            // Parse optional args
            let discussion_duration_str = create_matches.get_one::<String>("discussion-duration");
            let required_participants = create_matches.get_one::<u64>("required-participants").copied(); // Get Option<u64>

            // Parse duration string (need helper)
            let discussion_duration = discussion_duration_str.map(|s| parse_duration_string(s))
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
                user_did.to_string(), // Use DID as creator
                title.clone(),
                quorum,
                threshold,
                discussion_duration,
                required_participants,
            );

            // 4. Get storage backend MUTABLY for set_json
            let storage = vm.storage_backend.as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal create")?;

            // 5. Store lifecycle object
            // Assuming lifecycle is stored in "governance" namespace
            let namespace = "governance";
            let key = format!("proposals/{}/lifecycle", proposal_id);
            storage.set_json(vm.auth_context.as_ref(), namespace, &key, &proposal)?;
            println!("Proposal {} lifecycle stored.", proposal_id);

            // 6. Emit reputation hook
            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Created proposal {}\"", user_did, proposal_id);
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
                return Err(format!("Attachment file not found or is not a file: {:?}", file_path).into());
            }
            let file_content_bytes = fs::read(file_path)?;

            let attachment_name = attachment_name_opt.map(|s| s.clone()).unwrap_or_else(|| {
                file_path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "attachment".to_string())
            });
            let sanitized_attachment_name = attachment_name.replace('/', "_").replace('\\', "_");

            // Store attachment bytes directly using storage trait
            let storage = vm.storage_backend.as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal attach")?;

            // Assuming attachments stored in "governance" namespace
            let namespace = "governance";
            let key = format!("proposals/{}/attachments/{}", proposal_id, sanitized_attachment_name);

            storage.set(vm.auth_context.as_ref(), namespace, &key, file_content_bytes)?;
            println!("Attachment '{}' stored directly for proposal {}.", sanitized_attachment_name, proposal_id);

            // Emit reputation hook
            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Attached {} to proposal {}\"", user_did, sanitized_attachment_name, proposal_id);
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("comment", comment_matches)) => {
            println!("Handling proposal comment...");
            let proposal_id = comment_matches.get_one::<ProposalId>("id").unwrap().clone(); // Clone String ID
            let text = comment_matches.get_one::<String>("text").unwrap();
            let reply_to = comment_matches.get_one::<CommentId>("reply-to").cloned(); // Clone Option<String> ID

            // Use user_did for ID generation
            let timestamp_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let mut hasher = Sha256::new();
            hasher.update(proposal_id.as_bytes());
            hasher.update(user_did.as_bytes()); // Use DID
            hasher.update(&timestamp_nanos.to_le_bytes());
            let hash_result = hasher.finalize();
            let comment_id = hex::encode(&hash_result[..16]);

            println!("Generated Comment ID: {}", comment_id);

            let comment = Comment {
                id: comment_id.clone(),
                proposal_id: proposal_id.clone(),
                author: user_did.to_string(), // Use DID as author
                timestamp: Utc::now(),
                content: text.clone(),
                reply_to,
            };

            // Store comment object directly using storage trait
            let storage = vm.storage_backend.as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal comment")?;

            // Store comments in "deliberation" namespace
            let namespace = "deliberation";
            let key = format!("comments/{}/{}", proposal_id, comment_id);
            storage.set_json(vm.auth_context.as_ref(), namespace, &key, &comment)?;
            println!("Comment {} stored directly for proposal {}.", comment_id, proposal_id);
            // Explicitly print the ID
            println!("Comment ID: {}", comment_id);

            // Emit reputation hook
            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Commented on proposal {}\"", user_did, proposal_id);
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("view", view_matches)) => {
            println!("Handling proposal view...");
            let proposal_id = view_matches.get_one::<ProposalId>("id").unwrap().clone();
            let specific_version_num = view_matches.get_one::<u64>("version").copied(); // Get Option<u64>
            let show_comments = view_matches.get_flag("comments");
            let show_history = view_matches.get_flag("history");

            let proposal: ProposalLifecycle;
            let mut version_loaded: Option<u64> = None;

            // --- Load Proposal Data (latest or specific version) --- 
            let storage = vm.storage_backend.as_ref()
                .ok_or_else(|| "Storage backend not configured for view")?;
            let namespace = "governance";
            let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
            let auth_ctx_ref = vm.auth_context.as_ref(); // Borrow checker helper

            if let Some(v_num) = specific_version_num {
                println!("Attempting to load version {} for proposal {}", v_num, proposal_id);
                match storage.get_version(auth_ctx_ref, namespace, &lifecycle_key, v_num) {
                    Ok((bytes, version_info)) => {
                        proposal = serde_json::from_slice(&bytes)
                            .map_err(|e| format!("Failed to deserialize proposal version {}: {}", v_num, e))?;
                        version_loaded = Some(version_info.version); // Store which version we actually loaded
                         println!("Successfully loaded version {} (created by {} at {}).", 
                                version_info.version, version_info.created_by, version_info.timestamp);
                    }
                    Err(e) => {
                        return Err(format!("Failed to load version {} for proposal {}: {}", v_num, proposal_id, e).into());
                    }
                }
            } else {
                println!("Loading latest version for proposal {}", proposal_id);
                // Use existing helper which calls get_json (loads latest)
                proposal = load_proposal(vm, &proposal_id)?;
                version_loaded = Some(proposal.current_version); // Assume helper loads the current version
            }

            // --- Display Proposal Info --- 
            println!("--- Proposal {} (Version: {}) ---", proposal_id, version_loaded.map_or_else(|| "Unknown".to_string(), |v| v.to_string()));
            println!("Title: {}", proposal.title);
            println!("Creator: {}", proposal.creator);
            println!("Created: {}", proposal.created_at); // Note: This timestamp is from initial creation
            println!("State (at this version): {:?}", proposal.state);
            println!("Quorum: {}", proposal.quorum);
            println!("Threshold: {}", proposal.threshold);
            // Display execution status if it exists *in this loaded version*
            if let Some(status) = &proposal.execution_status {
                match status {
                    ExecutionStatus::Success => println!("Execution Status: Success"),
                    ExecutionStatus::Failure(reason) => println!("Execution Status: Failure ({})", reason),
                }
            } else if proposal.state == ProposalState::Executed {
                // If state is Executed but status is None (older version?), indicate maybe
                 println!("Execution Status: Not recorded in this version (or execution pending)");
            }

            // --- Display History (from the loaded version's perspective) --- 
            if show_history {
                println!("\n--- History (up to this version) ---");
                for (timestamp, state) in &proposal.history {
                    // Only show history relevant up to this version's creation time?
                    // Or just show all history recorded in this version's snapshot?
                    // Showing snapshot is simpler for now.
                    println!("- {}: {:?}", timestamp, state);
                }
            }

            // --- Display Comments (always shows latest comments, not versioned) --- 
            if show_comments {
                 println!("\n--- Comments ---");
                 let storage = vm.storage_backend.as_ref()
                    .ok_or_else(|| "Storage backend not configured")?;
                 let namespace = "deliberation";
                 let prefix = format!("comments/{}/", proposal_id);
                 let auth_ctx_ref = vm.auth_context.as_ref();

                 match storage.list_keys(auth_ctx_ref, namespace, Some(&prefix)) {
                     Ok(mut comment_keys) => {
                         if comment_keys.is_empty() {
                             println!("No comments found.");
                         } else {
                            let mut comments_map: HashMap<CommentId, Comment> = HashMap::new();
                            let mut replies_map: HashMap<Option<CommentId>, Vec<CommentId>> = HashMap::new();

                             comment_keys.retain(|k| k.starts_with(&prefix) && k.split('/').count() == 3);

                             for comment_key in comment_keys {
                                 match storage.get_json::<Comment>(auth_ctx_ref, namespace, &comment_key) {
                                     Ok(comment) => {
                                         // Store comment by its ID
                                         let id = comment.id.clone();
                                         replies_map.entry(comment.reply_to.clone()).or_default().push(id.clone());
                                         comments_map.insert(id, comment);
                                     }
                                     Err(e) => {
                                         eprintln!("Warning: Failed to load or deserialize comment {}: {}", comment_key, e);
                                     }
                                 }
                             }

                             // Sort root comments and replies by timestamp
                             if let Some(root_ids) = replies_map.get_mut(&None) {
                                 root_ids.sort_by_key(|id| comments_map.get(id).map(|c| c.timestamp).unwrap_or(Utc::now()));
                             }
                             for reply_list in replies_map.values_mut() {
                                  reply_list.sort_by_key(|id| comments_map.get(id).map(|c| c.timestamp).unwrap_or(Utc::now()));
                             }

                            // Recursive function to print threaded comments
                            fn print_threaded_comments(
                                comment_id: &CommentId, 
                                comments_map: &HashMap<CommentId, Comment>, 
                                replies_map: &HashMap<Option<CommentId>, Vec<CommentId>>, 
                                depth: usize
                            ) {
                                if let Some(comment) = comments_map.get(comment_id) {
                                    let indent = "  ".repeat(depth);
                                    println!("{}- [{}] ({}) {}: {}", 
                                        indent,
                                        comment.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                                        comment.id, // Display comment ID for context
                                        comment.author, 
                                        comment.content);

                                    // Print replies recursively
                                    if let Some(reply_ids) = replies_map.get(&Some(comment_id.clone())) {
                                        for reply_id in reply_ids {
                                            print_threaded_comments(reply_id, comments_map, replies_map, depth + 1);
                                        }
                                    }
                                }
                            }

                            // Print root comments
                            if let Some(root_ids) = replies_map.get(&None) {
                                for root_id in root_ids {
                                    print_threaded_comments(root_id, &comments_map, &replies_map, 0);
                                }
                            } else {
                                println!("(No root comments found, check data)");
                            }
                         }
                     }
                     Err(e) => {
                         eprintln!("Error listing comments: {}", e);
                     }
                 }
            }
            // Note: Attachments are also not explicitly loaded by version here.
            // Viewing attachments would likely always show the latest unless specific attachment versioning is added.
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
            if !matches!(proposal.state, ProposalState::Draft | ProposalState::OpenForFeedback) {
                return Err(format!("Proposal {} cannot be edited in its current state: {:?}", proposal_id, proposal.state).into());
            }
            // Check permissions using DID
            if proposal.creator != user_did {
                 return Err(format!("User {} does not have permission to edit proposal {}", user_did, proposal_id).into());
            }

            let mut edited = false;
            let namespace = "governance"; // Namespace for attachments

            // Get mutable storage backend reference once
            let storage = vm.storage_backend.as_mut()
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
                 println!("Proposal {} edited. New version: {}.", proposal_id, proposal.current_version);

                 // 9. Emit reputation hook
                 let rep_dsl = format!("increment_reputation \"{}\" reason=\"Edited proposal {}\"", user_did, proposal_id);
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
             let storage = vm.storage_backend.as_mut()
                 .ok_or_else(|| "Storage backend not configured for proposal publish")?;
             let namespace = "governance";
             let key = format!("proposals/{}/lifecycle", proposal_id);
             // Use direct method call
             storage.set_json(vm.auth_context.as_ref(), namespace, &key, &proposal)?;
             println!("Proposal {} published (state: {:?}).", proposal_id, proposal.state);

             // TODO: Add reputation hook?
        }
        Some(("vote", vote_matches)) => {
            println!("Handling proposal vote...");
            let proposal_id = vote_matches.get_one::<ProposalId>("id").unwrap().clone();
            let choice_enum = vote_matches.get_one::<VoteChoice>("choice").unwrap().clone();

            let choice_str = match choice_enum {
                VoteChoice::Yes => "yes",
                VoteChoice::No => "no",
                VoteChoice::Abstain => "abstain",
            }.to_string();

            let storage_ref_mut = vm.storage_backend.as_mut()
                .ok_or_else(|| "Storage backend not configured for proposal vote")?;
            let namespace = "governance";
            let key = format!("proposals/{}/votes/{}", proposal_id, user_did); // Use DID
            storage_ref_mut.set(Some(auth_context), namespace, &key, choice_str.clone().into_bytes())?;
            println!("Vote '{}' recorded for proposal {} by {}.", choice_str, proposal_id, user_did);

            let mut proposal = load_proposal(vm, &proposal_id)?;

            if let Err(e) = proposal.transition_to_executed(vm) {
                eprintln!("Error during execution check/transition for proposal {}: {}", proposal_id, e);
            }

            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Voted on proposal {}\"", user_did, proposal_id);
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        _ => unreachable!("Subcommand should be required"),
    }
    Ok(())
}

// Helper function to parse duration strings (e.g., "7d", "48h", "30m")
// Consider moving this to a common utility module later
fn parse_duration_string(duration_str: &str) -> Result<Duration, String> {
    let duration_str = duration_str.trim();
    if let Some(days) = duration_str.strip_suffix('d') {
        days.parse::<i64>().map(Duration::days).map_err(|_| "Invalid day value".to_string())
    } else if let Some(hours) = duration_str.strip_suffix('h') {
        hours.parse::<i64>().map(Duration::hours).map_err(|_| "Invalid hour value".to_string())
    } else if let Some(minutes) = duration_str.strip_suffix('m') {
        minutes.parse::<i64>().map(Duration::minutes).map_err(|_| "Invalid minute value".to_string())
    } else {
        Err(format!("Invalid duration format: {}. Use d, h, or m suffix.", duration_str))
    }
}
