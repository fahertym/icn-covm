use crate::compiler::{CompilerError, SourcePosition};
use crate::compiler::line_parser::{parse_line, parse_block};
use crate::vm::Op;
use chrono::Duration;
use std::collections::HashMap;

/// Configuration for proposal lifecycle extracted from governance blocks in DSL
#[derive(Debug, Default, Clone)]
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

/// Parse a duration string like "72h" or "14d" into a chrono::Duration
fn parse_duration(duration_str: &str) -> Result<Duration, CompilerError> {
    let duration_str = duration_str.trim();
    if duration_str.is_empty() {
        return Err(CompilerError::SyntaxError {
            details: "Empty duration string".to_string(),
        });
    }

    let last_char = duration_str.chars().last().unwrap();
    let value = &duration_str[0..duration_str.len() - 1];
    
    let value: i64 = value.parse().map_err(|_| CompilerError::SyntaxError {
        details: format!("Invalid duration value: {}", value),
    })?;

    match last_char {
        'h' => Ok(Duration::hours(value)),
        'd' => Ok(Duration::days(value)),
        'w' => Ok(Duration::weeks(value)),
        _ => Err(CompilerError::SyntaxError {
            details: format!("Unknown duration unit: {}", last_char),
        }),
    }
}

/// Parse DSL source into a vector of operations and lifecycle configuration
///
/// This function parses the provided DSL source code and extracts both
/// the executable operations and any governance configuration.
///
/// # Arguments
///
/// * `source` - The DSL source code to parse
///
/// # Returns
///
/// * `Result<(Vec<Op>, LifecycleConfig), CompilerError>` - The parsed operations, 
///   lifecycle configuration, or an error
///
/// # Example
///
/// ```
/// use icn_covm::compiler::parse_dsl;
///
/// let source = "
///     governance {
///         quorumthreshold 0.6
///         votethreshold 0.5
///         mindeliberation 72h
///         expiresin 14d
///         require_role \"member\"
///     }
///
///     push 10
///     push 20
///     add
/// ";
///
/// let (ops, config) = parse_dsl(source).unwrap();
/// ```
pub fn parse_dsl(source: &str) -> Result<(Vec<Op>, LifecycleConfig), CompilerError> {
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let mut current_line = 0;
    let mut ops = Vec::new();
    let mut config = LifecycleConfig::default();
    let mut in_governance_block = false;
    let mut governance_block_start = 0;
    let mut governance_block_indent = 0;

    while current_line < lines.len() {
        let line = &lines[current_line];
        if line.trim().is_empty() {
            current_line += 1;
            continue;
        }

        let indent = crate::compiler::common::get_indent(line);
        let pos = SourcePosition::new(current_line + 1, indent + 1);
        let trimmed_line = line.trim();

        if trimmed_line == "governance {" {
            // Start of governance block
            in_governance_block = true;
            governance_block_start = current_line;
            governance_block_indent = indent;
            current_line += 1;
            continue;
        } else if in_governance_block && trimmed_line == "}" {
            // End of governance block
            in_governance_block = false;
            current_line += 1;
            continue;
        } else if in_governance_block {
            // Inside governance block, parse governance-specific commands
            let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
            if parts.is_empty() {
                current_line += 1;
                continue;
            }

            match parts[0] {
                "quorumthreshold" => {
                    if parts.len() < 2 {
                        return Err(CompilerError::MissingParameter(
                            "quorumthreshold".to_string(),
                            pos.line,
                            pos.column,
                        ));
                    }
                    let quorum = parts[1].parse::<f64>().map_err(|_| {
                        CompilerError::InvalidParameterValue(
                            "quorumthreshold".to_string(),
                            pos.line,
                            pos.column,
                        )
                    })?;
                    config.quorum = Some(quorum);
                }
                "votethreshold" => {
                    if parts.len() < 2 {
                        return Err(CompilerError::MissingParameter(
                            "votethreshold".to_string(),
                            pos.line,
                            pos.column,
                        ));
                    }
                    let threshold = parts[1].parse::<f64>().map_err(|_| {
                        CompilerError::InvalidParameterValue(
                            "votethreshold".to_string(),
                            pos.line,
                            pos.column,
                        )
                    })?;
                    config.threshold = Some(threshold);
                }
                "mindeliberation" => {
                    if parts.len() < 2 {
                        return Err(CompilerError::MissingParameter(
                            "mindeliberation".to_string(),
                            pos.line,
                            pos.column,
                        ));
                    }
                    let duration = parse_duration(parts[1])?;
                    config.min_deliberation = Some(duration);
                }
                "expiresin" => {
                    if parts.len() < 2 {
                        return Err(CompilerError::MissingParameter(
                            "expiresin".to_string(),
                            pos.line,
                            pos.column,
                        ));
                    }
                    let duration = parse_duration(parts[1])?;
                    config.expires_in = Some(duration);
                }
                "require_role" => {
                    if parts.len() < 2 {
                        return Err(CompilerError::MissingParameter(
                            "require_role".to_string(),
                            pos.line,
                            pos.column,
                        ));
                    }
                    let role = if parts[1].starts_with('"') && parts[1].ends_with('"') {
                        parts[1][1..parts[1].len() - 1].to_string()
                    } else {
                        parts[1].to_string()
                    };
                    config.required_roles.push(role);
                }
                _ => {
                    return Err(CompilerError::UnknownCommand(
                        parts[0].to_string(),
                        pos.line,
                        pos.column,
                    ));
                }
            }
            current_line += 1;
            continue;
        } else if trimmed_line.ends_with(':') {
            // Handle standard block types
            let op = if trimmed_line == "if:" {
                crate::compiler::if_block::parse_if_block(&lines, &mut current_line, pos)?
            } else if trimmed_line == "while:" {
                crate::compiler::while_block::parse_while_block(&lines, &mut current_line, pos)?
            } else if trimmed_line.starts_with("def ") {
                crate::compiler::function_block::parse_function_block(&lines, &mut current_line, pos)?
            } else if trimmed_line == "match:" {
                crate::compiler::match_block::parse_match_block(&lines, &mut current_line, pos)?
            } else if trimmed_line.starts_with("loop ") {
                crate::compiler::loop_block::parse_loop_block(&lines, &mut current_line, pos)?
            } else {
                return Err(CompilerError::UnknownBlockType(
                    trimmed_line.to_string(),
                    pos.line,
                    pos.column,
                ));
            };

            if !matches!(op, Op::Nop) {
                ops.push(op);
            }
            // current_line is already incremented by the block parser
        } else {
            // Regular line
            let op = parse_line(line, pos)?;
            if !matches!(op, Op::Nop) {
                ops.push(op);
            }
            current_line += 1;
        }
    }

    Ok((ops, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("72h").unwrap(), Duration::hours(72));
        assert_eq!(parse_duration("14d").unwrap(), Duration::days(14));
        assert_eq!(parse_duration("2w").unwrap(), Duration::weeks(2));
        
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("72x").is_err());
    }

    #[test]
    fn test_parse_governance_block() {
        let source = r#"
governance {
    quorumthreshold 0.6
    votethreshold 0.5
    mindeliberation 72h
    expiresin 14d
    require_role "member"
}

# Regular DSL code
push 10
push 20
add
"#;

        let (ops, config) = parse_dsl(source).unwrap();
        
        // Check parsed config
        assert_eq!(config.quorum, Some(0.6));
        assert_eq!(config.threshold, Some(0.5));
        assert_eq!(config.min_deliberation, Some(Duration::hours(72)));
        assert_eq!(config.expires_in, Some(Duration::days(14)));
        assert_eq!(config.required_roles, vec!["member"]);
        
        // Check regular operations were parsed
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_parse_without_governance_block() {
        let source = r#"
# Regular DSL code
push 10
push 20
add
"#;

        let (ops, config) = parse_dsl(source).unwrap();
        
        // Check default config
        assert_eq!(config.quorum, None);
        assert_eq!(config.threshold, None);
        assert_eq!(config.min_deliberation, None);
        assert_eq!(config.expires_in, None);
        assert_eq!(config.required_roles.len(), 0);
        
        // Check regular operations were parsed
        assert_eq!(ops.len(), 3);
    }
} 