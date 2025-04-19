# TypedValue Migration Plan

## Context

The ICN-COVM project is migrating from using raw `f64` values throughout the VM execution to a more robust `TypedValue` enum that supports multiple data types (Number, String, Boolean, Null).

This document outlines the migration strategy for completing this integration properly.

## Current Status

The migration is partially complete:

1. `TypedValue` definition and operations are implemented in `src/typed.rs` 
2. VM stack and memory systems have already been updated to use `TypedValue`
3. Several operation implementations in `vm/ops` module use `TypedValue`
4. Type mismatches exist between old `f64`-based VM interfaces and new `TypedValue`-based implementations

## Required Changes

### 1. Core Execution Trait (High Priority)

The `ExecutorOps` trait in `crates/icn-covm/src/vm/execution.rs` needs to be updated to use `TypedValue`:

```rust
// Current signatures using f64
fn execute_arithmetic(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError>;
fn execute_comparison(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError>;
fn execute_logical(&self, a: f64, op: &str) -> Result<f64, VMError>;
fn execute_binary_logical(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError>;
fn execute_store_p(&mut self, key: &str, value: f64) -> Result<(), VMError>;
fn execute_load_p(&mut self, key: &str, missing_key_behavior: MissingKeyBehavior) -> Result<f64, VMError>;
fn execute_balance(&mut self, resource: &str, account: &str) -> Result<f64, VMError>;

// These need to be updated to use TypedValue instead
```

### 2. Bytecode Operation Handlers (High Priority)

Update the bytecode operation handlers to match the new type signatures:

- `crates/icn-covm/src/vm/vm.rs` - All operation handler implementations 
- `crates/icn-covm/src/bytecode.rs` - Bytecode operation handlers

### 3. Test and Example Code (Medium Priority)

- Update tests to use `TypedValue` instead of raw `f64` values
- Ensure all test code is migrated properly

### 4. DSL and CLI Demos (Medium Priority)

- Update demo code and internal DSL functions to use TypedValues 
- Ensure REPL demos continue to function correctly

### 5. Documentation (Low Priority)

- Update documentation to reflect the TypedValue interfaces
- Add examples of creating and using different TypedValue variants

## Migration Strategy

### Phase 1: Core Interface Updates

1. Update `ExecutorOps` trait in `execution.rs` to use TypedValue
2. Update the `VMExecution` implementation to match new trait signatures
3. Make VMExecution convert between TypedValue and f64 internally as needed for storage operations

### Phase 2: VM Operation Handlers

1. Update the VM operation handlers to use TypedValue 
2. Convert any remaining f64 values to TypedValue when pushing to stack
3. Extract numeric values from TypedValue when needed for calculations

### Phase 3: Tests and Validation

1. Update all test code to use TypedValue
2. Run tests to ensure functionality works as expected
3. Fix any remaining type mismatches

### Phase 4: Documentation and Examples

1. Update documentation to reflect the TypedValue changes
2. Create examples showing how to work with different value types

## Affected Files

Here is a list of the main files that need updating:

1. `crates/icn-covm/src/vm/execution.rs` - ExecutorOps trait 
2. `crates/icn-covm/src/vm/vm.rs` - VM operation implementations
3. `crates/icn-covm/src/bytecode.rs` - Bytecode compiler
4. `crates/icn-covm/src/compiler/macros.rs` - Compiler macros
5. `crates/icn-covm/src/compiler/match_block.rs` - Match operations
6. `crates/icn-covm/src/governance/*.rs` - Governance-related operations
7. Test files throughout the codebase

## Testing Strategy

For each phase of the migration:

1. Run `cargo check` to identify type mismatches
2. Fix the identified issues
3. Run `cargo test --all-features` to validate functionality
4. Run `cargo clippy -- -D warnings` to ensure clean code

## Timeline

- Phase 1 (Core Interface): 1-2 days
- Phase 2 (VM Operations): 2-3 days
- Phase 3 (Tests): 1-2 days
- Phase 4 (Documentation): 1 day

## Risks and Mitigation

- **Risk**: Breaking existing functionality
  - **Mitigation**: Comprehensive testing after each phase

- **Risk**: Inconsistent type conversions
  - **Mitigation**: Create helper methods for TypedValue conversion

- **Risk**: Performance impact
  - **Mitigation**: Benchmark critical paths and optimize if needed 