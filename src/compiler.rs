use crate::vm::Op;

pub fn parse_dsl(source: &str) -> Result<Vec<Op>, String> {
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
            } else {
                return Err(format!("Unknown block type: {}", line.trim()));
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

fn parse_function_signature(line: &str) -> Result<(String, Vec<String>), String> {
    // Format: def name(x, y):
    let parts: Vec<&str> = line.trim_end_matches(':').splitn(2, '(').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid function definition: {}", line));
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

fn parse_line(line: &str) -> Result<Op, String> {
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
            let num = parts.next()
                .ok_or("Missing number for push")?
                .parse::<f64>()
                .map_err(|_| "Invalid number for push")?;
            Ok(Op::Push(num))
        }
        "emit" => {
            if let Some(inner) = line.find('"') {
                let inner = &line[inner + 1..line.rfind('"').unwrap_or(line.len())];
                Ok(Op::Emit(inner.to_string()))
            } else {
                Err("Missing quotes for emit command".to_string())
            }
        }
        "load" => Ok(Op::Load(
            parts.next().ok_or("Missing variable for load")?.to_string(),
        )),
        "store" => Ok(Op::Store(
            parts.next().ok_or("Missing variable for store")?.to_string(),
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
            parts.next().ok_or("Missing function name for call")?.to_string(),
        )),
        "dumpstack" => Ok(Op::DumpStack),
        "dumpmemory" => Ok(Op::DumpMemory),
        _ => Err(format!("Unknown command: {}", command)),
    }
}

fn get_indent(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn parse_if_statement(lines: &[String], current_line: &mut usize) -> Result<Op, String> {
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

fn parse_function_definition(lines: &[String], current_line: &mut usize) -> Result<Op, String> {
    let line = &lines[*current_line];
    
    // Expected format: def name(param1, param2):
    if !line.contains('(') || !line.contains(')') {
        return Err(format!("Invalid function definition format: {}", line));
    }
    
    // Extract name and parameters
    let parts = line.trim().split('(').collect::<Vec<&str>>();
    if parts.len() != 2 {
        return Err(format!("Invalid function definition: {}", line));
    }
    
    let name_part = parts[0].trim();
    if !name_part.starts_with("def ") {
        return Err(format!("Function definition must start with 'def': {}", line));
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

fn parse_while_statement(lines: &[String], current_line: &mut usize) -> Result<Op, String> {
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
} 