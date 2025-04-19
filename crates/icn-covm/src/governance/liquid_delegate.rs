use crate::governance::traits::GovernanceOpHandler;
use crate::storage::traits::Storage;
use crate::vm::execution::ExecutorOps;
use crate::vm::memory::MemoryScope;
use crate::vm::types::Op;
use crate::vm::{VMError, VM};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Handler for LiquidDelegate operations
pub struct LiquidDelegateHandler;

impl GovernanceOpHandler for LiquidDelegateHandler {
    fn handle<S>(vm: &mut VM<S>, op: &Op) -> Result<(), VMError>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if let Op::LiquidDelegate { from, to } = op {
            // Validate the from field
            if from.is_empty() {
                return Err(VMError::GovernanceError(
                    "LiquidDelegate requires a non-empty 'from' parameter".into(),
                ));
            }

            // Get current delegations from memory or initialize a new map
            let delegations_key = "governance_delegations";
            let mut delegations: HashMap<String, String> = match vm.memory.load(delegations_key) {
                Ok(_) => {
                    // Try to retrieve from VM metadata
                    if let Some(metadata) = vm.memory.get_string_metadata(delegations_key) {
                        match serde_json::from_str(&metadata) {
                            Ok(map) => map,
                            Err(_) => HashMap::new(),
                        }
                    } else {
                        HashMap::new()
                    }
                }
                Err(_) => {
                    // Initialize an empty delegation map
                    HashMap::new()
                }
            };

            if to.is_empty() {
                // If 'to' is empty, it's a revocation
                if delegations.remove(from).is_some() {
                    vm.executor
                        .emit_event("governance", &format!("Delegation revoked for {}", from));
                } else {
                    vm.executor.emit_event(
                        "governance",
                        &format!("No delegation found to revoke for {}", from),
                    );
                }
            } else {
                // Check for cycles in the delegation graph
                let mut visited = HashMap::new();
                visited.insert(from.clone(), true);

                // Start with the immediate delegation target
                let mut current = to.clone();

                // Follow the delegation chain to detect cycles
                while !current.is_empty() {
                    // If we've seen this node before, we have a cycle
                    if visited.contains_key(&current) {
                        return Err(VMError::GovernanceError(format!(
                            "Delegation from {} to {} would create a cycle",
                            from, to
                        )));
                    }

                    // Mark this node as visited
                    visited.insert(current.clone(), true);

                    // Move to the next node in the chain, if any
                    current = delegations.get(&current).cloned().unwrap_or_default();
                }

                // No cycles found, add the delegation
                delegations.insert(from.clone(), to.clone());
                vm.executor.emit_event(
                    "governance",
                    &format!("Delegation created from {} to {}", from, to),
                );
            }

            // Store the updated delegations map in memory
            let serialized = serde_json::to_string(&delegations).map_err(|e| {
                VMError::Deserialization(format!("Failed to serialize delegations: {}", e))
            })?;

            vm.memory.set_string_metadata(delegations_key, serialized);

            // Also store a numeric value to indicate the delegation count
            vm.memory.store(delegations_key, delegations.len() as f64);

            Ok(())
        } else {
            Err(VMError::UndefinedOperation(
                "Expected LiquidDelegate operation".into(),
            ))
        }
    }
}
