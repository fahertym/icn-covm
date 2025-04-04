use super::{common, CompilerError, SourcePosition};
use crate::vm::Op;

/// Parse a single line of DSL code
pub fn parse_line(line: &str, pos: SourcePosition) -> Result<Op, CompilerError> {
    // Skip comments
    if line.trim().starts_with('#') {
        return Ok(Op::Nop);
    }

    let mut parts = line.split_whitespace();
    let command = match parts.next() {
        Some(cmd) => cmd,
        None => return Ok(Op::Nop),
    };

    match command {
        "push" => {
            let num_str = parts
                .next()
                .ok_or(CompilerError::MissingPushValue(pos.line, pos.column))?;
            let num = num_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidPushValue(
                    num_str.to_string(),
                    pos.line,
                    common::adjusted_position(pos, line, num_str).column,
                )
            })?;
            Ok(Op::Push(num))
        }
        "emit" => {
            if let Some(inner) = line.find('"') {
                let inner = &line[inner + 1..line.rfind('"').unwrap_or(line.len())];
                Ok(Op::Emit(inner.to_string()))
            } else {
                Err(CompilerError::MissingEmitQuotes(pos.line, pos.column))
            }
        }
        "emitevent" => {
            // Format: emitevent "category" "message"
            let line_str = line.to_string();
            let parts: Vec<&str> = line_str.split('"').collect();
            if parts.len() < 5 {
                return Err(CompilerError::InvalidEmitEventFormat(pos.line, pos.column));
            }

            let category = parts[1].trim().to_string();
            let message = parts[3].trim().to_string();

            Ok(Op::EmitEvent { category, message })
        }
        "assertequalstack" => {
            let depth_str = parts
                .next()
                .ok_or(CompilerError::MissingAssertDepth(pos.line, pos.column))?;
            let depth = depth_str.parse::<usize>().map_err(|_| {
                CompilerError::InvalidAssertDepth(
                    depth_str.to_string(),
                    pos.line,
                    common::adjusted_position(pos, line, depth_str).column,
                )
            })?;

            if depth < 2 {
                return Err(CompilerError::InsufficientAssertDepth(pos.line, pos.column));
            }

            Ok(Op::AssertEqualStack { depth })
        }
        "break" => Ok(Op::Break),
        "continue" => Ok(Op::Continue),
        "load" => {
            let var_name = parts.next().ok_or(CompilerError::MissingVariable(
                "load".to_string(),
                pos.line,
                pos.column,
            ))?;
            Ok(Op::Load(var_name.to_string()))
        }
        "store" => {
            let var_name = parts.next().ok_or(CompilerError::MissingVariable(
                "store".to_string(),
                pos.line,
                pos.column,
            ))?;
            Ok(Op::Store(var_name.to_string()))
        }
        "add" => Ok(Op::Add),
        "sub" => Ok(Op::Sub),
        "mul" => Ok(Op::Mul),
        "div" => Ok(Op::Div),
        "mod" => Ok(Op::Mod),
        "eq" => Ok(Op::Eq),
        "gt" => Ok(Op::Gt),
        "lt" => Ok(Op::Lt),
        "not" => Ok(Op::Not),
        "and" => Ok(Op::And),
        "or" => Ok(Op::Or),
        "negate" => Ok(Op::Negate),
        "dup" => Ok(Op::Dup),
        "swap" => Ok(Op::Swap),
        "over" => Ok(Op::Over),
        "pop" => Ok(Op::Pop),
        "return" => Ok(Op::Return),
        "call" => {
            let fn_name = parts
                .next()
                .ok_or(CompilerError::MissingFunctionName(pos.line, pos.column))?;
            Ok(Op::Call(fn_name.to_string()))
        }
        "dumpstack" => Ok(Op::DumpStack),
        "dumpmemory" => Ok(Op::DumpMemory),
        "dumpstate" => Ok(Op::DumpState), // Debug/introspection opcode
        "rankedvote" => {
            // Parse rankedvote command with required parameters: candidates and ballots
            let candidates_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "rankedvote requires 'candidates' parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let ballots_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "rankedvote requires 'ballots' parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            // Parse candidates parameter
            let candidates = candidates_str.parse::<usize>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid candidates count: {}", candidates_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Parse ballots parameter
            let ballots = ballots_str.parse::<usize>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid ballots count: {}", ballots_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Create RankedVote operation
            Ok(Op::RankedVote {
                candidates,
                ballots,
            })
        }
        "liquiddelegate" => {
            // Parse liquiddelegate command with required parameters: from and to
            let from_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "liquiddelegate requires 'from' parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let to_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "liquiddelegate requires 'to' parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            // Create LiquidDelegate operation
            Ok(Op::LiquidDelegate {
                from: from_str.to_string(),
                to: to_str.to_string(),
            })
        }
        "votethreshold" => {
            // Parse votethreshold command with required threshold parameter
            let threshold_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "votethreshold requires threshold parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            // Parse threshold parameter
            let threshold = threshold_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid threshold value: {}", threshold_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Create VoteThreshold operation
            Ok(Op::VoteThreshold(threshold))
        }
        "quorumthreshold" => {
            // Parse quorumthreshold command with required threshold parameter
            let threshold_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "quorumthreshold requires threshold parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            // Parse threshold parameter
            let threshold = threshold_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid threshold value: {}", threshold_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Create QuorumThreshold operation
            Ok(Op::QuorumThreshold(threshold))
        }
        "storep" => {
            let key = parts.next().ok_or(CompilerError::MissingVariable(
                "storep".to_string(),
                pos.line,
                pos.column,
            ))?;
            Ok(Op::StoreP(key.to_string()))
        }
        "loadp" => {
            let key = parts.next().ok_or(CompilerError::MissingVariable(
                "loadp".to_string(),
                pos.line,
                pos.column,
            ))?;
            Ok(Op::LoadP(key.to_string()))
        }
        "loadversionp" => {
            // Parse loadversionp command with key and version number
            let key = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "loadversionp requires key parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let version_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "loadversionp requires version parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let version = version_str.parse::<u64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Version must be a positive integer, got: {}", version_str),
                    pos.line,
                    pos.column,
                )
            })?;

            Ok(Op::LoadVersionP {
                key: key.to_string(),
                version,
            })
        }
        "listversionsP" => {
            // Parse listversionsP command with key parameter
            let key = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "listversionsP requires key parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            Ok(Op::ListVersionsP(key.to_string()))
        }
        "diffversionsp" => {
            // Parse diffversionsp command with key and two version numbers
            let key = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "diffversionsp requires key parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let v1_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "diffversionsp requires first version parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let v2_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "diffversionsp requires second version parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let v1 = v1_str.parse::<u64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("First version must be a positive integer, got: {}", v1_str),
                    pos.line,
                    pos.column,
                )
            })?;

            let v2 = v2_str.parse::<u64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Second version must be a positive integer, got: {}", v2_str),
                    pos.line,
                    pos.column,
                )
            })?;

            Ok(Op::DiffVersionsP {
                key: key.to_string(),
                v1,
                v2,
            })
        }
        "verifyidentity" => {
            // Parse verifyidentity command with required parameters
            let identity_id = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "verifyidentity requires identity_id parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            // Extract message parameter from quoted string
            let rest_of_line = line[line.find(identity_id).unwrap() + identity_id.len()..].trim();

            // Use a simple parser to extract two quoted strings
            if let Some(quote1_start) = rest_of_line.find('"') {
                if let Some(quote1_end) = rest_of_line[quote1_start + 1..].find('"') {
                    let quote1_end = quote1_start + 1 + quote1_end;
                    let message = rest_of_line[quote1_start + 1..quote1_end].to_string();

                    if let Some(quote2_start) = rest_of_line[quote1_end + 1..].find('"') {
                        let quote2_start = quote1_end + 1 + quote2_start;
                        if let Some(quote2_end) = rest_of_line[quote2_start + 1..].find('"') {
                            let quote2_end = quote2_start + 1 + quote2_end;
                            let signature = rest_of_line[quote2_start + 1..quote2_end].to_string();

                            return Ok(Op::VerifyIdentity {
                                identity_id: identity_id.to_string(),
                                message,
                                signature,
                            });
                        }
                    }
                }
            }

            Err(CompilerError::InvalidFunctionFormat(
                "verifyidentity requires quoted message and signature parameters".to_string(),
                pos.line,
                pos.column,
            ))
        }
        "checkmembership" => {
            // Parse checkmembership command with required parameters
            let identity_id = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "checkmembership requires identity_id parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let namespace = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "checkmembership requires namespace parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            Ok(Op::CheckMembership {
                identity_id: identity_id.to_string(),
                namespace: namespace.to_string(),
            })
        }
        "checkdelegation" => {
            // Parse checkdelegation command with required parameters
            let delegator_id = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "checkdelegation requires delegator_id parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let delegate_id = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "checkdelegation requires delegate_id parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            Ok(Op::CheckDelegation {
                delegator_id: delegator_id.to_string(),
                delegate_id: delegate_id.to_string(),
            })
        }
        "transfer" => {
            // Format: transfer resource_id from_account to_account amount "memo"
            // Memo is optional

            // Parse resource_id
            let resource_id = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "transfer requires resource_id parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse from_account
            let from_account = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "transfer requires from_account parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse to_account
            let to_account = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "transfer requires to_account parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse amount
            let amount_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "transfer requires amount parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let amount = amount_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid amount value: {}", amount_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Check for optional memo in quotes
            let memo = if line.contains('"') {
                let start_quote = line.find('"').unwrap();
                let end_quote = line.rfind('"').unwrap();
                if start_quote != end_quote {
                    Some(line[start_quote + 1..end_quote].to_string())
                } else {
                    None
                }
            } else {
                None
            };

            // Create Transfer operation
            Ok(Op::Transfer {
                resource_id,
                from_account,
                to_account,
                amount,
                memo,
            })
        }
        "mint" => {
            // Format: mint resource_id to_account amount "memo"
            // Memo is optional

            // Parse resource_id
            let resource_id = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "mint requires resource_id parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse to_account
            let to_account = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "mint requires to_account parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse amount
            let amount_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "mint requires amount parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let amount = amount_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid amount value: {}", amount_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Check for optional memo in quotes
            let memo = if line.contains('"') {
                let start_quote = line.find('"').unwrap();
                let end_quote = line.rfind('"').unwrap();
                if start_quote != end_quote {
                    Some(line[start_quote + 1..end_quote].to_string())
                } else {
                    None
                }
            } else {
                None
            };

            // Create Mint operation
            Ok(Op::Mint {
                resource_id,
                to_account,
                amount,
                memo,
            })
        }
        "burn" => {
            // Format: burn resource_id from_account amount "memo"
            // Memo is optional

            // Parse resource_id
            let resource_id = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "burn requires resource_id parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse from_account
            let from_account = parts
                .next()
                .ok_or(CompilerError::InvalidFunctionFormat(
                    "burn requires from_account parameter".to_string(),
                    pos.line,
                    pos.column,
                ))?
                .to_string();

            // Parse amount
            let amount_str = parts.next().ok_or(CompilerError::InvalidFunctionFormat(
                "burn requires amount parameter".to_string(),
                pos.line,
                pos.column,
            ))?;

            let amount = amount_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidFunctionFormat(
                    format!("Invalid amount value: {}", amount_str),
                    pos.line,
                    pos.column,
                )
            })?;

            // Check for optional memo in quotes
            let memo = if line.contains('"') {
                let start_quote = line.find('"').unwrap();
                let end_quote = line.rfind('"').unwrap();
                if start_quote != end_quote {
                    Some(line[start_quote + 1..end_quote].to_string())
                } else {
                    None
                }
            } else {
                None
            };

            // Create Burn operation
            Ok(Op::Burn {
                resource_id,
                from_account,
                amount,
                memo,
            })
        }
        _ => Err(CompilerError::UnknownCommand(
            command.to_string(),
            pos.line,
            pos.column,
        )),
    }
}

/// Parse a series of lines as a block of code
pub fn parse_block(
    lines: &[String],
    start_line: &mut usize,
    base_indent: usize,
    pos: SourcePosition,
) -> Result<Vec<Op>, CompilerError> {
    let mut block_ops = Vec::new();

    while *start_line < lines.len() {
        let line = &lines[*start_line];
        let indent = common::get_indent(line);
        //println!("  parse_block LOOP: line={}, indent={}, base_indent={}", *start_line, indent, base_indent); // DEBUG

        // If we've hit a non-empty line that is dedented, we're done with this block.
        // Skip empty lines entirely.
        if !line.trim().is_empty() && indent <= base_indent {
            //println!("  parse_block BREAK: Dedented non-empty line."); // DEBUG
            break;
        } else if line.trim().is_empty() {
            //println!("  parse_block SKIP: Empty line."); // DEBUG
            *start_line += 1; // Skip the empty line
            continue; // Continue to the next line
        }

        let current_pos = SourcePosition::new(pos.line + *start_line, indent + 1);

        // Detect and parse nested blocks
        if line.trim().ends_with(':') {
            let op = if line.trim() == "if:" {
                super::if_block::parse_if_block(lines, start_line, current_pos)?
            } else if line.trim() == "while:" {
                super::while_block::parse_while_block(lines, start_line, current_pos)?
            } else if line.trim().starts_with("def ") {
                super::function_block::parse_function_block(lines, start_line, current_pos)?
            } else if line.trim() == "match:" {
                super::match_block::parse_match_block(lines, start_line, current_pos)?
            } else if line.trim().starts_with("loop ") {
                super::loop_block::parse_loop_block(lines, start_line, current_pos)?
            } else {
                return Err(CompilerError::UnknownBlockType(
                    line.trim().to_string(),
                    current_pos.line,
                    current_pos.column,
                ));
            };

            if !matches!(op, Op::Nop) {
                block_ops.push(op);
            }

            // Don't increment start_line here since the block parser already did it
        } else {
            // Regular statements
            let op = parse_line(line, current_pos)?;
            if !matches!(op, Op::Nop) {
                block_ops.push(op);
            }
            *start_line += 1;
        }
    }
    //println!("  parse_block END: block_ops count={}", block_ops.len()); // DEBUG

    Ok(block_ops)
}

// Helper to parse quoted strings (handles both single and double quotes)
fn parse_quoted_string(input: &str) -> Result<String, CompilerError> {
    let trimmed = input.trim();

    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        // Remove the quotes
        let result = &trimmed[1..trimmed.len() - 1];
        Ok(result.to_string())
    } else {
        Err(CompilerError::SyntaxError {
            details: format!("Expected a quoted string, got: '{}'", input),
        })
    }
}
