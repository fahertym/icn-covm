# TypedValue Migration Plan

## Overview
This document tracks the migration from `f64` to `TypedValue` throughout the codebase. The goal is to ensure consistent use of the `TypedValue` enum for representing values in the VM and related systems.

## Phase 1: ✅ Initial Migration
- [x] Define `TypedValue` enum in `typed.rs`
- [x] Implement core methods like `as_number()`, `as_boolean()`, etc.
- [x] Implement arithmetic and comparison operations
- [x] Update `VMError::TypeError` to `VMError::TypeMismatch`

## Phase 2: 🚧 Cleanup (In Progress)
- [ ] Search for and replace f64 literals (e.g., `0.0`, `1.0`) with `TypedValue::Number()`
- [ ] Fix `Op::Push` usage to use `TypedValue` rather than raw `f64`
- [ ] Fix failing tests
- [ ] Update all `as f64` casts to use proper TypedValue methods

### Critical Areas to Address
1. **VM Operations**
   - [x] Fix `execute_increment_reputation` to properly handle the conversion from TypedValue to u64
   - [ ] Update VM stack operations to handle TypedValue consistently
   - [ ] Ensure all arithmetic operations use TypedValue methods

2. **Test Cases**
   - [ ] Update test assertions to use TypedValue
   - [ ] Fix equality checks to compare TypedValue instead of f64

3. **Governance Operations**
   - [ ] Fix quorum and vote threshold calculations to use TypedValue
   - [ ] Update ranked voting to handle TypedValue properly

## Phase 3: 🔜 Trace and Explain
- [ ] Implement `--trace` and `--explain` CLI flags
- [ ] Add logging hooks inside VM loop
- [ ] Create `TypedFrameTrace` struct for execution tracing
- [ ] Use `TypedValue::describe()` in logs

## Usage Examples

### TypedValue Debug Helper
```rust
// Improved debugging with describe()
println!("→ Pushed {}", value.describe());
```

### TypedValue Comparison
```rust
// Old style
if value == 0.0 { ... }

// New style
if value == TypedValue::Number(0.0) { ... }
// or better:
if value.is_falsey() { ... }
```

### Arithmetic Operations
```rust
// Old style
let result = val1 + val2;

// New style
let result = val1.add(&val2)?;
```

## Tracking Progress
- Current completion: ~70%
- Remaining files with f64 usage: ~15
- Critical tests passing: 0/10 