use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use crate::storage::traits::Storage;
use crate::storage::errors::StorageError;
use crate::storage::auth::AuthContext;
use std::collections::HashMap;
use crate::vm::VM;
use crate::compiler::parse_dsl;
// Placeholder for identity type, replace with actual type later
type Identity = String;
// Placeholder for attachment metadata, replace with actual type later
type Attachment = String;
// Placeholder for comment ID, replace with actual type later
type CommentId = u64;
// Placeholder for proposal ID, replace with actual type later
type ProposalId = u64;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProposalState {
    Draft,
    OpenForFeedback,
    Voting,
    Executed,
    Rejected,
    Expired,
}

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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub id: CommentId, // Unique ID for this comment
    pub proposal_id: ProposalId,
    pub author: Identity,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub reply_to: Option<CommentId>, // ID of the comment being replied to
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
                    match vote_str.to_lowercase().as_str() {
                        "yes" => yes_votes += 1,
                        "no" => no_votes += 1,
                        "abstain" => abstain_votes += 1,
                        _ => eprintln!("Warning: Invalid vote choice '{}' found for key {}", vote_str, key),
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
    fn execute_proposal_logic(
        &self,
        vm: &mut VM,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("[EXEC] Attempting to execute logic for proposal {}", self.id);

        let storage = vm.storage_backend.as_ref().ok_or("Storage backend unavailable for execution")?;
        let auth_context = vm.auth_context.as_ref(); // Use VM's auth context
        let namespace = "governance";
        let logic_key = format!("proposals/{}/attachments/logic", self.id);

        // 1. Load Logic Attachment Bytes
        println!("[EXEC] Loading logic from {}/{}...", namespace, logic_key);
        let logic_bytes = storage.get(auth_context, namespace, &logic_key)
            .map_err(|e| format!("Failed to load logic attachment: {}", e))?;

        // 2. Convert Bytes to String
        let logic_dsl = String::from_utf8(logic_bytes)
            .map_err(|e| format!("Logic attachment is not valid UTF-8: {}", e))?;
        println!("[EXEC] Logic DSL loaded ({} bytes).", logic_dsl.len());

        // 3. Parse DSL to Ops
        println!("[EXEC] Parsing logic DSL...");
        let ops = parse_dsl(&logic_dsl)
            .map_err(|e| format!("Failed to parse logic DSL: {}", e))?;
        println!("[EXEC] Logic parsed into {} Ops.", ops.len());

        // 4. Execute Ops in the provided VM
        // Note: This executes in the *same* VM context that called the vote handler.
        // Consider if sandboxing or a separate VM instance is truly needed.
        println!("[EXEC] Executing parsed Ops...");
        vm.execute(&ops)
            .map_err(|e| format!("Runtime error executing proposal logic: {}", e))?;

        println!("[EXEC] Proposal logic execution finished successfully.");
        Ok(())
    }

     // Updated state transition for execution - takes &mut VM
     pub fn transition_to_executed(
         &mut self,
         vm: &mut VM,
     ) -> Result<(), Box<dyn std::error::Error>> {
        if self.state == ProposalState::Voting {
            // Tally votes using the VM's storage/auth context
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(vm)?;
            if self.check_passed(yes_votes, no_votes, abstain_votes) {
                 self.state = ProposalState::Executed;
                 self.history.push((Utc::now(), self.state.clone()));

                 // Save the updated state using the VM's storage
                 let storage = vm.storage_backend.as_mut().ok_or("Storage backend unavailable")?;
                 let auth_context = vm.auth_context.as_ref();
                 let namespace = "governance";
                 let key = format!("proposals/{}/lifecycle", self.id);
                 storage.set_json(auth_context, namespace, &key, self)?;
                 println!("Proposal {} state transitioned to Executed.", self.id);

                 // Attempt to execute associated logic within the same VM
                 if let Err(e) = self.execute_proposal_logic(vm) {
                    eprintln!("Error executing proposal {} logic: {}", self.id, e);
                    // Persist the execution error? Revert state? For now, just log.
                 }
             } else {
                 println!("Proposal {} did not pass, cannot transition to Executed.", self.id);
             }
        } else {
             println!("Proposal {} not in Voting state, cannot transition to Executed.", self.id);
        }
        Ok(())
     }

     // Updated state transition for rejection - takes &mut VM
     pub fn transition_to_rejected(
         &mut self,
         vm: &mut VM,
     ) -> Result<(), Box<dyn std::error::Error>> {
        if self.state == ProposalState::Voting {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(vm)?;
            if !self.check_passed(yes_votes, no_votes, abstain_votes) {
                 self.state = ProposalState::Rejected;
                 self.history.push((Utc::now(), self.state.clone()));
                 let storage = vm.storage_backend.as_mut().ok_or("Storage backend unavailable")?;
                 let auth_context = vm.auth_context.as_ref();
                 let namespace = "governance";
                 let key = format!("proposals/{}/lifecycle", self.id);
                 storage.set_json(auth_context, namespace, &key, self)?;
                 println!("Proposal {} state transitioned to Rejected.", self.id);
             } else {
                 println!("Proposal {} passed, cannot transition to Rejected.", self.id);
             }
        } else {
             println!("Proposal {} not in Voting state, cannot transition to Rejected.", self.id);
        }
        Ok(())
     }

      // Updated state transition for expiration - takes &mut VM
     pub fn transition_to_expired(
         &mut self,
         vm: &mut VM,
     ) -> Result<(), Box<dyn std::error::Error>> {
         if self.state == ProposalState::Voting && self.expires_at.map_or(false, |exp| Utc::now() > exp) {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(vm)?;
            if self.check_passed(yes_votes, no_votes, abstain_votes) {
                println!("Proposal {} passed but expired before execution.", self.id);
            }
            self.state = ProposalState::Expired;
            self.history.push((Utc::now(), self.state.clone()));
            let storage = vm.storage_backend.as_mut().ok_or("Storage backend unavailable")?;
            let auth_context = vm.auth_context.as_ref();
            let namespace = "governance";
            let key = format!("proposals/{}/lifecycle", self.id);
            storage.set_json(auth_context, namespace, &key, self)?;
            println!("Proposal {} state transitioned to Expired.", self.id);
         } else if self.state == ProposalState::Voting {
             println!("Proposal {} voting period has not expired yet.", self.id);
         } else {
             println!("Proposal {} not in Voting state, cannot transition to Expired.", self.id);
         }
         Ok(())
     }
} 