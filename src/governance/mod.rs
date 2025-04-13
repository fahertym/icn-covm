pub mod proposal_lifecycle;
pub mod proposal;
// Make contents public for use in tests/CLI
pub use proposal_lifecycle::{Comment, ExecutionStatus, ProposalLifecycle, ProposalState};
pub use proposal::{Proposal, ProposalStatus};
