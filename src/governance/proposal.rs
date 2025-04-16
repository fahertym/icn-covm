use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Proposal {
    pub id: String,
    pub creator: String,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub logic_path: Option<String>,
    pub discussion_path: Option<String>,
    pub votes_path: Option<String>,
    pub attachments: Vec<String>,
    pub execution_result: Option<String>,
    pub deliberation_started_at: Option<DateTime<Utc>>,
    pub min_deliberation_hours: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProposalStatus {
    Draft,
    Deliberation,
    Active,
    Voting,
    Approved,
    Executed,
    Rejected,
    Expired,
}

impl Proposal {
    pub fn new(
        id: String,
        creator: String,
        logic_path: Option<String>,
        expires_at: Option<DateTime<Utc>>,
        discussion_path: Option<String>,
        attachments: Vec<String>,
    ) -> Self {
        Self {
            id,
            creator,
            status: ProposalStatus::Draft, // New proposals start as drafts
            created_at: Utc::now(),
            expires_at,
            logic_path,
            discussion_path,
            votes_path: None,
            attachments,
            execution_result: None,
            deliberation_started_at: None,
            min_deliberation_hours: None,
        }
    }

    /// Returns the storage key for this proposal
    pub fn storage_key(&self) -> String {
        format!("governance/proposals/{}", self.id)
    }

    pub fn mark_active(&mut self) {
        self.status = ProposalStatus::Active;
    }

    pub fn mark_deliberation(&mut self) {
        self.status = ProposalStatus::Deliberation;
        self.deliberation_started_at = Some(Utc::now());
    }

    pub fn mark_voting(&mut self) {
        self.status = ProposalStatus::Voting;
    }

    pub fn mark_approved(&mut self) {
        self.status = ProposalStatus::Approved;
    }

    pub fn mark_executed(&mut self, result: String) {
        self.status = ProposalStatus::Executed;
        self.execution_result = Some(result);
    }

    pub fn mark_rejected(&mut self) {
        self.status = ProposalStatus::Rejected;
    }

    pub fn mark_expired(&mut self) {
        self.status = ProposalStatus::Expired;
    }
}
