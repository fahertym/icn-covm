use crate::governance::traits::GovernanceOpHandler;
use crate::storage::traits::Storage;
use crate::vm::execution::ExecutorOps;
use crate::vm::stack::StackOps;
use crate::vm::types::Op;
use crate::vm::{VMError, VM};
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Handler for RankedVote operations
pub struct RankedVoteHandler;

impl GovernanceOpHandler for RankedVoteHandler {
    fn handle<S>(vm: &mut VM<S>, op: &Op) -> Result<(), VMError>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if let Op::RankedVote {
            candidates,
            ballots,
        } = op
        {
            // Validate parameters
            if *candidates < 2 {
                return Err(VMError::GovernanceError(
                    "RankedVote requires at least 2 candidates".into(),
                ));
            }

            if *ballots < 1 {
                return Err(VMError::GovernanceError(
                    "RankedVote requires at least 1 ballot".into(),
                ));
            }

            // Collect all ballots from the stack
            let mut all_ballots = Vec::new();

            for _ in 0..*ballots {
                let mut ballot = Vec::new();
                for _ in 0..*candidates {
                    let choice = vm.pop_one("RankedVote")?;
                    ballot.push(choice);
                }
                all_ballots.push(ballot);
            }

            // Perform ranked choice voting calculation
            vm.executor.emit_event(
                "governance",
                &format!(
                    "Running ranked-choice vote with {} candidates and {} ballots",
                    candidates, ballots
                ),
            );

            // Simple implementation of instant-runoff voting
            let mut eliminated = vec![false; *candidates];
            let mut remaining_candidates = *candidates;

            while remaining_candidates > 1 {
                // Count first-choice votes for each candidate
                let mut votes = vec![0; *candidates];

                for ballot in &all_ballots {
                    for (i, &choice) in ballot.iter().enumerate() {
                        let candidate = choice as usize;
                        if candidate < *candidates && !eliminated[candidate] {
                            votes[candidate] += 1;
                            break;
                        }
                    }
                }

                // Find candidate with fewest votes
                let mut min_votes = *ballots + 1;
                let mut min_candidate = 0;

                for (candidate, &vote_count) in votes.iter().enumerate() {
                    if !eliminated[candidate] && vote_count < min_votes && vote_count > 0 {
                        min_votes = vote_count;
                        min_candidate = candidate;
                    }
                }

                // Eliminate candidate with fewest votes
                eliminated[min_candidate] = true;
                remaining_candidates -= 1;

                vm.executor.emit_event(
                    "governance",
                    &format!(
                        "Eliminated candidate {} with {} votes",
                        min_candidate, min_votes
                    ),
                );
            }

            // Find the winner (last non-eliminated candidate)
            let winner = eliminated.iter().position(|&e| !e).unwrap_or(0);

            vm.executor.emit_event(
                "governance",
                &format!("Winner of ranked-choice vote: candidate {}", winner),
            );

            // Push the winner to the stack
            vm.stack.push(winner as f64);
            Ok(())
        } else {
            Err(VMError::UndefinedOperation(
                "Expected RankedVote operation".into(),
            ))
        }
    }
}
