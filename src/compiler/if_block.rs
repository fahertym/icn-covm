use crate::vm::Op;
use super::{CompilerError, SourcePosition, common, line_parser};

/// Parse an if statement block
pub fn parse_if_block(lines: &[String], current_line: &mut usize, pos: SourcePosition) -> Result<Op, CompilerError> {
    let mut condition = Vec::new();
    let mut then_block = Vec::new();
    let mut else_block = None;
    let current_indent = common::get_indent(&lines[*current_line]);

    // Skip the "if:" line
    *current_line += 1;

    // Parse the then block
    then_block = line_parser::parse_block(lines, current_line, current_indent, pos)?;

    // Check for else block
    if *current_line < lines.len() && lines[*current_line].trim() == "else:" {
        let else_pos = SourcePosition::new(pos.line + *current_line, common::get_indent(&lines[*current_line]) + 1);
        *current_line += 1;
        
        let else_ops = line_parser::parse_block(lines, current_line, current_indent, else_pos)?;
        else_block = Some(else_ops);
    }

    Ok(Op::If {
        condition,
        then: then_block,
        else_: else_block,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_if_block_parsing() {
        let source = vec![
            "if:".to_string(),
            "    push 1".to_string(),
            "    push 2".to_string(),
            "    add".to_string(),
            "else:".to_string(),
            "    push 0".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let op = parse_if_block(&source, &mut current_line, pos).unwrap();
        
        match op {
            Op::If { condition: _, then, else_ } => {
                assert_eq!(then.len(), 3);
                assert!(else_.is_some());
                let else_block = else_.unwrap();
                assert_eq!(else_block.len(), 1);
            },
            _ => panic!("Expected If operation"),
        }
    }
    
    #[test]
    fn test_nested_if_blocks() {
        let source = vec![
            "if:".to_string(),
            "    push 1".to_string(),
            "    if:".to_string(),
            "        push 2".to_string(),
            "        add".to_string(),
            "    else:".to_string(),
            "        push 3".to_string(),
            "else:".to_string(),
            "    push 0".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let op = parse_if_block(&source, &mut current_line, pos).unwrap();
        
        match op {
            Op::If { condition: _, then, else_ } => {
                assert_eq!(then.len(), 2); // push 1 and nested if
                assert!(else_.is_some());
                
                // Check nested if
                match &then[1] {
                    Op::If { condition: _, then: nested_then, else_: nested_else } => {
                        assert_eq!(nested_then.len(), 2);
                        assert!(nested_else.is_some());
                    },
                    _ => panic!("Expected nested If operation"),
                }
            },
            _ => panic!("Expected If operation"),
        }
    }
} 