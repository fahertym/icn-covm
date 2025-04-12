use crate::vm::Op;
use crate::compiler::parse_dsl; // Use the correct path from parent module
use crate::governance::proposal_lifecycle::{ProposalLifecycle, ProposalState}; // Import necessary structs
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc, Duration};
use serde_json;

#[derive(Debug, PartialEq)]
enum BlockType {
    Root,
    Execution,
}

// Helper to parse macro arguments within braces {}
fn parse_proposal_block(lines: &[&str]) -> Result<(HashMap<String, String>, HashMap<String, String>, Vec<Op>), String> {
    let mut properties: HashMap<String, String> = HashMap::new();
    let mut attachments: HashMap<String, String> = HashMap::new(); // name -> file_path
    let mut execution_ops: Vec<Op> = Vec::new();
    let mut current_block = BlockType::Root;
    let mut execution_block_lines: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed == "execution {" {
            if current_block != BlockType::Root {
                return Err("Nested execution blocks are not allowed.".to_string());
            }
            current_block = BlockType::Execution;
            continue;
        }

        if trimmed == "}" {
            if current_block == BlockType::Execution {
                current_block = BlockType::Root;
                // Parse the collected execution block lines
                 match parse_dsl(&execution_block_lines.join("
")) {
                    Ok(ops) => execution_ops = ops,
                    Err(e) => return Err(format!("Failed to parse execution block: {}", e)),
                }
                execution_block_lines.clear();
                continue;
            } else {
                 return Err("Unexpected closing brace '}'.".to_string());
            }
        }

        match current_block {
            BlockType::Root => {
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let key = parts[0];
                    let value = parts[1].trim().trim_matches('"').to_string();
                    match key {
                        "title" | "quorum" | "threshold" | "discussion_duration" | "expires_at" => {
                            properties.insert(key.to_string(), value);
                        },
                        "attachment" => {
                            let attachment_parts: Vec<&str> = value.splitn(2, ' ').collect();
                            if attachment_parts.len() == 2 {
                                let name = attachment_parts[0].trim_matches('"');
                                let path = attachment_parts[1].trim_matches('"');
                                attachments.insert(name.to_string(), path.to_string());
                            } else {
                                return Err(format!("Invalid attachment format: {}", trimmed));
                            }
                        },
                        _ => return Err(format!("Unknown property in proposal block: {}", key)),
                    }
                } else {
                     return Err(format!("Invalid line in proposal block: {}", trimmed));
                }
            }
            BlockType::Execution => {
                execution_block_lines.push(trimmed.to_string());
            }
        }
    }

     if current_block != BlockType::Root {
        return Err("Unclosed execution block.".to_string());
    }

    Ok((properties, attachments, execution_ops))
}


// Expand the proposal_lifecycle macro
// Note: This assumes it's called with the lines *inside* the braces {}
pub fn expand_proposal_lifecycle(lines: &[&str]) -> Result<Vec<Op>, String> {
    let (properties, attachments, _execution_ops) = parse_proposal_block(lines)?; // TODO: Use execution_ops later

    // --- 1. Create ProposalLifecycle Object ---
    // Generate ID (compile time - placeholder, runtime generation preferred)
    let proposal_id = Utc::now().timestamp_millis() as u64;

    let title = properties.get("title").cloned().ok_or("Missing 'title' property")?;
    let quorum_str = properties.get("quorum").ok_or("Missing 'quorum' property")?;
    let threshold_str = properties.get("threshold").ok_or("Missing 'threshold' property")?;

    let quorum = quorum_str.parse::<u64>().map_err(|_| format!("Invalid quorum value: {}", quorum_str))?;
    let threshold = threshold_str.parse::<u64>().map_err(|_| format!("Invalid threshold value: {}", threshold_str))?;

    // Optional properties
    let discussion_duration = properties.get("discussion_duration").and_then(|s| parse_duration(s));
    let expires_at = properties.get("expires_at").and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)));

    // TODO: Get creator from an assumed context or macro arg? Using placeholder.
    let creator = "macro_creator".to_string();

    let proposal = ProposalLifecycle::new(
        proposal_id,
        creator,
        title,
        quorum,
        threshold,
        discussion_duration,
        None, // required_participants not handled yet
    );

    // --- 2. Generate StoreP for Lifecycle ---
    let proposal_json = serde_json::to_string(&proposal)
        .map_err(|e| format!("Failed to serialize proposal lifecycle: {}", e))?;
    let lifecycle_key = format!("governance/proposals/{}/lifecycle", proposal_id);
    // Escape JSON string for DSL - Use double quotes for replacements
    let escaped_json = proposal_json.replace("\\", "\\\\").replace("\"", "\\\"");
    let store_lifecycle_op = format!("StoreP \"{}\" \"{}\"", lifecycle_key, escaped_json);

    let mut ops = vec![
        Op::Emit(format!("Creating proposal {}...", proposal_id)), // Debug emit
    ];
    ops.extend(parse_dsl(&store_lifecycle_op)?);


    // --- 3. Generate StoreP for Attachments ---
    let namespace = "governance";
    for (name, file_path_str) in attachments {
        let file_path = Path::new(&file_path_str);
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read attachment file '{}': {}", file_path_str, e))?;

        // Basic sanitization for attachment name used in key
        let sanitized_name = name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let attachment_key = format!("proposals/{}/attachments/{}", proposal_id, sanitized_name);

        // Escape content for DSL string
        let escaped_content = content.replace("\\", "\\\\").replace("\"", "\\\"");
        // Corrected format string syntax for StoreP
        let store_attachment_op = format!("StoreP \"{}\"/\"{}\" \"{}\"", namespace, attachment_key, escaped_content);
        ops.extend(parse_dsl(&store_attachment_op)?);
        ops.push(Op::Emit(format!("Stored attachment '{}' from {}", name, file_path_str)));
    }

    // --- 4. Emit Event for Metadata (Optional) ---
    // Example: Emit event with proposal ID and title
    let event_msg = format!("Proposal {} created: {}", proposal_id, proposal.title);
    ops.push(Op::EmitEvent { category: "governance".to_string(), message: event_msg });


    // TODO: Add lifecycle validation Ops?
    // TODO: Incorporate execution_ops (likely needs IfPassed Op or similar structure)

    Ok(ops)
}

// Helper to parse duration strings like "2d", "3h", "30m"
fn parse_duration(duration_str: &str) -> Option<Duration> {
    let duration_str = duration_str.trim();
    if duration_str.ends_with('d') {
        duration_str.trim_end_matches('d').parse::<i64>().ok().map(Duration::days)
    } else if duration_str.ends_with('h') {
        duration_str.trim_end_matches('h').parse::<i64>().ok().map(Duration::hours)
    } else if duration_str.ends_with('m') {
         duration_str.trim_end_matches('m').parse::<i64>().ok().map(Duration::minutes)
    } else {
        None // Or handle seconds, or return error
    }
}


// Main macro expansion function - needs modification to call expand_proposal_lifecycle
pub fn macro_expand(macro_name: &str, lines: &[&str]) -> Result<Vec<Op>, String> {
     match macro_name {
         "proposal_lifecycle" => expand_proposal_lifecycle(lines),
         // Add other macros here
         _ => Err(format!("Unknown macro: {}", macro_name)),
     }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests for the new expand_proposal_lifecycle function
    // Need to mock fs::read_to_string or create dummy files
} 