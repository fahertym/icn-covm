use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
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
} 