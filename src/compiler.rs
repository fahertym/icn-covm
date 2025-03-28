use crate::vm::Op;

pub fn parse_dsl(source: &str) -> Result<Vec<Op>, String> {
    let mut ops = Vec::new();
    let mut lines = source.lines().peekable();
    let mut current_if: Option<(Vec<Op>, Option<Vec<Op>>)> = None;

    while let Some(line) = lines.next() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("def ") {
            // Parse function definition
            let (name, params) = parse_function_signature(line)?;
            let mut body = Vec::new();

            while let Some(next_line) = lines.peek() {
                if next_line.starts_with("    ") {
                    let indented = lines.next().unwrap().trim_start();
                    let inner_ops = parse_line(indented)?;
                    body.extend(inner_ops);
                } else {
                    break;
                }
            }

            ops.push(Op::Def {
                name,
                params,
                body,
            });
        } else if line.trim() == "if:" {
            // Start a new if block
            current_if = Some((Vec::new(), None));
        } else if line.trim() == "else:" {
            // Switch to else block
            if let Some((then_block, None)) = &mut current_if {
                let then = then_block.clone();
                current_if = Some((then, Some(Vec::new())));
            } else {
                return Err("Unexpected else block".to_string());
            }
        } else if line.starts_with("    ") {
            // Handle indented block
            let indented = line.trim_start();
            let inner_ops = parse_line(indented)?;
            
            if let Some((ref mut then_block, ref mut else_block)) = current_if {
                if let Some(else_ops) = else_block {
                    else_ops.extend(inner_ops);
                } else {
                    then_block.extend(inner_ops);
                }
            } else {
                return Err("Unexpected indented block".to_string());
            }
        } else {
            // Handle regular instruction or end of if block
            if let Some((then_block, else_block)) = current_if.take() {
                ops.push(Op::If {
                    condition: vec![],
                    then: then_block,
                    else_: else_block,
                });
            }
            
            let line_ops = parse_line(line)?;
            ops.extend(line_ops);
        }
    }

    // Handle any remaining if block
    if let Some((then_block, else_block)) = current_if {
        ops.push(Op::If {
            condition: vec![],
            then: then_block,
            else_: else_block,
        });
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

fn parse_line(line: &str) -> Result<Vec<Op>, String> {
    // Skip comments
    if line.starts_with('#') {
        return Ok(vec![]);
    }

    let mut parts = line.split_whitespace();
    let instr = parts.next().ok_or("Empty line")?;

    match instr {
        "push" => {
            let num = parts
                .next()
                .ok_or("Missing number for push")?
                .parse::<f64>()
                .map_err(|e| format!("Invalid number: {}", e))?;
            Ok(vec![Op::Push(num)])
        }
        "emit" => {
            let rest = line.trim_start_matches("emit").trim();
            if rest.starts_with('"') && rest.ends_with('"') {
                let inner = &rest[1..rest.len() - 1];
                Ok(vec![Op::Emit(inner.to_string())])
            } else {
                Err("emit expects a quoted string".to_string())
            }
        }
        "if:" => {
            // The condition has already been pushed to the stack
            Ok(vec![Op::If {
                condition: vec![],
                then: vec![],
                else_: None,
            }])
        }
        "else:" => {
            // The else block will be handled by the main parser
            Ok(vec![])
        }
        "load" => Ok(vec![Op::Load(
            parts.next().ok_or("Missing variable for load")?.to_string(),
        )]),
        "store" => Ok(vec![Op::Store(
            parts.next().ok_or("Missing variable for store")?.to_string(),
        )]),
        "add" => Ok(vec![Op::Add]),
        "sub" => Ok(vec![Op::Sub]),
        "mul" => Ok(vec![Op::Mul]),
        "div" => Ok(vec![Op::Div]),
        "mod" => Ok(vec![Op::Mod]),
        "eq" => Ok(vec![Op::Eq]),
        "gt" => Ok(vec![Op::Gt]),
        "lt" => Ok(vec![Op::Lt]),
        "not" => Ok(vec![Op::Not]),
        "and" => Ok(vec![Op::And]),
        "or" => Ok(vec![Op::Or]),
        "negate" => Ok(vec![Op::Negate]),
        "dup" => Ok(vec![Op::Dup]),
        "swap" => Ok(vec![Op::Swap]),
        "over" => Ok(vec![Op::Over]),
        "pop" => Ok(vec![Op::Pop]),
        "return" => Ok(vec![Op::Return]),
        "call" => Ok(vec![Op::Call(
            parts.next().ok_or("Missing function name for call")?.to_string(),
        )]),
        "dumpstack" => Ok(vec![Op::DumpStack]),
        "dumpmemory" => Ok(vec![Op::DumpMemory]),
        _ => Err(format!("Unknown instruction: {}", instr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::Op;

    #[test]
    fn test_simple_push_emit() {
        let source = r#"
            push 42
            emit "hello world"
        "#;
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

            push 1
            push 2
            call add
        "#;
        let ops = parse_dsl(source).unwrap();
        match &ops[0] {
            Op::Def { name, params, body } => {
                assert_eq!(name, "add");
                assert_eq!(params, &vec!["x", "y"]);
                assert_eq!(
                    body,
                    &vec![Op::Load("x".into()), Op::Load("y".into()), Op::Add, Op::Return]
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
} 