pub mod proposal_lifecycle;
// Make contents public for use in tests/CLI
pub use proposal_lifecycle::{Comment, ExecutionStatus, ProposalLifecycle, ProposalState};
