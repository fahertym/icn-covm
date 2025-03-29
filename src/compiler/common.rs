use super::SourcePosition;

/// Get the indentation level of a line (number of leading spaces)
pub fn get_indent(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

/// Check if a block is properly indented
pub fn is_indented_block(line: &str, base_indent: usize) -> bool {
    get_indent(line) > base_indent
}

/// Helper to extract the text content between double quotes
pub fn extract_quoted_text(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.split('"').collect();
    if parts.len() >= 3 {
        Some((parts[1].to_string(), parts[parts.len() - 2].to_string()))
    } else if parts.len() >= 2 {
        Some((parts[1].to_string(), String::new()))
    } else {
        None
    }
}

/// Create a source position with column adjusted for specific part of line
pub fn adjusted_position(pos: SourcePosition, line: &str, part: &str) -> SourcePosition {
    if let Some(idx) = line.find(part) {
        SourcePosition::new(pos.line, pos.column + idx)
    } else {
        pos
    }
}

/// Helper to find nested blocks within indented code
pub fn find_block_end(lines: &[String], start_line: usize, base_indent: usize) -> usize {
    let mut end_line = start_line;

    while end_line < lines.len() && get_indent(&lines[end_line]) > base_indent {
        end_line += 1;
    }

    end_line
}

/// Helper to collect lines in a block with consistent indentation
pub fn collect_block_lines(lines: &[String], start_line: usize, base_indent: usize) -> Vec<String> {
    let end_line = find_block_end(lines, start_line, base_indent);
    lines[start_line..end_line].to_vec()
}
