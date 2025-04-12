use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use crate::storage::traits::Storage;
use crate::storage::errors::StorageError;
use crate::storage::auth::AuthContext;
use std::collections::HashMap;
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
    pub fn tally_votes<S: Storage>(
        &self,
        storage: &S,
        auth_context: Option<&AuthContext>, // Needed for storage access
    ) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> { // (yes, no, abstain)
        if self.state != ProposalState::Voting {
            return Err(format!("Proposal {} is not in Voting state", self.id).into());
        }

        let namespace = "governance";
        let prefix = format!("proposals/{}/votes/", self.id);
        let vote_keys = storage.list_keys(auth_context, namespace, Some(&prefix))?;

        let mut yes_votes = 0;
        let mut no_votes = 0;
        let mut abstain_votes = 0;

        for key in vote_keys {
            // Ensure the key matches the expected pattern (prefix + voter_id)
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
                    // Log error but continue tallying other votes
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

    // Placeholder - Actual execution logic needs more context
    fn execute_proposal_logic<S: Storage>(
        &self,
        _storage: &S,
        _auth_context: Option<&AuthContext>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Placeholder: Executing logic for proposal {}", self.id);
        // Here you would:
        // 1. Load the execution logic attachment (e.g., logic.ccl/dsl)
        // 2. Parse and execute the logic using the VM
        //    - This might require spawning a new VM instance or using a dedicated execution context
        //    - Pass necessary context (proposal ID, etc.) to the script
        Ok(())
    }

     // Updated state transition for execution
     pub fn transition_to_executed<S: Storage>(
         &mut self,
         storage: &mut S,
         auth_context: Option<&AuthContext>,
     ) -> Result<(), Box<dyn std::error::Error>> {
        if self.state == ProposalState::Voting {
            // Optionally re-tally here or assume tally was done before calling
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(storage, auth_context)?;
            if self.check_passed(yes_votes, no_votes, abstain_votes) {
                 self.state = ProposalState::Executed;
                 self.history.push((Utc::now(), self.state.clone()));
                 // Save the updated state
                 let namespace = "governance";
                 let key = format!("proposals/{}/lifecycle", self.id);
                 storage.set_json(auth_context, namespace, &key, self)?;
                 println!("Proposal {} state transitioned to Executed.", self.id);

                 // Attempt to execute associated logic
                 if let Err(e) = self.execute_proposal_logic(storage, auth_context) {
                    eprintln!("Error executing proposal {} logic: {}", self.id, e);
                    // Should the state revert? Or stay Executed but log the error?
                 }
                 // TODO: Emit execution event?
             } else {
                 println!("Proposal {} did not pass, cannot transition to Executed.", self.id);
                 // Optionally transition to Rejected here? Or handle in a separate check.
             }
        } else {
             println!("Proposal {} not in Voting state, cannot transition to Executed.", self.id);
        }
        Ok(())
     }

     // Updated state transition for rejection
     pub fn transition_to_rejected<S: Storage>(
         &mut self,
         storage: &mut S,
         auth_context: Option<&AuthContext>,
     ) -> Result<(), Box<dyn std::error::Error>> {
        if self.state == ProposalState::Voting {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(storage, auth_context)?;
            if !self.check_passed(yes_votes, no_votes, abstain_votes) {
                 self.state = ProposalState::Rejected;
                 self.history.push((Utc::now(), self.state.clone()));
                 // Save the updated state
                 let namespace = "governance";
                 let key = format!("proposals/{}/lifecycle", self.id);
                 storage.set_json(auth_context, namespace, &key, self)?;
                 println!("Proposal {} state transitioned to Rejected.", self.id);
                 // TODO: Emit rejection event?
             } else {
                 println!("Proposal {} passed, cannot transition to Rejected.", self.id);
             }
        } else {
             println!("Proposal {} not in Voting state, cannot transition to Rejected.", self.id);
        }
        Ok(())
     }

      // Updated state transition for expiration
     pub fn transition_to_expired<S: Storage>(
         &mut self,
         storage: &mut S,
         auth_context: Option<&AuthContext>,
     ) -> Result<(), Box<dyn std::error::Error>> {
         if self.state == ProposalState::Voting && self.expires_at.map_or(false, |exp| Utc::now() > exp) {
            let (yes_votes, no_votes, abstain_votes) = self.tally_votes(storage, auth_context)?;
            // Check if it passed *before* expiring
            if self.check_passed(yes_votes, no_votes, abstain_votes) {
                // If it passed but wasn't explicitly executed, maybe move to Executed?
                // Or have a separate 'PassedNotExecuted' state?
                // For now, let Expired take precedence if time is up.
                println!("Proposal {} passed but expired before execution.", self.id);
            }
            self.state = ProposalState::Expired;
            self.history.push((Utc::now(), self.state.clone()));
            // Save the updated state
            let namespace = "governance";
            let key = format!("proposals/{}/lifecycle", self.id);
            storage.set_json(auth_context, namespace, &key, self)?;
            println!("Proposal {} state transitioned to Expired.", self.id);
            // TODO: Emit expiration event?
         } else if self.state == ProposalState::Voting {
             // Still voting time left
             println!("Proposal {} voting period has not expired yet.", self.id);
         } else {
             println!("Proposal {} not in Voting state, cannot transition to Expired.", self.id);
         }
         Ok(())
     }
} 