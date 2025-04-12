use clap::{arg, Command, Arg, ArgAction, value_parser};
use std::error::Error;
use icn_covm::vm::VM;
use icn_covm::compiler::parse_dsl;
use icn_covm::storage::auth::AuthContext;
use std::path::PathBuf;
use chrono::Utc;
use serde_json;
use icn_covm::governance::proposal_lifecycle::Comment;
use std::fs;
use std::path::Path;
use icn_covm::governance::proposal_lifecycle::ProposalLifecycle;
use icn_covm::storage::traits::StorageExtensions; // Import the extension trait
use icn_covm::governance::proposal_lifecycle::ProposalState; // Import ProposalState

// Placeholder types - replace with actual types from governance module
type ProposalId = u64;
type CommentId = u64;

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
                // TODO: Add args for discussion_duration, required_participants
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
                        .value_parser(value_parser!(ProposalId)),
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
                        .value_parser(value_parser!(ProposalId)),
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
                        .value_parser(value_parser!(CommentId)), // Not required
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
                        .value_parser(value_parser!(ProposalId)),
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
                        .value_parser(value_parser!(ProposalId)),
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
                        .value_parser(value_parser!(ProposalId)),
                )
                .arg(
                    Arg::new("choice")
                        .long("choice")
                        .value_name("STRING")
                        .help("Your vote choice (e.g., 'yes', 'no', 'abstain')") // TODO: Define valid choices
                        .required(true),
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
                        .value_parser(value_parser!(ProposalId)),
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

fn load_proposal(vm: &VM, proposal_id: ProposalId) -> Result<ProposalLifecycle, Box<dyn Error>> {
    let storage = vm.storage_backend.as_ref()
        .ok_or_else(|| "Storage backend not configured for load_proposal")?;
    let namespace = "governance";
    let key = format!("proposals/{}/lifecycle", proposal_id);
    storage.get_json::<ProposalLifecycle>(vm.auth_context.as_ref(), namespace, &key)
        .map_err(|e| format!("Failed to load proposal {} lifecycle: {}", proposal_id, e).into())
}

pub fn handle_proposal_command(
    matches: &clap::ArgMatches,
    vm: &mut VM,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>> {
    match matches.subcommand() {
        Some(("create", create_matches)) => {
            println!("Handling proposal create...");
            // 1. Parse args
            let title = create_matches.get_one::<String>("title").unwrap();
            let quorum = *create_matches.get_one::<u64>("quorum").unwrap();
            let threshold = *create_matches.get_one::<u64>("threshold").unwrap();
            // TODO: Parse discussion_duration, required_participants when added to CLI args

            // 2. Generate Proposal ID (using timestamp - needs improvement)
            let proposal_id = Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;

            // 3. Create ProposalLifecycle instance
            let proposal = ProposalLifecycle::new(
                proposal_id,
                auth_context.user_id.clone(), // Use authenticated user ID
                title.clone(),
                quorum,
                threshold,
                None, // TODO: Get from args
                None, // TODO: Get from args
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

            // 6. Emit reputation hook (placeholder via DSL)
            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Created proposal {}\"", auth_context.user_id, proposal_id);
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;

            println!("Proposal {} created with title '{}'.", proposal_id, title);
        }
        Some(("attach", attach_matches)) => {
            println!("Handling proposal attach...");
            let proposal_id = *attach_matches.get_one::<ProposalId>("id").unwrap();
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

            // Emit reputation hook (placeholder via DSL)
            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Attached {} to proposal {}\"", auth_context.user_id, sanitized_attachment_name, proposal_id);
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("comment", comment_matches)) => {
            println!("Handling proposal comment...");
            let proposal_id = *comment_matches.get_one::<ProposalId>("id").unwrap();
            let text = comment_matches.get_one::<String>("text").unwrap();
            let reply_to = comment_matches.get_one::<CommentId>("reply-to").copied();

            let comment_id = Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            let comment = Comment {
                id: comment_id,
                proposal_id,
                author: auth_context.user_id.clone(),
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

            // Emit reputation hook (placeholder via DSL)
            let rep_dsl = format!("increment_reputation \"{}\" reason=\"Commented on proposal {}\"", auth_context.user_id, proposal_id);
            let ops = parse_dsl(&rep_dsl)?;
            vm.execute(&ops)?;
        }
        Some(("view", view_matches)) => {
            println!("Handling proposal view...");
            let proposal_id = *view_matches.get_one::<ProposalId>("id").unwrap();
            let _version = view_matches.get_one::<u64>("version"); // Handle version later
            let show_comments = view_matches.get_flag("comments");
            let show_history = view_matches.get_flag("history");

            // Load the proposal lifecycle data using the helper
            // Pass immutable vm reference here
            let proposal = load_proposal(vm, proposal_id)?;

            // Display basic proposal info
            println!("--- Proposal {} ---", proposal.id);
            println!("Title: {}", proposal.title);
            println!("Creator: {}", proposal.creator);
            println!("Created: {}", proposal.created_at);
            println!("State: {:?}", proposal.state);
            println!("Quorum: {}", proposal.quorum);
            println!("Threshold: {}", proposal.threshold);
            println!("Current Version: {}", proposal.current_version);
            if let Some(expires) = proposal.expires_at {
                println!("Voting Expires: {}", expires);
            }

            if show_history {
                println!("\n--- History ---");
                for (timestamp, state) in &proposal.history {
                    println!("- {}: {:?}", timestamp, state);
                }
            }

            if show_comments {
                 println!("\n--- Comments ---");
                 let storage = vm.storage_backend.as_ref()
                    .ok_or_else(|| "Storage backend not configured")?;
                 let namespace = "deliberation";
                 let prefix = format!("comments/{}/", proposal_id);

                 match storage.list_keys(vm.auth_context.as_ref(), namespace, Some(&prefix)) {
                     Ok(mut comment_keys) => {
                         if comment_keys.is_empty() {
                             println!("No comments found.");
                         } else {
                             let mut comments: Vec<Comment> = Vec::new();
                             // Filter keys to ensure they match the expected comment pattern
                             comment_keys.retain(|k| k.starts_with(&prefix) && k.split('/').count() == 3);

                             for comment_key in comment_keys {
                                 // Key returned by list_keys should be relative to namespace root
                                 println!("Loading comment key: {}", comment_key); // Debug print

                                 // Use direct method call
                                 match storage.get_json::<Comment>(vm.auth_context.as_ref(), namespace, &comment_key) {
                                     Ok(comment) => {
                                         comments.push(comment);
                                     }
                                     Err(e) => {
                                         eprintln!("Warning: Failed to load or deserialize comment {}: {}", comment_key, e);
                                     }
                                 }
                             }

                             comments.sort_by_key(|c| c.timestamp);

                             for comment in comments {
                                 print!("- [{}] {}: {}", comment.timestamp.format("%Y-%m-%d %H:%M:%S UTC"), comment.author, comment.content);
                                 if let Some(reply_id) = comment.reply_to {
                                     print!(" (in reply to: {})", reply_id);
                                 }
                                 println!();
                             }
                         }
                     }
                     Err(e) => {
                         eprintln!("Error listing comments: {}", e);
                     }
                 }
            }
        }
        Some(("edit", edit_matches)) => {
            println!("Handling proposal edit...");
            // 1. Parse args
            let proposal_id = *edit_matches.get_one::<ProposalId>("id").unwrap();
            let new_body_path = edit_matches.get_one::<PathBuf>("new-body");
            let new_logic_path = edit_matches.get_one::<PathBuf>("new-logic");

            // 2. Load proposal
            let mut proposal = load_proposal(vm, proposal_id)?;

            // 3. Check state
            if !matches!(proposal.state, ProposalState::Draft | ProposalState::OpenForFeedback) {
                return Err(format!("Proposal {} cannot be edited in its current state: {:?}", proposal_id, proposal.state).into());
            }
            // Check permissions - does the auth_context user match proposal.creator?
            if proposal.creator != auth_context.user_id {
                 return Err(format!("User {} does not have permission to edit proposal {}", auth_context.user_id, proposal_id).into());
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
                 let rep_dsl = format!("increment_reputation \"{}\" reason=\"Edited proposal {}\"", auth_context.user_id, proposal_id);
                 let ops = parse_dsl(&rep_dsl)?;
                 vm.execute(&ops)?;
            } else {
                println!("No changes specified for proposal {}.", proposal_id);
            }
        }
        Some(("publish", publish_matches)) => {
            println!("Handling proposal publish...");
             let proposal_id = *publish_matches.get_one::<ProposalId>("id").unwrap();

            let mut proposal = load_proposal(vm, proposal_id)?;
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
            let proposal_id = *vote_matches.get_one::<ProposalId>("id").unwrap();
            let choice = vote_matches.get_one::<String>("choice").unwrap();

             // 1. Load Proposal to check state
             let proposal = load_proposal(vm, proposal_id)?;
             if proposal.state != icn_covm::governance::proposal_lifecycle::ProposalState::Voting {
                 return Err(format!("Proposal {} is not in Voting state (current: {:?})", proposal_id, proposal.state).into());
             }

             // 2. Record vote (Store in storage, e.g., governance/proposals/{id}/votes/{voter_id})
             let storage = vm.storage_backend.as_mut()
                 .ok_or_else(|| "Storage backend not configured for proposal vote")?;
             let namespace = "governance";
             let key = format!("proposals/{}/votes/{}", proposal_id, auth_context.user_id);

             // Simple storage of the choice string for now
             storage.set(vm.auth_context.as_ref(), namespace, &key, choice.clone().into_bytes())?;
             println!("Vote '{}' recorded for proposal {} by {}.", choice, proposal_id, auth_context.user_id);

             // 3. Emit reputation hook (placeholder via DSL)
             let rep_dsl = format!("increment_reputation \"{}\" reason=\"Voted on proposal {}\"", auth_context.user_id, proposal_id);
             let ops = parse_dsl(&rep_dsl)?;
             vm.execute(&ops)?;

             // TODO: Add logic to check if vote changes proposal outcome (check quorum/threshold, transition state)
        }
        _ => unreachable!("Subcommand should be required"),
    }
    Ok(())
}
