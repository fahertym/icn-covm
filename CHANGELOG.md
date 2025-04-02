# Changelog

All notable changes to the ICN Cooperative Virtual Machine (icn-covm) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2024-04-02

### Added

- **Governance Primitives**: Implemented the first cooperative governance primitive
  - `RankedVote`: Instant-runoff voting (ranked-choice) operation
  - Full support in VM, bytecode, DSL parser
  - Added comprehensive unit tests and error handling
  - Included `demo/governance/ranked_vote.dsl` demonstration
  - Created documentation in `docs/governance.md`

- **VM Improvements**:
  - Added `LoopControl` variant to `VMError` enum for better control flow handling

### Changed

- Renamed project from `nano-cvm` to `icn-covm` to reflect its purpose as the 
  Inter-Cooperative Network's Cooperative Virtual Machine
- Updated serialization and bytecode handling for governance operations
- Improved documentation across the codebase

### Fixed

- Fixed bugs in the VM's loop control and error propagation
- Improved error handling in the bytecode interpreter

## [0.2.0] - 2023-03-30

### Added

- Bytecode compilation and interpretation layer
- Serialization/deserialization of programs
- Performance optimizations for repeated execution
- Integration tests for complex programs

## [0.1.0] - 2023-03-15

### Added

- Initial release of the stack-based virtual machine
- Core VM operations: arithmetic, memory, conditionals, loops
- DSL parser for human-readable program creation
- Basic function definition and calls
- Simple debugging and introspection tools 