use crate::governance::traits::GovernanceOpHandler;
use crate::storage::traits::Storage;
use crate::vm::{VM, VMError};
use crate::vm::types::Op;
use crate::vm::execution::ExecutorOps;
use crate::vm::stack::StackOps;
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Handler for QuorumThreshold operations
pub struct QuorumThresholdHandler;

impl GovernanceOpHandler for QuorumThresholdHandler {
    fn handle<S>(vm: &mut VM<S>, op: &Op) -> Result<(), VMError>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if let Op::QuorumThreshold(threshold) = op {
            // Validate threshold value
            if *threshold < 0.0 || *threshold > 1.0 {
                return Err(VMError::GovernanceError(
                    "QuorumThreshold must be between 0.0 and 1.0".into(),
                ));
            }

            // Pop two values from the stack: total possible votes and total votes cast
            let total_possible = vm.pop_one("QuorumThreshold:total_possible")?;
            let votes_cast = vm.pop_one("QuorumThreshold:votes_cast")?;

            // Validate inputs
            if total_possible <= 0.0 {
                return Err(VMError::GovernanceError(
                    "Total possible votes must be greater than zero".into(),
                ));
            }

            // Calculate participation ratio
            let participation_ratio = votes_cast / total_possible;

            // Log the calculation
            vm.executor.emit_event(
                "governance",
                &format!(
                    "Quorum check: {}/{} = {:.2}%, threshold: {:.2}%",
                    votes_cast,
                    total_possible,
                    participation_ratio * 100.0,
                    threshold * 100.0
                ),
            );

            // Push result to stack: 0.0 (truthy) if threshold met, 1.0 (falsey) if not
            if participation_ratio >= *threshold {
                vm.stack.push(0.0); // Threshold met (truthy in VM)
                vm.executor.emit_event("governance", "Quorum threshold met");
            } else {
                vm.stack.push(1.0); // Threshold not met (falsey in VM)
                vm.executor.emit_event("governance", "Quorum threshold not met");
            }

            Ok(())
        } else {
            Err(VMError::UndefinedOperation("Expected QuorumThreshold operation".into()))
        }
    }
} 