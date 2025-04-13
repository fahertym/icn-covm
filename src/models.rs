use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Open,
    Reviewing,
    Accepted,
    Rejected,
    Implemented,
}

impl Default for ProposalStatus {
    fn default() -> Self {
        Self::Draft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteType {
    Up,
    Down,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub user_id: Uuid,
    pub vote_type: VoteType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: ProposalStatus,
    pub author_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub votes: Vec<Vote>,
    pub tags: Vec<String>,
}

impl Proposal {
    pub fn new(
        title: String,
        description: String,
        author_id: Uuid,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            status: ProposalStatus::default(),
            author_id,
            created_at: now,
            updated_at: now,
            votes: Vec::new(),
            tags,
        }
    }

    pub fn vote_count(&self) -> i32 {
        self.votes.iter().fold(0, |acc, vote| {
            acc + match vote.vote_type {
                VoteType::Up => 1,
                VoteType::Down => -1,
                VoteType::Neutral => 0,
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Comment {
    pub fn new(proposal_id: Uuid, author_id: Uuid, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            proposal_id,
            author_id,
            content,
            created_at: now,
            updated_at: now,
        }
    }
}

// Request and Response DTOs

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProposalRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProposalStatus>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub vote_type: VoteType,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: usize,
    pub limit: usize,
    pub total: usize,
    pub total_pages: usize,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: usize, limit: usize, total: usize) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            (total + limit - 1) / limit
        };

        Self {
            data,
            page,
            limit,
            total,
            total_pages,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            status: "success".to_string(),
            data,
        }
    }
} 