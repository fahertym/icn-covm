use super::{common, line_parser, CompilerError, SourcePosition};
use crate::vm::Op;

/// Parse a while statement block
pub fn parse_while_block(
    lines: &[String],
    current_line: &mut usize,
    pos: SourcePosition,
) -> Result<Op, CompilerError> {
    let mut condition = Vec::new();
    let mut body = Vec::new();
    let current_indent = common::get_indent(&lines[*current_line]);
    let mut has_explicit_condition = false;

    // Skip the "while:" line
    *current_line += 1;

    // Parse the body, looking for an explicit condition: block
    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = common::get_indent(line);

        if indent <= current_indent {
            break;
        }

        // Check for condition: marker
        if line.trim() == "condition:" {
            has_explicit_condition = true;
            let condition_pos = SourcePosition::new(pos.line + *current_line, indent + 1);
            *current_line += 1;

            // Parse condition block
            condition = line_parser::parse_block(lines, current_line, indent, condition_pos)?;
        } else if line.trim().ends_with(':') {
            // Handle nested block structures
            let nested_pos = SourcePosition::new(pos.line + *current_line, indent + 1);

            if line.trim() == "if:" {
                let nested_op = super::if_block::parse_if_block(lines, current_line, nested_pos)?;
                body.push(nested_op);
            } else if line.trim() == "while:" {
                let nested_op = parse_while_block(lines, current_line, nested_pos)?;
                body.push(nested_op);
            } else if line.trim().starts_with("loop ") {
                let nested_op =
                    super::loop_block::parse_loop_block(lines, current_line, nested_pos)?;
                body.push(nested_op);
            } else if line.trim() == "match:" {
                let nested_op =
                    super::match_block::parse_match_block(lines, current_line, nested_pos)?;
                body.push(nested_op);
            } else {
                return Err(CompilerError::UnknownBlockType(
                    line.trim().to_string(),
                    nested_pos.line,
                    nested_pos.column,
                ));
            }
        } else {
            // Regular statement in body
            let line_pos = SourcePosition::new(pos.line + *current_line, indent + 1);
            let op = line_parser::parse_line(line, line_pos)?;
            if !matches!(op, Op::Nop) {
                body.push(op);
            }
            *current_line += 1;
        }
    }

    // If no explicit condition was found, the first instruction in the body is assumed to be a condition
    if !has_explicit_condition && !body.is_empty() {
        // Move the first body instruction to the condition
        condition = vec![body.remove(0)];
    }

    Ok(Op::While { condition, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_while_block_parsing() {
        let source = vec![
            "while:".to_string(),
            "    condition:".to_string(),
            "        push 1".to_string(),
            "        push 0".to_string(),
            "        gt".to_string(),
            "    push 1".to_string(),
            "    sub".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let op = parse_while_block(&source, &mut current_line, pos).unwrap();

        match op {
            Op::While { condition, body } => {
                assert_eq!(condition.len(), 3);
                assert_eq!(body.len(), 2);
            }
            _ => panic!("Expected While operation"),
        }
    }

    #[test]
    fn test_nested_while_blocks() {
        let source = vec![
            "while:".to_string(),
            "    push 1".to_string(),
            "    while:".to_string(),
            "        push 2".to_string(),
            "        push 1".to_string(),
            "        sub".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let op = parse_while_block(&source, &mut current_line, pos).unwrap();

        match op {
            Op::While { condition, body } => {
                assert_eq!(condition.len(), 1);
                assert_eq!(body.len(), 1);

                // Check nested while
                match &body[0] {
                    Op::While {
                        condition: nested_condition,
                        body: nested_body,
                    } => {
                        assert_eq!(nested_condition.len(), 1);
                        assert_eq!(nested_body.len(), 2);
                    }
                    _ => panic!("Expected nested While operation"),
                }
            }
            _ => panic!("Expected While operation"),
        }
    }
}
