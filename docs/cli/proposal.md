# Proposal CLI Commands

The `proposal` command group provides functionality for managing proposal lifecycles in the ICN Cooperative VM.

## Commands

### Create Proposal

Create a new proposal with metadata and thresholds.

```bash
icn-covm proposal create <id> [OPTIONS]
```

#### Arguments
- `id` - Unique proposal identifier (required)

#### Options
- `--title <STRING>` - Proposal title (default: "Untitled Proposal")
- `--author <STRING>` - Proposal author (default: "anonymous")
- `--quorum <FLOAT>` - Required quorum percentage (default: 0.6)
- `--threshold <FLOAT>` - Required approval threshold (default: 0.5)

#### Example
```bash
icn-covm proposal create prop-001 --title "Repair Plan" --author "matt" --quorum 0.75 --threshold 0.6
```

This generates and executes the following DSL:
```dsl
proposal_lifecycle "prop-001" quorum=0.75 threshold=0.6 title="Repair Plan" author="matt" {
    emit "Proposal created"
}
```

### Attach Document

Attach a text document section to a proposal (e.g., rationale, summary).

```bash
icn-covm proposal attach <id> <section> <text>
```

#### Arguments
- `id` - Proposal identifier (required)
- `section` - Document section name (e.g., "summary", "rationale") (required)
- `text` - Section text content (required)

#### Example
```bash
icn-covm proposal attach prop-001 summary "Fix the roof on building 14B"
```

This generates and executes the following DSL:
```dsl
storep "proposals/prop-001/docs/summary" "Fix the roof on building 14B"
```

### Vote on Proposal

Cast a ranked vote on a proposal.

```bash
icn-covm proposal vote <id> --ranked <RANKS>... [OPTIONS]
```

#### Arguments
- `id` - Proposal identifier (required)
- `--ranked <RANKS>...` - One or more integers representing ranked choices (required)

#### Options
- `--identity <STRING>` - Identity to sign the vote with (optional)

#### Example
```bash
icn-covm proposal vote prop-001 --ranked 3 1 2 --identity "member-001"
```

This generates and executes the following DSL:
```dsl
rankedvote "prop-001" 3 1 2
```

## Storage

All proposal commands use the configured storage backend (memory or file) to persist data. The storage path and backend can be configured using:

- `--storage-backend <TYPE>` - Storage backend type (memory or file, default: memory)
- `--storage-path <PATH>` - Path for file storage backend (default: ./storage)

## Identity Context

Commands that require authentication (like voting) will use the current CLI identity context. You can specify an identity using the `--identity` flag where applicable.

## Examples

### Complete Proposal Workflow

1. Create a new proposal:
```bash
icn-covm proposal create prop-001 --title "Repair Budget" --author "matt"
```

2. Attach proposal details:
```bash
icn-covm proposal attach prop-001 summary "Fix the roof on building 14B"
icn-covm proposal attach prop-001 rationale "The roof is leaking and needs immediate repair"
```

3. Cast votes:
```bash
icn-covm proposal vote prop-001 --ranked 1 2 3 --identity "member-001"
icn-covm proposal vote prop-001 --ranked 2 1 3 --identity "member-002"
``` 