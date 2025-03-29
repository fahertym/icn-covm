use crate::vm::Op;
use super::{CompilerError, SourcePosition, common, line_parser};

/// Parse a loop statement block
pub fn parse_loop_block(lines: &[String], current_line: &mut usize, pos: SourcePosition) -> Result<Op, CompilerError> {
    // Parse the "loop N:" line, extracting N
    let line = &lines[*current_line];
    let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
    if parts.len() != 2 || !parts[0].eq_ignore_ascii_case("loop") {
        return Err(CompilerError::InvalidLoopFormat(line.trim().to_string(), pos.line, pos.column));
    }
    
    let count_str = parts[1].trim_end_matches(':');
    let count = count_str.parse::<usize>()
        .map_err(|_| CompilerError::InvalidLoopCount(
            count_str.to_string(), 
            pos.line, 
            common::adjusted_position(pos, line, count_str).column
        ))?;
    
    let current_indent = common::get_indent(line);
    
    // Skip the "loop N:" line
    *current_line += 1;
    
    // Parse the body
    let body = line_parser::parse_block(lines, current_line, current_indent, pos)?;
    
    Ok(Op::Loop { count, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_loop_block_parsing() {
        let source = vec![
            "loop 5:".to_string(),
            "    push 1".to_string(),
            "    push 2".to_string(),
            "    add".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let op = parse_loop_block(&source, &mut current_line, pos).unwrap();
        
        match op {
            Op::Loop { count, body } => {
                assert_eq!(count, 5);
                assert_eq!(body.len(), 3);
            },
            _ => panic!("Expected Loop operation"),
        }
    }
    
    #[test]
    fn test_nested_loop_blocks() {
        let source = vec![
            "loop 3:".to_string(),
            "    push 1".to_string(),
            "    loop 2:".to_string(),
            "        push 2".to_string(),
            "        add".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let op = parse_loop_block(&source, &mut current_line, pos).unwrap();
        
        match op {
            Op::Loop { count, body } => {
                assert_eq!(count, 3);
                assert_eq!(body.len(), 2); // push 1 and nested loop
                
                // Check nested loop
                match &body[1] {
                    Op::Loop { count: nested_count, body: nested_body } => {
                        assert_eq!(*nested_count, 2);
                        assert_eq!(nested_body.len(), 2);
                    },
                    _ => panic!("Expected nested Loop operation"),
                }
            },
            _ => panic!("Expected Loop operation"),
        }
    }
    
    #[test]
    fn test_invalid_loop_count() {
        let source = vec![
            "loop abc:".to_string(),
            "    push 1".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let result = parse_loop_block(&source, &mut current_line, pos);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            CompilerError::InvalidLoopCount(_, line, _) => {
                assert_eq!(line, 1);
            },
            err => panic!("Expected InvalidLoopCount error, got {:?}", err),
        }
    }
} 