pub mod comments;
pub mod proposal;
pub mod proposal_lifecycle;
// Make contents public for use in tests/CLI
pub use comments::{CommentVersion, ProposalComment};
pub use proposal::{Proposal, ProposalStatus};
pub use proposal_lifecycle::{Comment, ExecutionStatus, ProposalLifecycle, ProposalState};
