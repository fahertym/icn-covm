use crate::compiler::parse_dsl;
use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::StorageError;
use crate::storage::traits::{Storage, StorageBackend};
use crate::vm::Op;
use crate::vm::VM;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json; // Import serde_json for serialization
use std::collections::HashMap;
use std::fmt::Debug; // Import the actual Identity struct
                     // Placeholder for attachment metadata, replace with actual type later
type Attachment = String;
// Use String for IDs
type CommentId = String;
type ProposalId = String;

// Define the Vote type
pub type Vote = u64; // Just an example, replace with your actual Vote type

// Define result and preview types
pub struct ProposalExecutionPreview {
    pub side_effects: Vec<String>,
    pub success_probability: f64,
}

pub enum ProposalExecutionResult {
    Success { log: Vec<String> },
    Failure { reason: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProposalState {
    Draft,
    OpenForFeedback,
    Voting,
    Executed,
    Rejected,
    Expired,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStatus {
    Success,
    Failure(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

// Implement FromStr to parse from CLI string input
use std::str::FromStr;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVoteChoiceError;
impl std::fmt::Display for ParseVoteChoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid vote choice. Must be 'yes', 'no', or 'abstain'.")
    }
}
impl std::error::Error for ParseVoteChoiceError {}

impl FromStr for VoteChoice {
    type Err = ParseVoteChoiceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "yes" => Ok(VoteChoice::Yes),
            "no" => Ok(VoteChoice::No),
            "abstain" => Ok(VoteChoice::Abstain),
            _ => Err(ParseVoteChoiceError),
        }
    }
}

// Implement Display to serialize for storage?
// Or maybe store as string directly is better for simplicity/flexibility?
// Let's stick to storing the string for now, less migration hassle.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProposalLifecycle {
    pub id: ProposalId,
    pub creator: Identity,
    pub created_at: DateTime<Utc>,
    pub state: ProposalState,
    pub title: String, // Added title based on CLI command
    // TODO: Define quorum and threshold properly (e.g., percentage, fixed number)
    pub quorum: u64,
    pub threshold: u64,
    pub expires_at: Option<DateTime<Utc>>, // Voting expiration
    pub discussion_duration: Option<Duration>, // For macro integration
    pub required_participants: Option<u64>, // For macro integration
    pub current_version: u64,
    // attachments: Vec<Attachment>, // Store attachment metadata or links? Store in storage layer.
    // comments: Vec<CommentId>, // Store comment IDs? Store in storage layer.
    pub history: Vec<(DateTime<Utc>, ProposalState)>, // Track state transitions
    pub execution_status: Option<ExecutionStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub id: CommentId,           // Now String
    pub proposal_id: ProposalId, // Now String
    pub author: Identity,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub reply_to: Option<CommentId>, // Now Option<String>
}

impl ProposalLifecycle {
    pub fn new(
        id: ProposalId,
        creator: Identity,
        title: String,
        quorum: u64,
        threshold: u64,
        discussion_duration: Option<Duration>,
        required_participants: Option<u64>,
    ) -> Self {
        let now = Utc::now();
        ProposalLifecycle {
            id,
            creator,
            created_at: now,
            state: ProposalState::Draft,
            title,
            quorum,
            threshold,
            expires_at: None, // Set when voting starts
            discussion_duration,
            required_participants,
            current_version: 1,
            history: vec![(now, ProposalState::Draft)],
            execution_status: None,
        }
    }

    // Placeholder methods for state transitions - logic to be added later
    pub fn open_for_feedback(&mut self) {
        if self.state == ProposalState::Draft {
            self.state = ProposalState::OpenForFeedback;
            self.history.push((Utc::now(), self.state.clone()));
            // TODO: Set expiration based on discussion_duration?
        }
    }

    pub fn start_voting(&mut self, voting_duration: Duration) {
        // TODO: Add checks (e.g., required participants) before allowing transition
        if self.state == ProposalState::OpenForFeedback {
            self.state = ProposalState::Voting;
            self.expires_at = Some(Utc::now() + voting_duration);
            self.history.push((Utc::now(), self.state.clone()));
        }
    }

    pub fn execute(&mut self) {
        if self.state == ProposalState::Voting {
            // Add logic for successful vote
            self.state = ProposalState::Executed;
            self.history.push((Utc::now(), self.state.clone()));
        }
    }

    pub fn reject(&mut self) {
        if self.state == ProposalState::Voting {
            // Add logic for failed vote
            self.state = ProposalState::Rejected;
            self.history.push((Utc::now(), self.state.clone()));
        }
    }

    pub fn expire(&mut self) {
        if self.state == ProposalState::Voting
            && self.expires_at.map_or(false, |exp| Utc::now() > exp)
        {
            self.state = ProposalState::Expired;
            self.history.push((Utc::now(), self.state.clone()));
        }
    }

    pub fn update_version(&mut self) {
        // Logic for handling updates, potentially resetting state or requiring new votes?
        self.current_version += 1;
        // Maybe move back to Draft or OpenForFeedback? Depends on governance rules.
        self.history.push((Utc::now(), self.state.clone()));
    }

    // Tally votes from storage
    pub fn tally_votes<S>(
        &self,
        vm: &mut VM<S>,
        auth_context: Option<&AuthContext>,
    ) -> Result<HashMap<String, Vote>, Box<dyn std::error::Error>>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if self.state != ProposalState::Voting {
            return Err(format!("Proposal {} is not in Voting state", self.id).into());
        }
        let storage = vm
            .storage_backend
            .as_ref()
            .ok_or("Storage backend not configured")?;
        let auth_context = vm.auth_context.as_ref();
        let namespace = "governance";
        let prefix = format!("proposals/{}/votes/", self.id);
        let vote_keys = storage.list_keys(auth_context, namespace, Some(&prefix))?;

        let mut yes_votes = 0;
        let mut no_votes = 0;
        let mut abstain_votes = 0;

        for key in vote_keys {
            if !key.starts_with(&prefix) || key.split('/').count() != 4 {
                eprintln!("Skipping unexpected key in votes directory: {}", key);
                continue;
            }
            match storage.get(auth_context, namespace, &key) {
                Ok(vote_bytes) => {
                    let vote_str = String::from_utf8(vote_bytes).unwrap_or_default();
                    // Parse the stored string into VoteChoice
                    match VoteChoice::from_str(&vote_str) {
                        Ok(VoteChoice::Yes) => yes_votes += 1,
                        Ok(VoteChoice::No) => no_votes += 1,
                        Ok(VoteChoice::Abstain) => abstain_votes += 1,
                        Err(_) => eprintln!(
                            "Warning: Invalid vote choice string '{}' found in storage for key {}",
                            vote_str, key
                        ),
                    }
                }
                Err(e) => {
                    eprintln!("Error reading vote key {}: {}", key, e);
                }
            }
        }

        let mut votes = HashMap::new();
        votes.insert("yes".to_string(), yes_votes);
        votes.insert("no".to_string(), no_votes);
        votes.insert("abstain".to_string(), abstain_votes);

        Ok(votes)
    }

    // Check if the proposal passed based on tallied votes
    pub fn check_passed<S>(
        &self,
        vm: &mut VM<S>,
        auth_context: Option<&AuthContext>,
        votes: &HashMap<String, Vote>,
    ) -> Result<bool, Box<dyn std::error::Error>>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        // 1. Quorum Check: Total participating votes (yes + no) >= quorum
        let total_votes = votes.get("yes").unwrap_or(&0) + votes.get("no").unwrap_or(&0);
        if total_votes < self.quorum {
            println!("Quorum not met: {} votes < {}", total_votes, self.quorum);
            return Ok(false);
        }

        // 2. Threshold Check: yes_votes >= threshold (assuming threshold is a fixed number for now)
        // TODO: Handle percentage thresholds (yes_votes as f64 / total_votes as f64 >= threshold_percentage)
        let yes_votes = votes.get("yes").unwrap_or(&0);
        if yes_votes < &self.threshold {
            println!(
                "Threshold not met: {} yes votes < {}",
                yes_votes, self.threshold
            );
            return Ok(false);
        }

        println!(
            "Proposal passed: Quorum ({}/{}) and Threshold ({}/{}) met.",
            total_votes, self.quorum, yes_votes, self.threshold
        );
        Ok(true)
    }

    // Execute the proposal's logic attachment within the given VM context
    // Returns Ok(ExecutionStatus) on completion (success or failure)
    // Returns Err only if loading/parsing fails before execution starts
    fn execute_proposal_logic<S>(
        &self,
        vm: &mut VM<S>, // Pass original VM mutably to allow commit/rollback
        auth_context: Option<&AuthContext>,
    ) -> Result<ExecutionStatus, Box<dyn std::error::Error>>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        println!(
            "[EXEC] Preparing sandboxed execution for proposal {}",
            self.id
        );

        // --- Create VM Fork ---
        let mut fork_vm = vm.fork()?; // fork() begins the transaction on original VM's storage
        println!("[EXEC] VM Fork created.");

        // --- Logic Loading (using fork's context) ---
        let logic_dsl = {
            let storage = fork_vm
                .storage_backend
                .as_ref()
                .ok_or("Fork storage backend unavailable")?;
            let auth_context = fork_vm.auth_context.as_ref();
            let namespace = "governance"; // Assuming logic is always in governance namespace
            let logic_key = format!("proposals/{}/attachments/logic", self.id);
            println!(
                "[EXEC] Loading logic from {}/{} within fork...",
                namespace, logic_key
            );

            match storage.get(auth_context, namespace, &logic_key) {
                Ok(bytes) => {
                    let dsl = String::from_utf8(bytes)
                        .map_err(|e| format!("Logic attachment is not valid UTF-8: {}", e))?;
                    if dsl.trim().is_empty() {
                        println!("[EXEC] Logic attachment is empty. Skipping execution.");
                        None // Treat empty DSL as skippable
                    } else {
                        println!("[EXEC] Logic DSL loaded ({} bytes) within fork.", dsl.len());
                        Some(dsl)
                    }
                }
                Err(StorageError::NotFound { .. }) => {
                    println!(
                        "[EXEC] No logic attachment found at {}. Skipping execution.",
                        logic_key
                    );
                    None // Treat missing logic as skippable
                }
                Err(e) => return Err(format!("Failed to load logic attachment: {}", e).into()),
            }
        };

        // --- Execution (within Fork) & Transaction Handling ---
        let execution_status = if let Some(dsl) = logic_dsl {
            println!("[EXEC] Parsing logic DSL within fork...");
            let ops = parse_dsl(&dsl).map_err(|e| format!("Failed to parse logic DSL: {}", e))?;
            println!("[EXEC] Logic parsed into {} Ops within fork.", ops.len());

            println!("[EXEC] Executing parsed Ops within fork VM...");
            match fork_vm.execute(&ops) {
                Ok(_) => {
                    println!("[EXEC] Fork execution successful. Committing transaction on original VM...");
                    vm.commit_fork_transaction()?;
                    ExecutionStatus::Success
                }
                Err(e) => {
                    let error_message = format!("Runtime error during fork execution: {}", e);
                    eprintln!("[EXEC] {}", error_message);
                    println!(
                        "[EXEC] Rolling back transaction on original VM due to fork failure..."
                    );
                    vm.rollback_fork_transaction()?; // Rollback original VM's transaction
                    ExecutionStatus::Failure(error_message)
                }
            }
        } else {
            // No logic to execute, commit the (empty) transaction
            println!(
                "[EXEC] No logic DSL found/loaded. Committing empty transaction on original VM."
            );
            vm.commit_fork_transaction()?;
            ExecutionStatus::Success
        };

        Ok(execution_status)
    }

    // Updated state transition for execution
    pub fn transition_to_executed<S>(
        &mut self,
        vm: &mut VM<S>,
        auth_context: Option<&AuthContext>,
    ) -> Result<bool, Box<dyn std::error::Error>>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if self.state == ProposalState::Voting {
            let votes = self.tally_votes(vm, auth_context)?;
            let passed = self.check_passed(vm, auth_context, &votes)?;
            if passed {
                self.state = ProposalState::Executed;
                self.history.push((Utc::now(), self.state.clone()));
                println!("Proposal {} state transitioning to Executed.", self.id);

                // Attempt to execute associated logic
                let exec_result = self.execute_proposal_logic(vm, auth_context);

                // Update status based on execution result
                match exec_result {
                    Ok(_) => {
                        println!("Proposal {} execution completed successfully.", self.id);
                    }
                    Err(e) => {
                        println!("Proposal {} execution failed: {}", self.id, e);
                        // TODO: Set execution_status to Failed
                    }
                }

                Ok(true)
            } else {
                println!(
                    "Proposal {} did not meet the voting requirements to execute.",
                    self.id
                );
                Ok(false)
            }
        } else {
            println!(
                "Proposal {} not in Voting state, cannot transition to Executed.",
                self.id
            );
            Ok(false)
        }
    }

    // Updated state transition for rejection
    pub fn transition_to_rejected<S>(
        &mut self,
        vm: &mut VM<S>,
        auth_context: Option<&AuthContext>,
    ) -> Result<bool, Box<dyn std::error::Error>>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if self.state == ProposalState::Voting {
            let votes = self.tally_votes(vm, auth_context)?;
            let passed = self.check_passed(vm, auth_context, &votes)?;
            if !passed {
                self.state = ProposalState::Rejected;
                self.history.push((Utc::now(), self.state.clone()));
                println!("Proposal {} state transitioning to Rejected.", self.id);
                Ok(true)
            } else {
                println!(
                    "Proposal {} met the voting requirements to execute, cannot reject.",
                    self.id
                );
                Ok(false)
            }
        } else {
            println!(
                "Proposal {} not in Voting state, cannot transition to Rejected.",
                self.id
            );
            Ok(false)
        }
    }

    // Updated state transition for expiration
    pub fn transition_to_expired<S>(
        &mut self,
        vm: &mut VM<S>,
        auth_context: Option<&AuthContext>,
    ) -> Result<bool, Box<dyn std::error::Error>>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if self.state == ProposalState::Voting
            && self.expires_at.map_or(false, |exp| Utc::now() > exp)
        {
            let votes = self.tally_votes(vm, auth_context)?;
            let passed = self.check_passed(vm, auth_context, &votes)?;
            if passed {
                println!("Proposal {} passed but expired before execution.", self.id);
                // Leave execution_status as None or set to Failure("Expired")?
            } else {
                println!(
                    "Proposal {} did not have enough votes before expiry.",
                    self.id
                );
            }
            self.state = ProposalState::Expired;
            self.history.push((Utc::now(), self.state.clone()));
            println!("Proposal {} state transitioning to Expired.", self.id);
            Ok(true)
        } else {
            println!(
                "Proposal {} not in Voting state, cannot transition to Expired.",
                self.id
            );
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import parent module content
    use crate::identity::Identity;
    use chrono::Duration;

    // Helper to create a dummy Identity for testing
    fn test_identity(username: &str) -> Identity {
        Identity::new(username.to_string(), None, "test_member".to_string(), None).unwrap()
    }

    // Helper to create a basic proposal for tests
    fn create_test_proposal() -> ProposalLifecycle {
        let creator_id = test_identity("test_creator");
        ProposalLifecycle::new(
            "prop-123".to_string(),
            creator_id,
            "Test Proposal".to_string(),
            10,                      // quorum
            5,                       // threshold
            Some(Duration::days(7)), // discussion_duration
            None,                    // required_participants
        )
    }

    #[test]
    fn test_proposal_creation_state() {
        let proposal = create_test_proposal();
        assert_eq!(proposal.state, ProposalState::Draft);
        assert_eq!(proposal.current_version, 1);
        assert_eq!(proposal.history.len(), 1);
        assert_eq!(proposal.history[0].1, ProposalState::Draft);
    }

    #[test]
    fn test_open_for_feedback_transition() {
        let mut proposal = create_test_proposal();
        assert_eq!(proposal.state, ProposalState::Draft);

        proposal.open_for_feedback();

        assert_eq!(proposal.state, ProposalState::OpenForFeedback);
        assert_eq!(proposal.history.len(), 2);
        assert_eq!(proposal.history[1].1, ProposalState::OpenForFeedback);
    }

    #[test]
    fn test_start_voting_transition() {
        let mut proposal = create_test_proposal();
        proposal.open_for_feedback(); // Must be in OpenForFeedback first
        assert_eq!(proposal.state, ProposalState::OpenForFeedback);
        assert!(proposal.expires_at.is_none());

        let voting_duration = Duration::days(3);
        let expected_expiry_min = Utc::now() + voting_duration - Duration::seconds(1);
        let expected_expiry_max = Utc::now() + voting_duration + Duration::seconds(1);

        proposal.start_voting(voting_duration);

        assert_eq!(proposal.state, ProposalState::Voting);
        assert_eq!(proposal.history.len(), 3);
        assert_eq!(proposal.history[2].1, ProposalState::Voting);
        assert!(proposal.expires_at.is_some());
        let expires_at = proposal.expires_at.unwrap();
        assert!(
            expires_at > expected_expiry_min && expires_at < expected_expiry_max,
            "Expiry time not within expected range"
        );
    }

    #[test]
    fn test_invalid_transitions() {
        let mut proposal = create_test_proposal();

        // Can't start voting from Draft
        let initial_state = proposal.state.clone();
        let initial_history_len = proposal.history.len();
        proposal.start_voting(Duration::days(1));
        assert_eq!(proposal.state, initial_state); // State should not change
        assert_eq!(proposal.history.len(), initial_history_len); // History should not change
        assert!(proposal.expires_at.is_none());

        // Can't open for feedback from Voting
        proposal.open_for_feedback(); // Move to OpenForFeedback
        proposal.start_voting(Duration::days(1)); // Move to Voting
        assert_eq!(proposal.state, ProposalState::Voting);
        let state_before_invalid = proposal.state.clone();
        let history_len_before_invalid = proposal.history.len();

        proposal.open_for_feedback(); // Attempt invalid transition

        assert_eq!(proposal.state, state_before_invalid); // State should not change
        assert_eq!(proposal.history.len(), history_len_before_invalid); // History should not change
    }

    // TODO: Add tests for tally_votes and check_passed (might require mocking storage or VM)
    // TODO: Add tests for execute/reject/expire transitions (likely better in integration tests)
}
