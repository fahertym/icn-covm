use crate::vm::Op;
use super::{CompilerError, SourcePosition, common, line_parser};

/// Parse a function definition block
pub fn parse_function_block(lines: &[String], current_line: &mut usize, pos: SourcePosition) -> Result<Op, CompilerError> {
    let line = &lines[*current_line];
    
    // Expected format: def name(param1, param2):
    if !line.contains('(') || !line.contains(')') {
        return Err(CompilerError::InvalidFunctionFormat(line.to_string(), pos.line, pos.column));
    }
    
    // Extract name and parameters
    let name_params = parse_function_signature(line, pos)?;
    let name = name_params.0;
    let params = name_params.1;
    
    let current_indent = common::get_indent(line);
    *current_line += 1;
    
    // Parse function body
    let body = line_parser::parse_block(lines, current_line, current_indent, pos)?;
    
    Ok(Op::Def {
        name,
        params,
        body,
    })
}

/// Helper function to parse function signature
pub fn parse_function_signature(line: &str, pos: SourcePosition) -> Result<(String, Vec<String>), CompilerError> {
    // Format: def name(x, y):
    let parts: Vec<&str> = line.trim_end_matches(':').splitn(2, '(').collect();
    if parts.len() != 2 {
        return Err(CompilerError::InvalidFunctionDefinition(line.to_string(), pos.line, pos.column));
    }

    let name_part = parts[0].trim();
    if !name_part.starts_with("def ") {
        return Err(CompilerError::InvalidFunctionStart(line.to_string(), pos.line, pos.column));
    }
    
    let name = name_part["def ".len()..].trim().to_string();
    
    // Extract parameters
    let params_str = parts[1].trim_end_matches(')');
    let params: Vec<String> = params_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok((name, params))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function_block_parsing() {
        let source = vec![
            "def add(x, y):".to_string(),
            "    load x".to_string(),
            "    load y".to_string(),
            "    add".to_string(),
            "    return".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let op = parse_function_block(&source, &mut current_line, pos).unwrap();
        
        match op {
            Op::Def { name, params, body } => {
                assert_eq!(name, "add");
                assert_eq!(params, vec!["x".to_string(), "y".to_string()]);
                assert_eq!(body.len(), 4);
            },
            _ => panic!("Expected Def operation"),
        }
    }
    
    #[test]
    fn test_function_without_params() {
        let source = vec![
            "def constant():".to_string(),
            "    push 42".to_string(),
            "    return".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let op = parse_function_block(&source, &mut current_line, pos).unwrap();
        
        match op {
            Op::Def { name, params, body } => {
                assert_eq!(name, "constant");
                assert_eq!(params.len(), 0);
                assert_eq!(body.len(), 2);
            },
            _ => panic!("Expected Def operation"),
        }
    }
    
    #[test]
    fn test_invalid_function_signature() {
        let source = vec![
            "def invalid".to_string(),
            "    push 1".to_string(),
        ];
        
        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);
        
        let result = parse_function_block(&source, &mut current_line, pos);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            CompilerError::InvalidFunctionFormat(_, line, _) => {
                assert_eq!(line, 1);
            },
            err => panic!("Expected InvalidFunctionFormat error, got {:?}", err),
        }
    }
} 