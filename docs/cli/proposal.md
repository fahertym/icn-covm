# Proposal CLI Commands

The `proposal` command group provides functionality for managing the complete proposal lifecycle in the governance system, from creation through deliberation, voting, and execution.

## Commands Overview

```bash
icn-covm proposal <SUBCOMMAND>
```

Available subcommands:
- `create` - Create a new proposal
- `attach` - Attach a file to a proposal
- `comment` - Add a comment to a proposal
- `comments` - View threaded comments for a proposal
- `edit` - Edit an existing proposal
- `publish` - Publish a draft proposal for feedback
- `vote` - Cast a vote on an active proposal
- `transition` - Transition a proposal to a new state
- `view` - View the details of a proposal
- `list` - List all proposals with optional filtering

## Detailed Commands

### Create Proposal

Creates a new governance proposal.

```bash
icn-covm proposal create --id <ID> [OPTIONS]
```

#### Arguments
- `--id <ID>` - Unique identifier for the proposal (required)

#### Options
- `--creator <ID>` - Identity ID of the proposal creator (defaults to current user)
- `--logic-path <PATH>` - Path to the proposal logic script
- `--expires-in <DURATION>` - Duration until proposal expires (e.g., "7d", "24h")
- `--discussion-path <PATH>` - Path to the proposal discussion thread
- `--attachments <LIST>` - Comma-separated list of attachment references
- `--min-deliberation <HOURS>` - Minimum hours required for deliberation phase
- `--title <STRING>` - Title of the proposal
- `--quorum <NUMBER>` - Quorum required for the proposal to pass (number of votes)
- `--threshold <NUMBER>` - Threshold required for the proposal to pass
- `--discussion-duration <DURATION>` - Duration for the feedback/discussion phase

#### Example
```bash
icn-covm proposal create --id "budget-2023-q3" --title "Q3 Budget Allocation" --quorum 15 --threshold 10 --min-deliberation 48
```

### Attach Files

Attaches a file to an existing proposal.

```bash
icn-covm proposal attach --id <PROPOSAL_ID> --file <FILE_PATH> [OPTIONS]
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to attach the file to (required)
- `--file <FILE_PATH>` - Path to the file to attach (required)

#### Options
- `--name <STRING>` - Optional name for the attachment (defaults to filename)

#### Example
```bash
icn-covm proposal attach --id "budget-2023-q3" --file budget_breakdown.xlsx --name "budget"
```

### Add Comment

Adds a comment to a proposal, optionally as a reply to another comment.

```bash
icn-covm proposal comment --id <PROPOSAL_ID> --text <COMMENT_TEXT> [OPTIONS]
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to comment on (required)
- `--text <COMMENT_TEXT>` - The text of the comment (required)

#### Options
- `--reply-to <COMMENT_ID>` - ID of the comment to reply to (for threaded discussions)

#### Example
```bash
icn-covm proposal comment --id "budget-2023-q3" --text "We should allocate more to R&D."
icn-covm proposal comment --id "budget-2023-q3" --text "I agree, at least 20% more." --reply-to "comment-12345"
```

### View Comments

View all comments for a proposal in a threaded format.

```bash
icn-covm proposal comments --id <PROPOSAL_ID> [OPTIONS]
```

#### Arguments
- `--id <PROPOSAL_ID>` - Proposal ID to view comments for (required)

#### Options
- `--sort <SORT_BY>` - Sort comments by: time (default), author

#### Example
```bash
icn-covm proposal comments --id "budget-2023-q3"
icn-covm proposal comments --id "budget-2023-q3" --sort author
```

### Edit Proposal

Edit an existing proposal (available in Draft or OpenForFeedback states).

```bash
icn-covm proposal edit --id <PROPOSAL_ID> [OPTIONS]
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to edit (required)

#### Options
- `--new-body <FILE_PATH>` - Path to the new proposal body file
- `--new-logic <FILE_PATH>` - Path to the new proposal logic file

#### Example
```bash
icn-covm proposal edit --id "budget-2023-q3" --new-body updated_proposal.md
```

### Publish Proposal

Transitions a proposal from Draft to OpenForFeedback state.

```bash
icn-covm proposal publish --id <PROPOSAL_ID>
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to publish (required)

#### Example
```bash
icn-covm proposal publish --id "budget-2023-q3"
```

### Vote on Proposal

Cast a vote on an active proposal.

```bash
icn-covm proposal vote --id <PROPOSAL_ID> --choice <VOTE>
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to vote on (required)
- `--choice <VOTE>` - Your vote choice: yes, no, or abstain (required)

#### Example
```bash
icn-covm proposal vote --id "budget-2023-q3" --choice yes
```

### Transition Proposal State

Manually transition a proposal to a new state.

```bash
icn-covm proposal transition --id <PROPOSAL_ID> --status <STATUS> [OPTIONS]
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to transition (required)
- `--status <STATUS>` - New status: deliberation, active, voting, executed, rejected, expired (required)

#### Options
- `--result <RESULT>` - Optional result message for executed proposals
- `--force` - Force status transition ignoring state transition rules

#### Example
```bash
icn-covm proposal transition --id "budget-2023-q3" --status voting
icn-covm proposal transition --id "budget-2023-q3" --status executed --result "Approved with amendments"
```

### View Proposal

View the details and current status of a proposal.

```bash
icn-covm proposal view --id <PROPOSAL_ID> [OPTIONS]
```

#### Arguments
- `--id <PROPOSAL_ID>` - ID of the proposal to view (required)

#### Options
- `--version <VERSION_NUMBER>` - Optionally specify a version to view
- `--comments` - Flag to also view comments
- `--history` - Flag to also view history

#### Example
```bash
icn-covm proposal view --id "budget-2023-q3"
icn-covm proposal view --id "budget-2023-q3" --comments --history
```

### List Proposals

List all proposals with optional filtering.

```bash
icn-covm proposal list [OPTIONS]
```

#### Options
- `--status <STATUS>` - Filter by status: draft, deliberation, active, voting, executed, rejected, expired
- `--creator <CREATOR_ID>` - Filter by creator ID
- `--limit <NUMBER>` - Limit number of proposals to display

#### Example
```bash
icn-covm proposal list
icn-covm proposal list --status voting
icn-covm proposal list --creator alice --limit 5
```

## Proposal Lifecycle

1. **Draft**: Initial proposal creation, editable by creator
2. **OpenForFeedback**: Published for community feedback and deliberation
3. **Active**: Ready for voting after minimum deliberation period
4. **Voting**: Accepting votes from eligible members
5. **Executed**: Proposal passed and executed
6. **Rejected**: Proposal failed to meet required threshold
7. **Expired**: Voting period ended without required participation

## Storage Structure

Proposals and related data are stored in the following structure:

- `proposals/<id>/lifecycle` - The main proposal lifecycle object
- `proposals/<id>/attachments/<name>` - Attached files
- `proposals/<id>/votes/<user_did>` - Individual votes
- `proposals/<id>/comments/<comment_id>` - Comments on the proposal
- `comments/<proposal_id>/<comment_id>` - Alternative location for comments

## Reputation Impact

Each proposal-related action affects a member's reputation:

- Creating a proposal: +1
- Attaching a file: +1
- Adding a comment: +1
- Voting on a proposal: +1
- Editing a proposal: +1
- Transitioning a proposal: +1

## Examples

### Complete Proposal Workflow

```bash
# 1. Create a new proposal
icn-covm proposal create --id "repair-budget" --title "Building Repair Budget" --quorum 20 --threshold 15

# 2. Attach supporting files
icn-covm proposal attach --id "repair-budget" --file inspection.pdf
icn-covm proposal attach --id "repair-budget" --file cost_estimates.xlsx

# 3. Publish for community feedback
icn-covm proposal publish --id "repair-budget"

# 4. Add a comment and replies during deliberation
icn-covm proposal comment --id "repair-budget" --text "We should get more quotes."
icn-covm proposal comment --id "repair-budget" --text "I agree, at least two more." --reply-to "comment-12345"

# 5. View comments and feedback
icn-covm proposal comments --id "repair-budget"

# 6. Move to voting phase
icn-covm proposal transition --id "repair-budget" --status voting

# 7. Cast votes
icn-covm proposal vote --id "repair-budget" --choice yes

# 8. View proposal details with current vote tally
icn-covm proposal view --id "repair-budget"

# 9. After voting concludes, execute the proposal
icn-covm proposal transition --id "repair-budget" --status executed --result "Approved with a 20% contingency"
``` 