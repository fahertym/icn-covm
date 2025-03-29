use super::{common, line_parser, CompilerError, SourcePosition};
use crate::vm::Op;

/// Parse a match statement block
pub fn parse_match_block(
    lines: &[String],
    current_line: &mut usize,
    pos: SourcePosition,
) -> Result<Op, CompilerError> {
    let mut value_ops = Vec::new();
    let mut cases = Vec::new();
    let mut default_ops = None;
    let current_indent = common::get_indent(&lines[*current_line]);

    // Skip the "match:" line
    *current_line += 1;

    // Parse the body of the match statement
    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = common::get_indent(line);

        if indent <= current_indent {
            break;
        }

        let line_pos = SourcePosition::new(pos.line + *current_line, indent + 1);

        if line.trim() == "value:" {
            *current_line += 1;
            // Parse the value block
            let value_indent = indent;
            let value_pos = SourcePosition::new(line_pos.line + 1, indent + 1);

            value_ops = line_parser::parse_block(lines, current_line, value_indent, value_pos)?;
        } else if line.trim().starts_with("case ") {
            // Parse case value
            let case_line = line.trim();
            let case_value_str = case_line[5..].trim().trim_end_matches(':');
            let case_value = case_value_str.parse::<f64>().map_err(|_| {
                CompilerError::InvalidCaseValue(
                    case_value_str.to_string(),
                    line_pos.line,
                    common::adjusted_position(line_pos, line, case_value_str).column,
                )
            })?;

            let case_indent = indent;
            *current_line += 1;

            // Parse case block
            let case_pos = SourcePosition::new(line_pos.line + 1, indent + 1);
            let case_ops = line_parser::parse_block(lines, current_line, case_indent, case_pos)?;

            cases.push((case_value, case_ops));
        } else if line.trim() == "default:" {
            *current_line += 1;
            let default_indent = indent;

            // Parse default block
            let default_pos = SourcePosition::new(line_pos.line + 1, indent + 1);
            let default_block =
                line_parser::parse_block(lines, current_line, default_indent, default_pos)?;

            default_ops = Some(default_block);
        } else {
            // If not in a special block, assume it's part of the value
            let op = line_parser::parse_line(line, line_pos)?;
            if !matches!(op, Op::Nop) {
                value_ops.push(op);
            }
            *current_line += 1;
        }
    }

    if value_ops.is_empty() {
        return Err(CompilerError::MissingMatchValue(pos.line, pos.column));
    }

    Ok(Op::Match {
        value: value_ops,
        cases,
        default: default_ops,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_block_parsing() {
        let source = vec![
            "match:".to_string(),
            "    value:".to_string(),
            "        push 2".to_string(),
            "    case 1:".to_string(),
            "        push 10".to_string(),
            "    case 2:".to_string(),
            "        push 20".to_string(),
            "    default:".to_string(),
            "        push 0".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let op = parse_match_block(&source, &mut current_line, pos).unwrap();

        match op {
            Op::Match { value, cases, default } => {
                assert_eq!(value.len(), 1);
                assert_eq!(cases.len(), 2);
                assert!(default.is_some());

                // Check case values
                assert_eq!(cases[0].0, 1.0);
                assert_eq!(cases[1].0, 2.0);

                // Check case blocks
                assert_eq!(cases[0].1.len(), 1);
                assert_eq!(cases[1].1.len(), 1);

                // Check default block
                let default_block = default.unwrap();
                assert_eq!(default_block.len(), 1);
            }
        }
    }

    #[test]
    fn test_match_without_default() {
        let source = vec![
            "match:".to_string(),
            "    value:".to_string(),
            "        push 2".to_string(),
            "    case 1:".to_string(),
            "        push 10".to_string(),
            "    case 2:".to_string(),
            "        push 20".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let op = parse_match_block(&source, &mut current_line, pos).unwrap();

        match op {
            Op::Match { value, cases, default } => {
                assert_eq!(value.len(), 1);
                assert_eq!(cases.len(), 2);
                assert!(default.is_none());
            }
        }
    }

    #[test]
    fn test_match_without_value_block() {
        let source = vec![
            "match:".to_string(),
            "    case 1:".to_string(),
            "        push 10".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let result = parse_match_block(&source, &mut current_line, pos);
        assert!(result.is_err());

        match result.unwrap_err() {
            CompilerError::SyntaxError(message, SourcePosition { line, column }) => {
                assert_eq!(message, "Expected value block in match statement");
                assert_eq!(line, 1);
                assert_eq!(column, 1);
            }
            err => panic!("Expected SyntaxError, got {:?}", err),
        }
    }
}
