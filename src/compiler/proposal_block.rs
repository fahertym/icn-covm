use super::{common, CompilerError, SourcePosition};

/// Parse a proposal block with if passed and else blocks
pub fn parse_proposal_block(
    lines: &[String],
    current_line: &mut usize,
    pos: SourcePosition,
) -> Result<(Vec<String>, Vec<String>, Option<Vec<String>>), CompilerError> {
    let mut execution_block = Vec::new();
    let mut passed_block = Vec::new();
    let mut failed_block = None;
    let current_indent = common::get_indent(&lines[*current_line]);
    let mut found_if_passed = false;
    let mut found_else = false;

    // Skip the opening brace line
    *current_line += 1;

    while *current_line < lines.len() {
        let line = &lines[*current_line];
        let indent = common::get_indent(line);

        // If we've hit a non-empty line that is dedented, we're done with this block
        if !line.trim().is_empty() && indent <= current_indent {
            break;
        } else if line.trim().is_empty() {
            *current_line += 1; // Skip empty lines
            continue;
        }

        let current_pos = SourcePosition::new(pos.line + *current_line, indent + 1);

        // Check for if passed block
        if line.trim() == "if passed:" {
            if found_if_passed {
                return Err(CompilerError::DuplicateIfPassedBlock(
                    current_pos.line,
                    current_pos.column,
                ));
            }
            found_if_passed = true;
            *current_line += 1;

            // Parse the if passed block
            let mut if_passed_lines = Vec::new();
            while *current_line < lines.len() {
                let if_line = &lines[*current_line];
                let if_indent = common::get_indent(if_line);

                if !if_line.trim().is_empty() && if_indent <= current_indent {
                    break;
                } else if if_line.trim().is_empty() {
                    *current_line += 1;
                    continue;
                }

                if_passed_lines.push(if_line.clone());
                *current_line += 1;
            }

            passed_block = if_passed_lines;
            continue;
        }

        // Check for else block
        if line.trim() == "else:" {
            if found_else {
                return Err(CompilerError::DuplicateElseBlock(
                    current_pos.line,
                    current_pos.column,
                ));
            }
            if !found_if_passed {
                return Err(CompilerError::ElseWithoutIfPassed(
                    current_pos.line,
                    current_pos.column,
                ));
            }
            found_else = true;
            *current_line += 1;

            // Parse the else block
            let mut else_lines = Vec::new();
            while *current_line < lines.len() {
                let else_line = &lines[*current_line];
                let else_indent = common::get_indent(else_line);

                if !else_line.trim().is_empty() && else_indent <= current_indent {
                    break;
                } else if else_line.trim().is_empty() {
                    *current_line += 1;
                    continue;
                }

                else_lines.push(else_line.clone());
                *current_line += 1;
            }

            failed_block = Some(else_lines);
            continue;
        }

        // Regular line in the execution block
        execution_block.push(line.clone());
        *current_line += 1;
    }

    Ok((execution_block, passed_block, failed_block))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_block_parsing() {
        let source = vec![
            "{".to_string(),
            "    emit \"Executing proposal\"".to_string(),
            "    if passed:".to_string(),
            "        emit \"Proposal passed\"".to_string(),
            "        transfer \"treasury\" \"fund\" 1000.0".to_string(),
            "    else:".to_string(),
            "        emit \"Proposal failed\"".to_string(),
            "}".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let (execution_block, passed_block, failed_block) =
            parse_proposal_block(&source, &mut current_line, pos).unwrap();

        assert_eq!(execution_block.len(), 1);
        assert_eq!(passed_block.len(), 2);
        assert!(failed_block.is_some());
        assert_eq!(failed_block.unwrap().len(), 1);
    }

    #[test]
    fn test_proposal_block_without_conditionals() {
        let source = vec![
            "{".to_string(),
            "    emit \"Executing proposal\"".to_string(),
            "    transfer \"treasury\" \"fund\" 1000.0".to_string(),
            "}".to_string(),
        ];

        let mut current_line = 0;
        let pos = SourcePosition::new(1, 1);

        let (execution_block, passed_block, failed_block) =
            parse_proposal_block(&source, &mut current_line, pos).unwrap();

        assert_eq!(execution_block.len(), 2);
        assert_eq!(passed_block.len(), 0);
        assert!(failed_block.is_none());
    }
}
