use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use crate::storage::traits::{StorageBackend, StorageExtensions};
use crate::storage::errors::StorageError;
use crate::storage::auth::AuthContext;
use std::collections::HashMap;
use crate::vm::VM;
use crate::compiler::parse_dsl;
use crate::vm::Op;
use serde_json; // Import serde_json for serialization
use crate::identity::Identity; // Import the actual Identity struct
// Placeholder for attachment metadata, replace with actual type later
type Attachment = String;
// Use String for IDs
type CommentId = String;
type ProposalId = String;

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
    pub id: CommentId, // Now String
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
        if self.state == ProposalState::Voting { // Add logic for successful vote
             self.state = ProposalState::Executed;
             self.history.push((Utc::now(), self.state.clone()));
        }
     }

     pub fn reject(&mut self) {
         if self.state == ProposalState::Voting { // Add logic for failed vote
             self.state = ProposalState::Rejected;
             self.history.push((Utc::now(), self.state.clone()));
         }
     }

     pub fn expire(&mut self) {
         if self.state == ProposalState::Voting && self.expires_at.map_or(false, |exp| Utc::now() > exp) {
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
    pub fn tally_votes(
        &self,
        vm: &VM,
    ) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> { // (yes, no, abstain)
        if self.state != ProposalState::Voting {
            return Err(format!("Proposal {} is not in Voting state", self.id).into());
        }
        let storage = vm.storage_backend.as_ref().ok_or("Storage backend not configured")?;
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
                        Err(_) => eprintln!("Warning: Invalid vote choice string '{}' found in storage for key {}", vote_str, key),
                    }
                }
                Err(e) => {
                    eprintln!("Error reading vote key {}: {}", key, e);
                }
            }
        }

        Ok((yes_votes, no_votes, abstain_votes))
    }

    // Check if the proposal passed based on tallied votes
    pub fn check_passed(&self, yes_votes: u64, no_votes: u64, _abstain_votes: u64) -> bool {
        // 1. Quorum Check: Total participating votes (yes + no) >= quorum
        let total_votes = yes_votes + no_votes;
        if total_votes < self.quorum {
            println!("Quorum not met: {} votes < {}", total_votes, self.quorum);
            return false;
        }

        // 2. Threshold Check: yes_votes >= threshold (assuming threshold is a fixed number for now)
        // TODO: Handle percentage thresholds (yes_votes as f64 / total_votes as f64 >= threshold_percentage)
        if yes_votes < self.threshold {
            println!("Threshold not met: {} yes votes < {}", yes_votes, self.threshold);
            return false;
        }

        println!("Proposal passed: Quorum ({}/{}) and Threshold ({}/{}) met.", total_votes, self.quorum, yes_votes, self.threshold);
        true
    }

    // Execute the proposal's logic attachment within the given VM context
    // Returns Ok(ExecutionStatus) on completion (success or failure)
    // Returns Err only if loading/parsing fails before execution starts
    fn execute_proposal_logic(
        &self,
        vm: &mut VM, // Pass original VM mutably to allow commit/rollback
    ) -> Result<ExecutionStatus, Box<dyn std::error::Error>> {
        println!("[EXEC] Preparing sandboxed execution for proposal {}", self.id);

        // --- Create VM Fork --- 
        let mut fork_vm = vm.fork()?; // fork() begins the transaction on original VM's storage
        println!("[EXEC] VM Fork created.");

        // --- Logic Loading (using fork's context) --- 
        let logic_dsl = {
            let storage = fork_vm.storage_backend.as_ref().ok_or("Fork storage backend unavailable")?;
            let auth_context = fork_vm.auth_context.as_ref();
            let namespace = "governance"; // Assuming logic is always in governance namespace
            let logic_key = format!("proposals/{}/attachments/logic", self.id);
            println!("[EXEC] Loading logic from {}/{} within fork...", namespace, logic_key);
            
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
                    println!("[EXEC] No logic attachment found at {}. Skipping execution.", logic_key);
                    None // Treat missing logic as skippable
                }
                Err(e) => return Err(format!("Failed to load logic attachment: {}", e).into()),
            }
        };

        // --- Execution (within Fork) & Transaction Handling --- 
        let execution_status = if let Some(dsl) = logic_dsl {
            println!("[EXEC] Parsing logic DSL within fork...");
            let ops = parse_dsl(&dsl)
                .map_err(|e| format!("Failed to parse logic DSL: {}", e))?;
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
                    println!("[EXEC] Rolling back transaction on original VM due to fork failure...");
                    vm.rollback_fork_transaction()?; // Rollback original VM's transaction
                    ExecutionStatus::Failure(error_message)
                }
            }
        } else {
            // No logic to execute, commit the (empty) transaction
            println!("[EXEC] No logic DSL found/loaded. Committing empty transaction on original VM.");
            vm.commit_fork_transaction()?;
            ExecutionStatus::Success
        };

        Ok(execution_status)
    }

     // Updated state transition for execution
     pub fn transition_to_executed(
         &mut self,
         vm: &mut VM,
     ) -> Result<(), Box<dyn std::error::Error>> {
        if self.state == ProposalState::Voting {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(vm)?;
            if self.check_passed(yes_votes, no_votes, abstain_votes) {
                 self.state = ProposalState::Executed;
                 self.history.push((Utc::now(), self.state.clone()));
                 println!("Proposal {} state transitioning to Executed.", self.id);

                 // Attempt to execute associated logic
                 let exec_result = self.execute_proposal_logic(vm);

                 // Update status based on execution result
                 let final_status = match exec_result {
                     Ok(status) => status,
                     Err(e) => {
                         // Error during loading/parsing before execution attempt
                         let err_msg = format!("Pre-execution error: {}", e);
                         eprintln!("{}", err_msg);
                         ExecutionStatus::Failure(err_msg)
                     }
                 };
                 self.execution_status = Some(final_status.clone());

                 // Emit Event for execution status
                 let event_message = match &final_status {
                     ExecutionStatus::Success => format!("Proposal {} executed successfully.", self.id),
                     ExecutionStatus::Failure(reason) => format!("Proposal {} execution failed: {}", self.id, reason),
                 };
                 // Execute EmitEvent Op directly
                 let event_op = Op::EmitEvent { category: "governance".to_string(), message: event_message };
                 if let Err(e) = vm.execute(&[event_op]) {
                     eprintln!("Failed to emit execution status event: {}", e);
                 }

                 // Save the final state including execution status
                 let storage = vm.storage_backend.as_mut().ok_or("Storage backend unavailable")?;
                 let auth_context = vm.auth_context.as_ref();
                 let namespace = "governance";
                 let key = format!("proposals/{}/lifecycle", self.id);
                 // Serialize self to JSON bytes
                 let proposal_bytes = serde_json::to_vec(self)
                    .map_err(|e| format!("Failed to serialize proposal state: {}", e))?;
                 // Use the object-safe `set` method from StorageBackend
                 storage.set(auth_context, namespace, &key, proposal_bytes)?;
                 println!("Proposal {} final state (Executed, Status: {:?}) saved.", self.id, self.execution_status);

             } else {
                 println!("Proposal {} did not pass, cannot transition to Executed.", self.id);
                 // Optionally attempt transition_to_rejected(vm)? here
             }
        } else {
             println!("Proposal {} not in Voting state, cannot transition to Executed.", self.id);
        }
        Ok(())
     }

     // Updated state transition for rejection
     pub fn transition_to_rejected(
         &mut self,
         vm: &mut VM,
     ) -> Result<(), Box<dyn std::error::Error>> {
        if self.state == ProposalState::Voting {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(vm)?;
            if !self.check_passed(yes_votes, no_votes, abstain_votes) {
                 self.state = ProposalState::Rejected;
                 self.history.push((Utc::now(), self.state.clone()));
                 self.execution_status = None; // Reset execution status on rejection
                 let storage = vm.storage_backend.as_mut().ok_or("Storage backend unavailable")?;
                 let auth_context = vm.auth_context.as_ref();
                 let namespace = "governance";
                 let key = format!("proposals/{}/lifecycle", self.id);
                 // Serialize self to JSON bytes
                 let proposal_bytes = serde_json::to_vec(self)
                    .map_err(|e| format!("Failed to serialize proposal state: {}", e))?;
                 // Use the object-safe `set` method from StorageBackend
                 storage.set(auth_context, namespace, &key, proposal_bytes)?;
                 println!("Proposal {} state transitioned to Rejected.", self.id);
                 // Emit rejection event?
                 let event_op = Op::EmitEvent {
                    category: "governance".to_string(),
                    message: format!("Proposal {} rejected.", self.id)
                 };
                 if let Err(e) = vm.execute(&[event_op]) {
                     eprintln!("Failed to emit rejection event: {}", e);
                 }
             } else {
                 println!("Proposal {} passed, cannot transition to Rejected.", self.id);
             }
        } else {
             println!("Proposal {} not in Voting state, cannot transition to Rejected.", self.id);
        }
        Ok(())
     }

      // Updated state transition for expiration
     pub fn transition_to_expired(
         &mut self,
         vm: &mut VM,
     ) -> Result<(), Box<dyn std::error::Error>> {
         if self.state == ProposalState::Voting && self.expires_at.map_or(false, |exp| Utc::now() > exp) {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(vm)?;
            if self.check_passed(yes_votes, no_votes, abstain_votes) {
                println!("Proposal {} passed but expired before execution.", self.id);
                // Leave execution_status as None or set to Failure("Expired")?
            }
            self.state = ProposalState::Expired;
            self.history.push((Utc::now(), self.state.clone()));
            self.execution_status = None; // Reset execution status on expiration
            let storage = vm.storage_backend.as_mut().ok_or("Storage backend unavailable")?;
            let auth_context = vm.auth_context.as_ref();
            let namespace = "governance";
            let key = format!("proposals/{}/lifecycle", self.id);
            // Serialize self to JSON bytes
            let proposal_bytes = serde_json::to_vec(self)
                .map_err(|e| format!("Failed to serialize proposal state: {}", e))?;
            // Use the object-safe `set` method from StorageBackend
            storage.set(auth_context, namespace, &key, proposal_bytes)?;
            println!("Proposal {} state transitioned to Expired.", self.id);
            // Emit expiration event?
             let event_op = Op::EmitEvent {
                category: "governance".to_string(),
                message: format!("Proposal {} expired.", self.id)
             };
             if let Err(e) = vm.execute(&[event_op]) {
                 eprintln!("Failed to emit expiration event: {}", e);
             }
         } else if self.state == ProposalState::Voting {
             println!("Proposal {} voting period has not expired yet.", self.id);
         } else {
             println!("Proposal {} not in Voting state, cannot transition to Expired.", self.id);
         }
         Ok(())
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
            10, // quorum
            5,  // threshold
            Some(Duration::days(7)), // discussion_duration
            None, // required_participants
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
        assert!(expires_at > expected_expiry_min && expires_at < expected_expiry_max, "Expiry time not within expected range");
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