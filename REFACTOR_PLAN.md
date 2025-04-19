# ICN-COVM Refactoring Plan

## Overview
This document outlines the concrete steps for modularizing and improving the icn-covm codebase according to the requirements. The goal is to improve maintainability, make onboarding easier, ensure full test coverage, and prepare for Phase 2 of the ICN Roadmap.

## 1. Standardize Feature Modules

### 1.1. Create Standard Module Structure
For each core subsystem (storage, identity, federation, governance, execution, CLI), ensure:
- Dedicated directory with mod.rs
- README.md explaining purpose and APIs
- Complete test coverage

### 1.2. Module Reorganization Tasks
- **Storage Module**
  - [x] Existing module structure looks good
  - [ ] Add README.md to src/storage/
  - [ ] Ensure comprehensive tests exist
  
- **Identity Module**
  - [ ] Move identity.rs into src/identity/ directory
  - [ ] Create src/identity/mod.rs exposing appropriate APIs
  - [ ] Split functionality into appropriate submodules
  - [ ] Add README.md to src/identity/
  
- **Federation Module**
  - [ ] Review and enhance existing federation module
  - [ ] Add README.md to src/federation/
  - [ ] Ensure comprehensive tests for p2p operations
  
- **Governance Module**
  - [ ] Review and enhance existing governance module
  - [ ] Add README.md to src/governance/
  - [ ] Create governance/templates/ submodule for template handling
  
- **VM/Execution Module**
  - [ ] Complete modularization of VM operations
  - [ ] Add README.md to src/vm/
  - [ ] Add dedicated src/vm/ops/ directory for operation handlers

- **CLI Module**
  - [ ] Ensure consistent organization
  - [ ] Add README.md to src/cli/
  - [ ] Add tests for CLI commands

## 2. VM Refactor Continuation

### 2.1. Op Handler Traits
- [ ] Create src/vm/ops/ directory
- [ ] Define handler traits in src/vm/ops/mod.rs:
  ```rust
  pub trait StorageOpHandler { /* methods */ }
  pub trait GovernanceOpHandler { /* methods */ }
  pub trait IdentityOpHandler { /* methods */ }
  ```
- [ ] Move storage operations from execution.rs to src/vm/ops/storage.rs
- [ ] Move governance operations from execution.rs to src/vm/ops/governance.rs
- [ ] Move identity operations from execution.rs to src/vm/ops/identity.rs

### 2.2. Dispatch Mechanism
- [ ] Create an opcode-to-handler dispatch table in src/vm/vm.rs
- [ ] Modify VM to use the dispatch mechanism
- [ ] Update tests to verify correct dispatch behavior

## 3. Typed Value Integration

### 3.1. TypedValue Implementation
- [ ] Review the current TypedValue implementation in typed.rs
- [ ] Ensure it supports all needed types (Number, String, Bool, Null)
- [ ] Define appropriate conversion functions

### 3.2. VM Integration
- [ ] Update VM to handle TypedValue instead of raw f64
- [ ] Modify stack.rs to work with TypedValue
- [ ] Update memory.rs to store TypedValue

### 3.3. Operation Updates
- [ ] Update VM operations to work with TypedValue
- [ ] Ensure parser emits typed operations
- [ ] Update tests to verify typed behavior

## 4. Governance DSL Template Improvements

### 4.1. Template Registry
- [ ] Create src/governance/templates/ directory
- [ ] Implement TemplateRegistry struct in templates/mod.rs
- [ ] Add versioning support for templates

### 4.2. Storage Integration
- [ ] Implement file-backed storage for templates
- [ ] Add methods to load/save templates

### 4.3. CLI Commands
- [ ] Add CLI subcommands for template management:
  - [ ] `governance template list`
  - [ ] `governance template view <name>`
  - [ ] `governance template edit <name>`
  - [ ] `governance template apply <name>`

## 5. Federation Robustness Enhancements

### 5.1. Error Handling
- [ ] Replace all unwrap() calls with proper error handling
- [ ] Add retry logic for libp2p connections
- [ ] Implement exponential backoff for reconnection attempts

### 5.2. CLI Visibility
- [ ] Add `federation peers` command
- [ ] Add `federation gossip-log` command
- [ ] Add `federation send-msg` command

## 6. Test Coverage Hardening

### 6.1. Unit Tests
- [ ] Ensure all CLI handlers have unit tests
- [ ] Add tests for proposal lifecycle execution
- [ ] Add tests for governance template overrides
- [ ] Add tests for typed value operations

### 6.2. Integration Tests
- [ ] Convert large demo files to integration tests
- [ ] Add federation integration tests
- [ ] Add end-to-end governance tests

## 7. CI and Linting Standardization

### 7.1. CI Workflow
- [ ] Create .github/workflows/ci.yml with:
  - [ ] cargo test
  - [ ] cargo fmt --check
  - [ ] cargo clippy -- -D warnings

### 7.2. Configuration
- [ ] Add .cargo/config.toml with rustflags
- [ ] Add pre-commit config

## 8. File Cleanup Tasks

### 8.1. Legacy File Migration
- [ ] Move large demo files to integration tests
- [ ] Move README_*.md files to docs/demos/
- [ ] Create docs/demos/index.md

## 9. Documentation Expansion

### 9.1. API Documentation
- [ ] Add missing TypedValue documentation
- [ ] Document governance macros
- [ ] Document federation behavior

### 9.2. User Guides
- [ ] Generate cargo doc output
- [ ] Update main README.md
- [ ] Create user guides for each major feature

## Implementation Timeline

### Week 1
- Standardize module structure
- Begin VM refactoring

### Week 2
- Complete VM refactoring
- Implement TypedValue integration

### Week 3
- Implement governance template improvements
- Enhance federation robustness

### Week 4
- Improve test coverage
- Set up CI and linting
- Complete documentation

## Conclusion
This refactoring plan will significantly improve the maintainability, robustness, and developer experience of the icn-covm project while preparing it for Phase 2 of the ICN Roadmap. The modular approach will make it easier to add new features and for new developers to understand the codebase. 