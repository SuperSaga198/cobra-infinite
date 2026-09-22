//! Human-readable diagnostics shared by Cobra tools.

use cobra_lexer_quantum::LexError;
use cobra_parser_ast::ParseError;

pub fn render_lex_errors(errors: &[LexError]) -> String {
    errors
        .iter()
        .map(|error| format_error(error.span.line, error.span.column, &error.message))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_parse_errors(errors: &[ParseError]) -> String {
    errors
        .iter()
        .map(|error| format_error(error.span.line, error.span.column, &error.message))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_error(line: usize, column: usize, message: &str) -> String {
    format!("error[{line}:{column}]: {message}")
}
