#![allow(dead_code)] // Allow dead code during development

use crate::vm::Op;
use thiserror::Error;

// Sub-modules
pub mod common;
pub mod function_block;
pub mod if_block;
pub mod line_parser;
pub mod loop_block;
pub mod match_block;
pub mod while_block;

// Re-export the parser functions
pub use function_block::parse_function_block;
pub use if_block::parse_if_block;
pub use line_parser::parse_line;
pub use loop_block::parse_loop_block;
pub use match_block::parse_match_block;
pub use while_block::parse_while_block;

/// Standard library support
pub mod stdlib;

/// Parse DSL source with standard library functions included
///
/// This function parses the provided DSL source code after first
/// prepending the standard library code. This ensures that standard
/// functions are available to the program.
///
/// # Arguments
///
/// * `source` - The DSL source code to parse
///
/// # Returns
///
/// * `Result<Vec<Op>, CompilerError>` - The parsed operations or an error
///
/// # Example
///
/// ```
/// use icn_covm::compiler::parse_dsl_with_stdlib;
///
/// let source = "
///     call abs  # Call standard library function
/// ";
///
/// let ops = parse_dsl_with_stdlib(source).unwrap();
/// ```
pub fn parse_dsl_with_stdlib(source: &str) -> Result<Vec<Op>, CompilerError> {
    // First load the standard library code
    let stdlib_code = stdlib::get_stdlib_code();

    // Concatenate the standard library code with the user code
    let combined_code = format!("{}\n\n{}", stdlib_code, source);

    // Parse the combined code
    parse_dsl(&combined_code)
}

/// Errors that can occur during compilation of DSL code
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompilerError {
    /// Unknown command encountered during parsing
    #[error("Unknown command: {0} at line {1}, column {2}")]
    UnknownCommand(String, usize, usize),

    /// Unknown block type encountered during parsing
    #[error("Unknown block type: {0} at line {1}, column {2}")]
    UnknownBlockType(String, usize, usize),

    /// Invalid function definition syntax
    #[error("Invalid function definition: {0} at line {1}, column {2}")]
    InvalidFunctionDefinition(String, usize, usize),

    /// Invalid function definition format
    #[error("Invalid function definition format: {0} at line {1}, column {2}")]
    InvalidFunctionFormat(String, usize, usize),

    /// Function definition must start with 'def'
    #[error("Function definition must start with 'def': {0} at line {1}, column {2}")]
    InvalidFunctionStart(String, usize, usize),

    /// Missing a number for a push operation
    #[error("Missing number for push at line {0}, column {1}")]
    MissingPushValue(usize, usize),

    /// Invalid number format for a push operation
    #[error("Invalid number for push: {0} at line {1}, column {2}")]
    InvalidPushValue(String, usize, usize),

    /// Missing quotes for an emit command
    #[error("Missing quotes for emit command at line {0}, column {1}")]
    MissingEmitQuotes(usize, usize),

    /// Invalid format for an emitevent command
    #[error("Invalid format for emitevent at line {0}, column {1}, expected: emitevent \"category\" \"message\"")]
    InvalidEmitEventFormat(usize, usize),

    /// Missing variable name for a command
    #[error("Missing variable for {0} at line {1}, column {2}")]
    MissingVariable(String, usize, usize),

    /// Missing function name for a call operation
    #[error("Missing function name for call at line {0}, column {1}")]
    MissingFunctionName(usize, usize),

    /// Missing depth for assertequalstack operation
    #[error("Missing depth for assertequalstack at line {0}, column {1}")]
    MissingAssertDepth(usize, usize),

    /// Invalid depth format for assertequalstack operation
    #[error("Invalid depth for assertequalstack: {0} at line {1}, column {2}")]
    InvalidAssertDepth(String, usize, usize),

    /// Insufficient depth for assertequalstack operation
    #[error("Depth for assertequalstack must be at least 2 at line {0}, column {1}")]
    InsufficientAssertDepth(usize, usize),

    /// Invalid case value in a match statement
    #[error("Invalid case value: {0} at line {1}, column {2}")]
    InvalidCaseValue(String, usize, usize),

    /// Missing value block in a match statement
    #[error("Match statement must have a value block at line {0}, column {1}")]
    MissingMatchValue(usize, usize),

    /// Invalid loop format
    #[error("Invalid loop format: {0} at line {1}, column {2}")]
    InvalidLoopFormat(String, usize, usize),

    /// Invalid loop count
    #[error("Invalid loop count: {0} at line {1}, column {2}")]
    InvalidLoopCount(String, usize, usize),

    /// Unexpected end of file while parsing a block
    #[error("Unexpected end of file while parsing block at line {0}")]
    UnexpectedEOF(usize),

    /// Invalid indentation level
    #[error("Invalid indentation level at line {0}")]
    InvalidIndentation(usize),

    /// Syntax error in parsing
    #[error("Syntax error: {details}")]
    SyntaxError { details: String },
}

/// Source position information for error reporting
///
/// Contains line and column information to pinpoint errors in the source code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePosition {
    /// Line number (1-indexed)
    pub line: usize,

    /// Column number (1-indexed)
    pub column: usize,
}

impl SourcePosition {
    /// Create a new source position
    ///
    /// # Arguments
    ///
    /// * `line` - Line number (1-indexed)
    /// * `column` - Column number (1-indexed)
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Parse DSL source into a vector of operations
///
/// This function parses the provided DSL source code into a vector of
/// executable operations. The DSL is a simple stack-based language with
/// support for basic operations like arithmetic, memory access, and
/// control flow.
///
/// # Arguments
///
/// * `source` - The DSL source code to parse
///
/// # Returns
///
/// * `Result<Vec<Op>, CompilerError>` - The parsed operations or an error
///
/// # Example
///
/// ```
/// use icn_covm::compiler::parse_dsl;
///
/// let source = "
///     push 10
///     push 20
///     add
/// ";
///
/// let ops = parse_dsl(source).unwrap();
/// ```
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

        let pos = SourcePosition::new(current_line + 1, common::get_indent(line) + 1);

        let op = if line.trim().ends_with(':') {
            if line.trim() == "if:" {
                parse_if_block(&lines, &mut current_line, pos)?
            } else if line.trim() == "while:" {
                parse_while_block(&lines, &mut current_line, pos)?
            } else if line.trim().starts_with("def ") {
                parse_function_block(&lines, &mut current_line, pos)?
            } else if line.trim() == "match:" {
                parse_match_block(&lines, &mut current_line, pos)?
            } else if line.trim().starts_with("loop ") {
                parse_loop_block(&lines, &mut current_line, pos)?
            } else {
                return Err(CompilerError::UnknownBlockType(
                    line.trim().to_string(),
                    pos.line,
                    pos.column,
                ));
            }
        } else {
            parse_line(line, pos)?
        };

        if !matches!(op, Op::Nop) {
            ops.push(op);
        }
        current_line += 1;
    }

    Ok(ops)
}

#[cfg(test)]
mod tests;
