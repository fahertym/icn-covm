use crate::compiler::parse_dsl; // Use the correct path from parent module
use crate::governance::proposal_lifecycle::{ProposalLifecycle, ProposalState}; // Import necessary structs
use crate::vm::Op;
use chrono::{DateTime, Duration, Utc};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq)]
enum BlockType {
    Root,
    Execution,
}

// Helper to parse macro arguments within braces {}
// Returns properties, file attachments, and the raw execution DSL string
fn parse_proposal_block(
    lines: &[&str],
) -> Result<(HashMap<String, String>, HashMap<String, String>, String), String> {
    let mut properties: HashMap<String, String> = HashMap::new();
    let mut attachments: HashMap<String, String> = HashMap::new(); // name -> file_path
    let mut execution_dsl = String::new(); // Store raw DSL lines
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
                // Join the collected lines into the final DSL string
                execution_dsl = execution_block_lines.join("\n");
                execution_block_lines.clear(); // Clear for safety
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
                        }
                        "attachment" => {
                            let attachment_parts: Vec<&str> = value.splitn(2, ' ').collect();
                            if attachment_parts.len() == 2 {
                                let name = attachment_parts[0].trim_matches('"');
                                let path = attachment_parts[1].trim_matches('"');
                                // Prevent overwriting the dedicated 'logic' attachment key from the execution block
                                if name.eq_ignore_ascii_case("logic") {
                                    return Err("Cannot use 'logic' as a name for file attachment; it's reserved for the execution block.".to_string());
                                }
                                attachments.insert(name.to_string(), path.to_string());
                            } else {
                                return Err(format!("Invalid attachment format: {}", trimmed));
                            }
                        }
                        _ => return Err(format!("Unknown property in proposal block: {}", key)),
                    }
                } else {
                    return Err(format!("Invalid line in proposal block: {}", trimmed));
                }
            }
            BlockType::Execution => {
                // Collect raw lines for the execution block
                execution_block_lines.push(trimmed.to_string());
            }
        }
    }

    if current_block != BlockType::Root {
        return Err("Unclosed execution block.".to_string());
    }

    Ok((properties, attachments, execution_dsl))
}

// Expand the proposal_lifecycle macro
// Note: This assumes it's called with the lines *inside* the braces {}
pub fn expand_proposal_lifecycle(lines: &[&str]) -> Result<Vec<Op>, String> {
    let (properties, attachments, execution_dsl) = parse_proposal_block(lines)?;

    // --- 1. Create ProposalLifecycle Object ---
    let proposal_id = Utc::now().timestamp_millis().to_string(); // Use String for ID
    let title = properties
        .get("title")
        .cloned()
        .ok_or("Missing 'title' property")?;
    let quorum_str = properties
        .get("quorum")
        .ok_or("Missing 'quorum' property")?;
    let threshold_str = properties
        .get("threshold")
        .ok_or("Missing 'threshold' property")?;
    let quorum = quorum_str
        .parse::<u64>()
        .map_err(|_| format!("Invalid quorum value: {}", quorum_str))?;
    let threshold = threshold_str
        .parse::<u64>()
        .map_err(|_| format!("Invalid threshold value: {}", threshold_str))?;
    let discussion_duration = properties
        .get("discussion_duration")
        .and_then(|s| parse_duration(s));
    
    // Create a basic Identity for the creator
    let creator_name = properties
        .get("author")
        .cloned()
        .unwrap_or_else(|| "macro_creator".to_string());
    
    // We need to create an actual Identity struct for the creator
    // Using a simple approach to create a basic Identity with just the name and type
    let creator = crate::identity::Identity::new(
        creator_name,
        None, // no full name
        "member".to_string(), // default type
        None, // no extra profile fields
    ).map_err(|e| format!("Failed to create identity: {}", e))?;
    
    let proposal = ProposalLifecycle::new(
        proposal_id.clone(),
        creator,
        title.clone(),
        quorum,
        threshold,
        discussion_duration,
        None, // no required participants
    );

    // --- 2. Generate StoreP for Lifecycle ---
    let proposal_json = serde_json::to_string(&proposal)
        .map_err(|e| format!("Failed to serialize proposal lifecycle: {}", e))?;
    let lifecycle_key = format!("governance/proposals/{}/lifecycle", proposal_id);
    let escaped_json = proposal_json.replace("\\", "\\\\").replace("\"", "\\\"");
    let store_lifecycle_op_str = format!("StoreP \"{}\" \"{}\"", lifecycle_key, escaped_json);

    let mut ops = vec![Op::Emit(format!("Creating proposal {}...", proposal_id))];
    // Map CompilerError to String
    ops.extend(parse_dsl(&store_lifecycle_op_str).map_err(|e| e.to_string())?);

    // --- 3. Generate StoreP for File Attachments ---
    let namespace = "governance";
    for (name, file_path_str) in attachments {
        let file_path = Path::new(&file_path_str);
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read attachment file '{}': {}", file_path_str, e))?;
        let sanitized_name =
            name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
        let attachment_key = format!("proposals/{}/attachments/{}", proposal_id, sanitized_name);
        let escaped_content = content.replace("\\", "\\\\").replace("\"", "\\\"");
        let store_attachment_op_str = format!(
            "StoreP \"{}\"/\"{}\" \"{}\"",
            namespace, attachment_key, escaped_content
        );
        // Map CompilerError to String
        ops.extend(parse_dsl(&store_attachment_op_str).map_err(|e| e.to_string())?);
        ops.push(Op::Emit(format!(
            "Stored attachment '{}' from {}",
            name, file_path_str
        )));
    }

    // --- 4. Generate StoreP for Execution Block Logic ---
    if !execution_dsl.is_empty() {
        let logic_attachment_key = format!("proposals/{}/attachments/logic", proposal_id);
        println!(
            "[MACRO] Storing execution DSL ({} bytes) to key: {}/{}...",
            execution_dsl.len(),
            namespace,
            logic_attachment_key
        );
        let escaped_dsl = execution_dsl.replace("\\", "\\\\").replace("\"", "\\\"");
        let store_logic_op_str = format!(
            "StoreP \"{}\"/\"{}\" \"{}\"",
            namespace, logic_attachment_key, escaped_dsl
        );
        // Map CompilerError to String
        ops.extend(parse_dsl(&store_logic_op_str).map_err(|e| e.to_string())?);
        // Optional: Emit event for stored logic
        ops.push(Op::EmitEvent {
            category: "governance".to_string(),
            message: format!("Stored execution logic for proposal {}", proposal_id),
        });
    } else {
        println!(
            "[MACRO] No execution block found for proposal {}.",
            proposal_id
        );
    }

    // --- 5. Emit Event for Metadata ---
    let event_msg = format!("Proposal {} ('{}') created.", proposal_id, proposal.title);
    ops.push(Op::EmitEvent {
        category: "governance".to_string(),
        message: event_msg,
    });

    // TODO: Add lifecycle validation Ops?
    // TODO: Incorporate execution_dsl into an IfPassed Op?

    Ok(ops)
}

// Helper to parse duration strings like "2d", "3h", "30m"
fn parse_duration(duration_str: &str) -> Option<Duration> {
    let duration_str = duration_str.trim();
    if duration_str.ends_with('d') {
        duration_str
            .trim_end_matches('d')
            .parse::<i64>()
            .ok()
            .map(Duration::days)
    } else if duration_str.ends_with('h') {
        duration_str
            .trim_end_matches('h')
            .parse::<i64>()
            .ok()
            .map(Duration::hours)
    } else if duration_str.ends_with('m') {
        duration_str
            .trim_end_matches('m')
            .parse::<i64>()
            .ok()
            .map(Duration::minutes)
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

/// Represents a proposal lifecycle macro with execution blocks and conditional execution paths
#[derive(Debug, Clone)]
pub struct ProposalLifecycleMacro {
    /// Unique identifier for the proposal
    pub proposal_id: String,
    
    /// Required quorum percentage (0.0-1.0) for the proposal to be valid
    pub quorum: f64,
    
    /// Required threshold percentage (0.0-1.0) for the proposal to pass
    pub threshold: f64,
    
    /// Block of code to execute when the proposal is processed
    pub execution_block: Vec<String>,
    
    /// Title of the proposal
    pub title: String,
    
    /// Creator of the proposal
    pub created_by: String,
    
    /// Timestamp when the proposal was created
    pub created_at: f64,
    
    /// Block of code to execute if the proposal passes
    pub passed_block: Vec<String>,
    
    /// Optional block of code to execute if the proposal fails
    pub failed_block: Option<Vec<String>>,
}

impl ProposalLifecycleMacro {
    /// Create a new ProposalLifecycleMacro instance
    pub fn new(
        proposal_id: String,
        quorum: f64,
        threshold: f64,
        execution_block: Vec<String>,
        title: String,
        created_by: String,
        created_at: f64,
        passed_block: Vec<String>,
        failed_block: Option<Vec<String>>,
    ) -> Self {
        Self {
            proposal_id,
            quorum,
            threshold,
            execution_block,
            title,
            created_by,
            created_at,
            passed_block,
            failed_block,
        }
    }
    
    /// Process this macro and generate the operations for VM execution
    pub fn process(&self) -> Result<Vec<Op>, String> {
        // Call the existing expand_proposal_lifecycle function
        // We'll use the blocks from self to construct the input
        let mut lines = Vec::new();
        
        // Add metadata properties
        lines.push(format!("title {}", self.title));
        lines.push(format!("quorum {}", self.quorum));
        lines.push(format!("threshold {}", self.threshold));
        lines.push(format!("author {}", self.created_by));
        
        // Add the execution block lines
        for line in &self.execution_block {
            lines.push(line.clone());
        }
        
        // Add if passed block if present
        if !self.passed_block.is_empty() {
            lines.push("if passed:".to_string());
            for line in &self.passed_block {
                lines.push(format!("    {}", line)); // Add indentation
            }
        }
        
        // Add else block if present
        if let Some(failed_lines) = &self.failed_block {
            if !failed_lines.is_empty() {
                lines.push("else:".to_string());
                for line in failed_lines {
                    lines.push(format!("    {}", line)); // Add indentation
                }
            }
        }
        
        // Convert string lines to string slices for expand_proposal_lifecycle
        let lines_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        
        // Use the existing function to process the macro
        expand_proposal_lifecycle(&lines_refs)
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests for the new expand_proposal_lifecycle function
    // Need to mock fs::read_to_string or create dummy files
}
