use crate::vm::Op;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompilerError {
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    
    #[error("Unknown block type: {0}")]
    UnknownBlockType(String),
    
    #[error("Invalid function definition: {0}")]
    InvalidFunctionDefinition(String),
    
    #[error("Invalid function definition format: {0}")]
    InvalidFunctionFormat(String),
    
    #[error("Function definition must start with 'def': {0}")]
    InvalidFunctionStart(String),
    
    #[error("Missing number for push")]
    MissingPushValue,
    
    #[error("Invalid number for push: {0}")]
    InvalidPushValue(String),
    
    #[error("Missing quotes for emit command")]
    MissingEmitQuotes,
    
    #[error("Invalid format for emitevent, expected: emitevent \"category\" \"message\"")]
    InvalidEmitEventFormat,
    
    #[error("Missing variable for {0}")]
    MissingVariable(String),
    
    #[error("Missing function name for call")]
    MissingFunctionName,
    
    #[error("Missing depth for assertequalstack")]
    MissingAssertDepth,
    
    #[error("Invalid depth for assertequalstack: {0}")]
    InvalidAssertDepth(String),
    
    #[error("Depth for assertequalstack must be at least 2")]
    InsufficientAssertDepth,
    
    #[error("Invalid case value: {0}")]
    InvalidCaseValue(String),
    
    #[error("Match statement must have a value block")]
    MissingMatchValue,
}

pub fn parse_dsl(source: &str) -> Result<Vec<Op>, CompilerError> {
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let mut current_line = 0;
    let mut ops = Vec::new();

    while current_line < lines.len() {
        let line = &lines[current_line];
        if line.trim().is_empty() {
            current_line += 1;
            continue;
        }

        let op = if line.trim().ends_with(':') {
            if line.trim() == "if:" {
                parse_if_statement(&lines, &mut current_line)?
            } else if line.trim() == "while:" {
                parse_while_statement(&lines, &mut current_line)?
            } else if line.trim().starts_with("def ") {
                parse_function_definition(&lines, &mut current_line)?
            } else if line.trim() == "match:" {
                parse_match_statement(&lines, &mut current_line)?
            } else {
                return Err(CompilerError::UnknownBlockType(line.trim().to_string()));
            }
        } else {
            parse_line(line)?
        };

        if !matches!(op, Op::Nop) {
            ops.push(op);
        }
        current_line += 1;
    }

    Ok(ops)
}

fn parse_function_signature(line: &str) -> Result<(String, Vec<String>), CompilerError> {
    // Format: def name(x, y):
    let parts: Vec<&str> = line.trim_end_matches(':').splitn(2, '(').collect();
    if parts.len() != 2 {
        return Err(CompilerError::InvalidFunctionDefinition(line.to_string()));
    }

    let name = parts[0].trim_start_matches("def").trim().to_string();
    let params_str = parts[1].trim_end_matches(')');
    let params: Vec<String> = params_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok((name, params))
}

fn parse_line(line: &str) -> Result<Op, CompilerError> {
    // Skip comments
    if line.starts_with('#') {
        return Ok(Op::Nop);
    }

    let mut parts = line.split_whitespace();
    let command = match parts.next() {
        Some(cmd) => cmd,
        None => return Ok(Op::Nop),
    };

    match command {
        "push" => {
            let num_str = parts.next().ok_or(CompilerError::MissingPushValue)?;
            let num = num_str.parse::<f64>()
                .map_err(|_| CompilerError::InvalidPushValue(num_str.to_string()))?;
            Ok(Op::Push(num))
        }
        "emit" => {
            if let Some(inner) = line.find('"') {
                let inner = &line[inner + 1..line.rfind('"').unwrap_or(line.len())];
                Ok(Op::Emit(inner.to_string()))
            } else {
                Err(CompilerError::MissingEmitQuotes)
            }
        }
        "emitevent" => {
            // Format: emitevent "category" "message"
            let line_str = line.to_string();
            let parts: Vec<&str> = line_str.split('"').collect();
            if parts.len() < 5 {
                return Err(CompilerError::InvalidEmitEventFormat);
            }
            
            let category = parts[1].trim().to_string();
            let message = parts[3].trim().to_string();
            
            Ok(Op::EmitEvent { category, message })
        }
        "assertequalstack" => {
            let depth_str = parts.next().ok_or(CompilerError::MissingAssertDepth)?;
            let depth = depth_str.parse::<usize>()
                .map_err(|_| CompilerError::InvalidAssertDepth(depth_str.to_string()))?;
            
            if depth < 2 {
                return Err(CompilerError::InsufficientAssertDepth);
            }
            
            Ok(Op::AssertEqualStack { depth })
        }
        "break" => Ok(Op::Break),
        "continue" => Ok(Op::Continue),
        "load" => Ok(Op::Load(
            parts.next().ok_or(CompilerError::MissingVariable("load".to_string()))?.to_string(),
        )),
        "store" => Ok(Op::Store(
            parts.next().ok_or(CompilerError::MissingVariable("store".to_string()))?.to_string(),
        )),
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
        "call" => Ok(Op::Call(
            parts.next().ok_or(CompilerError::MissingFunctionName)?.to_string(),
        )),
        "dumpstack" => Ok(Op::DumpStack),
        "dumpmemory" => Ok(Op::DumpMemory),
        _ => Err(CompilerError::UnknownCommand(command.to_string())),
    }
}

fn get_indent(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn parse_if_statement(lines: &[String], current_line: &mut usize) -> Result<Op, CompilerError> {
    let mut condition = Vec::new();
    let mut then_block = Vec::new();
    let mut else_block = None;
    let mut current_indent = get_indent(&lines[*current_line]);

    // Skip the "if:" line
    *current_line += 1;

    // Parse the then block
    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = get_indent(line);
        
        if indent <= current_indent {
            break;
        }

        // Check for nested if
        if line.trim() == "if:" {
            let nested_if = parse_if_statement(lines, current_line)?;
            then_block.push(nested_if);
            continue; // parse_if_statement already incremented current_line
        }

        let op = parse_line(line)?;
        if !matches!(op, Op::Nop) {
            then_block.push(op);
        }
        *current_line += 1;
    }

    // Check for else block
    if *current_line < lines.len() && lines[*current_line].trim() == "else:" {
        *current_line += 1;
        let mut else_ops = Vec::new();

        while *current_line < lines.len() {
            let line = &lines[*current_line];
            let indent = get_indent(line);
            
            if indent <= current_indent {
                break;
            }

            // Check for nested if in else block
            if line.trim() == "if:" {
                let nested_if = parse_if_statement(lines, current_line)?;
                else_ops.push(nested_if);
                continue; // parse_if_statement already incremented current_line
            }

            let op = parse_line(line)?;
            if !matches!(op, Op::Nop) {
                else_ops.push(op);
            }
            *current_line += 1;
        }

        else_block = Some(else_ops);
    }

    Ok(Op::If {
        condition,
        then: then_block,
        else_: else_block,
    })
}

fn parse_function_definition(lines: &[String], current_line: &mut usize) -> Result<Op, CompilerError> {
    let line = &lines[*current_line];
    
    // Expected format: def name(param1, param2):
    if !line.contains('(') || !line.contains(')') {
        return Err(CompilerError::InvalidFunctionFormat(line.to_string()));
    }
    
    // Extract name and parameters
    let parts = line.trim().split('(').collect::<Vec<&str>>();
    if parts.len() != 2 {
        return Err(CompilerError::InvalidFunctionDefinition(line.to_string()));
    }
    
    let name_part = parts[0].trim();
    if !name_part.starts_with("def ") {
        return Err(CompilerError::InvalidFunctionStart(line.to_string()));
    }
    
    let name = name_part["def ".len()..].trim().to_string();
    
    // Extract parameters
    let params_part = parts[1].split(')').next().unwrap().trim();
    let params: Vec<String> = params_part
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    let mut body = Vec::new();
    let current_indent = get_indent(line);
    *current_line += 1;
    
    // Parse function body
    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = get_indent(line);
        
        if indent <= current_indent {
            break;
        }
        
        let op = parse_line(line)?;
        if !matches!(op, Op::Nop) {
            body.push(op);
        }
        *current_line += 1;
    }
    
    Ok(Op::Def {
        name,
        params,
        body,
    })
}

fn parse_while_statement(lines: &[String], current_line: &mut usize) -> Result<Op, CompilerError> {
    let mut condition = Vec::new();
    let mut body = Vec::new();
    let mut current_indent = get_indent(&lines[*current_line]);

    // Skip the "while:" line
    *current_line += 1;

    // Parse the body
    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = get_indent(line);
        
        if indent <= current_indent {
            break;
        }

        let op = parse_line(line)?;
        if !matches!(op, Op::Nop) {
            body.push(op);
        }
        *current_line += 1;
    }

    Ok(Op::While {
        condition,
        body,
    })
}

fn parse_match_statement(lines: &[String], current_line: &mut usize) -> Result<Op, CompilerError> {
    let mut value_ops = Vec::new();
    let mut cases = Vec::new();
    let mut default_ops = None;
    let current_indent = get_indent(&lines[*current_line]);
    
    // Skip the "match:" line
    *current_line += 1;
    
    // First line after match: should be the value block
    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = get_indent(line);
        
        if indent <= current_indent {
            break;
        }
        
        if line.trim() == "value:" {
            *current_line += 1;
            let value_indent = indent;
            
            // Parse the value block
            while *current_line < lines.len() {
                let line = &lines[*current_line];
                let current_indent = get_indent(line);
                
                if current_indent <= value_indent {
                    break;
                }
                
                let op = parse_line(line)?;
                if !matches!(op, Op::Nop) {
                    value_ops.push(op);
                }
                *current_line += 1;
            }
        } else if line.trim().starts_with("case ") {
            // Parse case value
            let case_line = line.trim();
            let case_value_str = case_line[5..].trim();
            let case_value = case_value_str.parse::<f64>()
                .map_err(|_| CompilerError::InvalidCaseValue(case_value_str.to_string()))?;
            
            let case_indent = indent;
            *current_line += 1;
            
            let mut case_ops = Vec::new();
            
            // Parse case block
            while *current_line < lines.len() {
                let line = &lines[*current_line];
                let current_indent = get_indent(line);
                
                if current_indent <= case_indent {
                    break;
                }
                
                let op = parse_line(line)?;
                if !matches!(op, Op::Nop) {
                    case_ops.push(op);
                }
                *current_line += 1;
            }
            
            cases.push((case_value, case_ops));
        } else if line.trim() == "default:" {
            *current_line += 1;
            let default_indent = indent;
            
            let mut default_block = Vec::new();
            
            // Parse default block
            while *current_line < lines.len() {
                let line = &lines[*current_line];
                let current_indent = get_indent(line);
                
                if current_indent <= default_indent {
                    break;
                }
                
                let op = parse_line(line)?;
                if !matches!(op, Op::Nop) {
                    default_block.push(op);
                }
                *current_line += 1;
            }
            
            default_ops = Some(default_block);
        } else {
            let op = parse_line(line)?;
            if !matches!(op, Op::Nop) {
                value_ops.push(op);
            }
            *current_line += 1;
        }
    }
    
    if value_ops.is_empty() {
        return Err(CompilerError::MissingMatchValue);
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
    use crate::vm::Op;

    #[test]
    fn test_simple_push_emit() {
        let source = r#"push 42
emit "hello world""#;
        let ops = parse_dsl(source).unwrap();
        assert_eq!(
            ops,
            vec![Op::Push(42.0), Op::Emit("hello world".to_string())]
        );
    }

    #[test]
    fn test_function_definition() {
        let source = r#"
def add(x, y):
    load x
    load y
    add
    return
"#;
        let ops = parse_dsl(source).unwrap();
        match &ops[0] {
            Op::Def { name, params, body } => {
                assert_eq!(name, "add");
                assert_eq!(params, &vec!["x".to_string(), "y".to_string()]);
                assert_eq!(
                    body,
                    &vec![
                        Op::Load("x".to_string()),
                        Op::Load("y".to_string()),
                        Op::Add,
                        Op::Return
                    ]
                );
            }
            _ => panic!("Expected function definition"),
        }
    }

    #[test]
    fn test_invalid_instruction() {
        let result = parse_dsl("foobar");
        assert!(result.is_err());
    }

    #[test]
    fn test_emit_without_quotes() {
        let result = parse_dsl("emit hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_if_statement() {
        let source = r#"
push 1
if:
    push 42
else:
    push 24
"#;
        let ops = parse_dsl(source).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Push(1.0),
                Op::If {
                    condition: vec![],
                    then: vec![Op::Push(42.0)],
                    else_: Some(vec![Op::Push(24.0)])
                }
            ]
        );
    }

    #[test]
    fn test_nested_if() {
        let source = r#"push 1
if:
    push 2
    if:
        push 3
    else:
        push 4
else:
    push 5"#;
        let ops = parse_dsl(source).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Push(1.0),
                Op::If {
                    condition: vec![],
                    then: vec![
                        Op::Push(2.0),
                        Op::If {
                            condition: vec![],
                            then: vec![Op::Push(3.0)],
                            else_: Some(vec![Op::Push(4.0)])
                        }
                    ],
                    else_: Some(vec![Op::Push(5.0)])
                }
            ]
        );
    }

    #[test]
    fn test_break_continue() {
        let source = r#"
push 0
store counter
loop 10:
    load counter
    push 1
    add
    store counter
    load counter
    push 5
    eq
    if:
        break
        
push 0
while:
    push 1
    if:
        continue
"#;
        let ops = parse_dsl(source).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Push(0.0),
                Op::Store("counter".to_string()),
                Op::Loop {
                    count: 10,
                    body: vec![
                        Op::Load("counter".to_string()),
                        Op::Push(1.0),
                        Op::Add,
                        Op::Store("counter".to_string()),
                        Op::Load("counter".to_string()),
                        Op::Push(5.0),
                        Op::Eq,
                        Op::If {
                            condition: vec![],
                            then: vec![Op::Break],
                            else_: None,
                        },
                    ],
                },
                Op::Push(0.0),
                Op::While {
                    condition: vec![
                        Op::Push(1.0),
                    ],
                    body: vec![
                        Op::If {
                            condition: vec![],
                            then: vec![Op::Continue],
                            else_: None,
                        },
                    ],
                },
            ]
        );
    }
    
    #[test]
    fn test_emitevent() {
        let source = r#"emitevent "governance" "proposal accepted""#;
        let ops = parse_dsl(source).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::EmitEvent {
                    category: "governance".to_string(),
                    message: "proposal accepted".to_string(),
                },
            ]
        );
    }
    
    #[test]
    fn test_assertequalstack() {
        let source = r#"
push 42
push 42
push 42
assertequalstack 3
"#;
        let ops = parse_dsl(source).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Push(42.0),
                Op::Push(42.0),
                Op::Push(42.0),
                Op::AssertEqualStack { depth: 3 },
            ]
        );
    }
    
    #[test]
    fn test_match_statement() {
        let source = r#"
push 2
match:
    value:
        # Empty - will use the value on the stack
    case 1:
        push 10
    case 2:
        push 20
    case 3:
        push 30
    default:
        push 0
"#;
        let ops = parse_dsl(source).unwrap();
        
        match &ops[1] {
            Op::Match { value, cases, default } => {
                assert!(value.is_empty()); // Empty value block will use stack
                assert_eq!(cases.len(), 3);
                assert_eq!(cases[0], (1.0, vec![Op::Push(10.0)]));
                assert_eq!(cases[1], (2.0, vec![Op::Push(20.0)]));
                assert_eq!(cases[2], (3.0, vec![Op::Push(30.0)]));
                assert_eq!(default.as_ref().unwrap(), &vec![Op::Push(0.0)]);
            }
            _ => panic!("Expected match statement"),
        }
    }
    
    #[test]
    fn test_match_with_computed_value() {
        let source = r#"
match:
    value:
        push 1
        push 2
        add
    case 3:
        push 30
"#;
        let ops = parse_dsl(source).unwrap();
        
        match &ops[0] {
            Op::Match { value, cases, default } => {
                assert_eq!(value, &vec![Op::Push(1.0), Op::Push(2.0), Op::Add]);
                assert_eq!(cases.len(), 1);
                assert_eq!(cases[0], (3.0, vec![Op::Push(30.0)]));
                assert!(default.is_none());
            }
            _ => panic!("Expected match statement"),
        }
    }
    
    #[test]
    fn test_invalid_match() {
        // Missing value block
        let source = r#"
match:
    case 1:
        push 10
"#;
        assert!(parse_dsl(source).is_err());
        
        // Invalid case value
        let source = r#"
match:
    value:
        push 1
    case invalid:
        push 10
"#;
        assert!(parse_dsl(source).is_err());
    }
} 