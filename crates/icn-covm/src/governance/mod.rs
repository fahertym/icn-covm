//! Governance operations module
//!
//! This module contains implementations of governance operations:
//! - RankedVote: Ranked-choice voting implementation
//! - LiquidDelegate: Delegate voting power to another account
//! - QuorumThreshold: Check if voting participation meets a threshold
//! - VoteThreshold: Check if vote approval meets a threshold
//!
//! Centralizing governance operations in this module:
//! - Separates governance logic from core VM execution
//! - Enables easier extension with new governance operation types
//! - Improves maintainability of governance-specific code
//! - Sets up for future plugin-style governance logic

pub mod comments;
pub mod proposal;
pub mod proposal_lifecycle;
// Make contents public for use in tests/CLI
pub use comments::{CommentVersion, ProposalComment};
pub use proposal::{Proposal, ProposalStatus};
pub use proposal_lifecycle::{Comment, ExecutionStatus, ProposalLifecycle, ProposalState};

mod liquid_delegate;
mod quorum_threshold;
mod ranked_vote;
pub mod traits;
mod vote_threshold;

use crate::governance::traits::GovernanceOpHandler;
use crate::storage::traits::Storage;
use crate::vm::types::Op;
use crate::vm::{VMError, VM};
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Try to handle a governance operation
///
/// Returns Some(()) if the operation was handled, None otherwise
pub fn try_handle_governance_op<S>(vm: &mut VM<S>, op: &Op) -> Result<Option<()>, VMError>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    match op {
        Op::RankedVote { .. } => {
            ranked_vote::RankedVoteHandler::handle(vm, op)?;
            Ok(Some(()))
        }
        Op::LiquidDelegate { .. } => {
            liquid_delegate::LiquidDelegateHandler::handle(vm, op)?;
            Ok(Some(()))
        }
        Op::QuorumThreshold(..) => {
            quorum_threshold::QuorumThresholdHandler::handle(vm, op)?;
            Ok(Some(()))
        }
        Op::VoteThreshold(..) => {
            vote_threshold::VoteThresholdHandler::handle(vm, op)?;
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}
