use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API version information
/// 
/// Provides information about the API version, including release date, stability status, and deprecation info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    /// Version string in semver format (e.g. "1.0.0")
    pub version: String,
    /// Release date in ISO 8601 format
    pub released: String,
    /// Indicates if this version is stable
    pub stable: bool,
    /// Indicates when this version will be deprecated (if known)
    pub deprecation_date: Option<String>,
}

//
// DSL Models
//

/// Macro definition model for API requests/responses
/// 
/// Represents a DSL macro with its code, metadata, and visual representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDefinition {
    /// Unique identifier for the macro
    pub id: String,
    /// Name of the macro
    pub name: String,
    /// The DSL code contents of the macro
    pub code: String,
    /// Optional description of the macro's purpose
    pub description: Option<String>,
    /// Creation timestamp in ISO 8601 format
    pub created_at: String,
    /// Last update timestamp in ISO 8601 format
    pub updated_at: String,
    /// Category for grouping macros (e.g., "economic", "governance")
    pub category: Option<String>,
    /// Visual representation for the UI
    pub visual_representation: Option<MacroVisualRepresentation>,
}

/// Model for creating a new macro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMacroRequest {
    /// Name of the macro
    pub name: String,
    /// The DSL code contents of the macro
    pub code: String,
    /// Optional description of the macro's purpose
    pub description: Option<String>,
    /// Category for grouping macros
    pub category: Option<String>,
    /// Visual representation for the UI
    pub visual_representation: Option<MacroVisualRepresentation>,
}

/// Visual representation of a macro for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroVisualRepresentation {
    /// Node definitions for the visual graph
    pub nodes: Vec<NodeInfo>,
    /// Edge definitions connecting the nodes
    pub edges: Vec<EdgeInfo>,
}

/// Node information for visual representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique identifier for the node
    pub id: String,
    /// Type of node (e.g., "dslNode", "macroNode", "actionNode")
    pub node_type: String,
    /// Node data containing label, value, and other properties
    pub data: HashMap<String, serde_json::Value>,
    /// Position of the node in the visual editor
    pub position: Position,
}

/// Position data for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

/// Edge information for visual representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInfo {
    /// Unique identifier for the edge
    pub id: String,
    /// ID of the source node
    pub source: String,
    /// ID of the target node
    pub target: String,
    /// Whether the edge should be animated
    pub animated: Option<bool>,
    /// Optional label for the edge
    pub label: Option<String>,
}

/// Macro list response model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroListResponse {
    /// Total count of available macros
    pub total: usize,
    /// Page number for pagination
    pub page: usize,
    /// Number of items per page
    pub page_size: usize,
    /// List of macro definitions
    pub macros: Vec<MacroSummary>,
}

/// Summary information about a macro
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroSummary {
    /// Unique identifier for the macro
    pub id: String,
    /// Name of the macro
    pub name: String,
    /// Short description or excerpt
    pub description: Option<String>,
    /// Creation timestamp in ISO 8601 format
    pub created_at: String,
    /// Last update timestamp in ISO 8601 format
    pub updated_at: String,
    /// Category for grouping macros
    pub category: Option<String>,
}

//
// Governance Models
//

/// Proposal model for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier for the proposal
    pub id: String,
    /// Title of the proposal
    pub title: String,
    /// Creator of the proposal
    pub creator: String,
    /// Current status of the proposal
    pub status: String,
    /// Creation timestamp in ISO 8601 format
    pub created_at: String,
    /// Vote statistics
    pub votes: VoteCounts,
    /// Percentage of quorum reached (0.0-100.0)
    pub quorum_percentage: f64,
    /// Percentage of threshold reached (0.0-100.0)
    pub threshold_percentage: f64,
    /// Result of proposal execution (if executed)
    pub execution_result: Option<String>,
    /// Details of the proposal
    pub details: Option<String>,
    /// List of attachments
    pub attachments: Vec<ProposalAttachment>,
}

/// Model for creating a new proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposalRequest {
    /// Title of the proposal
    pub title: String,
    /// Details of the proposal
    pub details: String,
    /// Execution DSL code (if applicable)
    pub execution_code: Option<String>,
    /// Quorum percentage required (0.0-100.0)
    pub quorum: f64,
    /// Threshold percentage required (0.0-100.0)
    pub threshold: f64,
    /// List of attachments
    pub attachments: Vec<ProposalAttachment>,
}

/// Proposal attachment model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalAttachment {
    /// Name of the attachment
    pub name: String,
    /// MIME type of the attachment
    pub mime_type: String,
    /// Content of the attachment (base64 encoded if binary)
    pub content: String,
}

/// Vote count information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteCounts {
    /// Number of yes votes
    pub yes: u32,
    /// Number of no votes
    pub no: u32,
    /// Number of abstain votes
    pub abstain: u32,
    /// Total number of votes
    pub total: u32,
}

/// Comment model for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique identifier for the comment
    pub id: String,
    /// Author of the comment
    pub author: String,
    /// Timestamp in ISO 8601 format
    pub timestamp: String,
    /// Content of the comment
    pub content: String,
    /// ID of the parent comment if this is a reply
    pub reply_to: Option<String>,
    /// Tags associated with the comment
    pub tags: Vec<String>,
    /// Reactions to the comment
    pub reactions: HashMap<String, u32>,
    /// Whether the comment is hidden
    pub hidden: bool,
    /// Number of edits
    pub edit_count: usize,
}

/// Model for creating a new comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    /// Content of the comment
    pub content: String,
    /// ID of the parent comment if this is a reply
    pub reply_to: Option<String>,
    /// Tags to associate with the comment
    pub tags: Vec<String>,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based)
    pub page: Option<usize>,
    /// Items per page
    pub page_size: Option<usize>,
}

/// Sort parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortParams {
    /// Field to sort by
    pub sort_by: Option<String>,
    /// Sort direction (asc or desc)
    pub sort_dir: Option<String>,
}

/// Proposal execution history model
#[derive(Debug, Deserialize, Serialize)]
pub struct ProposalExecution {
    /// Unique identifier for the proposal
    pub proposal_id: String,
    /// Current status of the execution
    pub status: String,
    /// Result of proposal execution
    pub execution_result: String,
    /// Logs from the execution process
    pub execution_logs: String,
    /// Timestamp when execution occurred
    pub executed_at: String,
}

// Proposal related models
#[derive(Debug, Deserialize, Serialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub author: String,
    pub votes_for: i32,
    pub votes_against: i32,
    pub votes_abstain: i32,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProposalRequest {
    pub title: String,
    pub description: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Attachment {
    pub name: String,
    pub content_type: String,
    pub url: String,
    pub size: i64,
}

// Comment related models
#[derive(Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: String,
    pub proposal_id: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub reactions: HashMap<String, i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

// Macro related models
#[derive(Debug, Deserialize, Serialize)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dsl_code: String,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMacroRequest {
    pub name: String,
    pub description: String,
    pub dsl_code: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateMacroRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub dsl_code: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteMacroRequest {
    pub params: HashMap<String, serde_json::Value>,
}

// Storage extension models
#[derive(Debug, Deserialize, Serialize)]
pub struct StorageInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub item_count: i64,
    pub size_bytes: i64,
}

// User related models
#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_login: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
    pub user: User,
}

// Common response models
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct PagedResponse<T> {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub data: Vec<T>,
}

// Execution related models
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionVersionMeta {
    /// Version number
    pub version: u64,
    /// Timestamp when execution occurred
    pub executed_at: String,
    /// Whether the execution was successful
    pub success: bool,
    /// Short summary of the execution result
    pub summary: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionVersionsResponse {
    /// Total count of execution versions
    pub total: usize,
    /// List of execution version metadata
    pub versions: Vec<ExecutionVersionMeta>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionVersionQuery {
    /// Optional version number to retrieve specific version
    pub version: Option<u64>,
} 