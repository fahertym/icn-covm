# Governance Templates

This document describes the governance template system in the ICN-COVM DSL, which allows for reusable governance configurations across proposals.

## Overview

Governance templates provide a way to define standard governance parameters that can be reused across multiple proposals. This promotes consistency in governance processes and simplifies proposal creation. Templates can define common voting thresholds, quorum requirements, deliberation periods, and role restrictions.

## LifecycleConfig

At the core of governance templates is the `LifecycleConfig` struct, which stores all governance parameters for a proposal.

```rust
pub struct LifecycleConfig {
    /// Quorum threshold as a fraction (e.g., 0.6 for 60%)
    pub quorum: Option<f64>,
    /// Vote threshold as a fraction (e.g., 0.5 for 50%)
    pub threshold: Option<f64>,
    /// Minimum deliberation period before voting can start
    pub min_deliberation: Option<Duration>,
    /// Time until proposal expires after being opened for voting
    pub expires_in: Option<Duration>,
    /// Roles required to vote on this proposal
    pub required_roles: Vec<String>,
}
```

### Fields

- **quorum**: Optional fractional value representing the minimum participation required for a valid vote (e.g., 0.6 means 60% of eligible voters must participate)
- **threshold**: Optional fractional value representing the required majority for approval (e.g., 0.5 means >50% of votes must be "yes")
- **min_deliberation**: Optional duration specifying how long a proposal must be in deliberation before voting starts
- **expires_in**: Optional duration specifying how long the voting period lasts before it automatically closes
- **required_roles**: List of roles that are allowed to vote on this proposal

### Merging Behavior

The `LifecycleConfig` struct includes a `merge_from` method that allows combining configurations from templates with explicit configurations:

```rust
impl LifecycleConfig {
    pub fn merge_from(&mut self, other: &Self) {
        if self.quorum.is_none() {
            self.quorum = other.quorum;
        }
        if self.threshold.is_none() {
            self.threshold = other.threshold;
        }
        if self.min_deliberation.is_none() {
            self.min_deliberation = other.min_deliberation;
        }
        if self.expires_in.is_none() {
            self.expires_in = other.expires_in;
        }
        if self.required_roles.is_empty() {
            self.required_roles = other.required_roles.clone();
        }
    }
}
```

This method preserves any existing field values in the target configuration, and only applies values from the source configuration if the corresponding field in the target is empty or unset.

## DSL Syntax

### Template Definition

Templates are defined using the `template` keyword followed by a name in quotes and a block of governance parameters:

```
template "standard_budget" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 48h
    expiresin 7d
    require_role "member"
}
```

### Template Usage

Templates are applied using the `governance use` directive followed by the template name:

```
governance use "standard_budget"
```

This will load all settings from the named template into the current governance configuration.

### Overriding Template Values

Explicit governance blocks can override template values:

```
governance use "standard_budget"

governance {
    quorumthreshold 0.7  # Overrides the 0.5 from the template
    require_role "board" # Overrides "member" from the template
}
```

## Implementation Details

### Template Storage and Lookup

Templates are stored in a `HashMap<String, LifecycleConfig>` during the DSL parsing process. When a template is defined, it's added to this map. When a template is used via `governance use`, it's looked up in the map and merged into the current configuration.

### Error Handling

If a template is referenced but not defined, the parser will return an error:

```
Unknown template 'nonexistent_template' at line X
```

### Parse DSL Return Value

The `parse_dsl` function returns a tuple of operations and the final lifecycle configuration:

```rust
pub fn parse_dsl(source: &str) -> Result<(Vec<Op>, LifecycleConfig), CompilerError>
```

This configuration represents the merged result of all template applications and explicit governance blocks.

## Integration with Proposal CLI

The proposal CLI uses the lifecycle configuration returned by `parse_dsl` when creating new proposals:

1. When a proposal is created with an attached logic file, the DSL is parsed
2. The resulting `LifecycleConfig` is extracted and used to set proposal parameters
3. Command-line arguments can override these values if explicitly provided
4. The finalized configuration is stored in `proposals/{id}/lifecycle`

## Examples

### Basic Template

```
template "standard" {
    quorumthreshold 0.6
    votethreshold 0.5
    mindeliberation 72h
    expiresin 14d
    require_role "member"
}

governance use "standard"

# Proposal logic follows
push 10
push 20
add
```

### Multiple Templates

```
template "basic" {
    quorumthreshold 0.5
    votethreshold 0.6
}

template "emergency" {
    quorumthreshold 0.3
    votethreshold 0.8
    mindeliberation 1h
    expiresin 1d
    require_role "guardian"
}

governance use "emergency"
push 100
```

### Template with Overrides

```
template "standard" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 24h
    expiresin 7d
    require_role "member"
}

governance use "standard"

governance {
    quorumthreshold 0.7        # Override template value
    mindeliberation 48h        # Override template value
}

push 1
push 2
add
```

## Test Cases

The implementation includes several test cases to verify correct behavior:

1. **test_parse_governance_block**: Tests parsing of a standalone governance block without templates
2. **test_parse_without_governance_block**: Tests parsing of DSL code without any governance configuration
3. **test_parse_governance_template**: Tests parsing and applying a template
4. **test_governance_template_with_override**: Tests applying a template and then overriding some values
5. **test_multiple_templates**: Tests defining and using multiple templates

## Changelog

### Added
- Support for reusable governance lifecycle templates in DSL
- `template` and `governance use` syntax
- LifecycleConfig merging with override logic
- Extended `parse_dsl()` to return config and ops
- Integrated config usage in proposal CLI creation
- Five test cases verifying all parsing and merge behavior

### Changed
- `parse_dsl()` now returns `(Vec<Op>, LifecycleConfig)`
- Proposal CLI respects governance config from DSL 