//! The Cobra lexer. The "quantum" name is a forward-looking module boundary;
//! this first implementation is deliberately deterministic and portable.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(f64),
    String(String),
    Let,
    Fn,
    If,
    Else,
    While,
    True,
    False,
    Return,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<LexError>> {
    Lexer::new(source).tokenize()
}

struct Lexer {
    chars: Vec<char>,
    current: usize,
    line: usize,
    column: usize,
    errors: Vec<LexError>,
    tokens: Vec<Token>,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            current: 0,
            line: 1,
            column: 1,
            errors: Vec::new(),
            tokens: Vec::new(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, Vec<LexError>> {
        while !self.is_at_end() {
            self.scan_token();
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            span: Span {
                start: self.current,
                end: self.current,
                line: self.line,
                column: self.column,
            },
        });

        if self.errors.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.errors)
        }
    }

    fn scan_token(&mut self) {
        let start = self.current;
        let line = self.line;
        let column = self.column;
        let character = self.advance();

        match character {
            Some('(') => self.add_simple(TokenKind::LeftParen, start, line, column),
            Some(')') => self.add_simple(TokenKind::RightParen, start, line, column),
            Some('{') => self.add_simple(TokenKind::LeftBrace, start, line, column),
            Some('}') => self.add_simple(TokenKind::RightBrace, start, line, column),
            Some(',') => self.add_simple(TokenKind::Comma, start, line, column),
            Some(';') => self.add_simple(TokenKind::Semicolon, start, line, column),
            Some('+') => self.add_simple(TokenKind::Plus, start, line, column),
            Some('-') => self.add_simple(TokenKind::Minus, start, line, column),
            Some('*') => self.add_simple(TokenKind::Star, start, line, column),
            Some('%') => self.add_simple(TokenKind::Percent, start, line, column),
            Some('!') => {
                let kind = if self.take_if('=') {
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                };
                self.add_simple(kind, start, line, column);
            }
            Some('=') => {
                let kind = if self.take_if('=') {
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                };
                self.add_simple(kind, start, line, column);
            }
            Some('<') => {
                let kind = if self.take_if('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                };
                self.add_simple(kind, start, line, column);
            }
            Some('>') => {
                let kind = if self.take_if('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                };
                self.add_simple(kind, start, line, column);
            }
            Some('/') if self.peek() == Some('/') => {
                while self.peek().is_some() && self.peek() != Some('\n') {
                    self.advance();
                }
            }
            Some('/') => self.add_simple(TokenKind::Slash, start, line, column),
            Some('"') => self.scan_string(start, line, column),
            Some(character) if character.is_ascii_digit() => {
                self.scan_number(start, line, column);
            }
            Some(character) if is_identifier_start(character) => {
                self.scan_identifier(start, line, column);
            }
            Some(character) if character.is_whitespace() => {}
            Some(character) => self.errors.push(LexError {
                message: format!("unexpected character `{character}`"),
                span: self.span(start, line, column),
            }),
            None => {}
        }
    }

    fn scan_string(&mut self, start: usize, line: usize, column: usize) {
        let mut value = String::new();
        let mut terminated = false;

        while let Some(character) = self.peek() {
            if character == '"' {
                self.advance();
                terminated = true;
                break;
            }
            if character == '\n' {
                self.errors.push(LexError {
                    message: "unterminated string literal".into(),
                    span: self.span(start, line, column),
                });
                return;
            }
            if character == '\\' {
                self.advance();
                match self.advance() {
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(escaped) => {
                        self.errors.push(LexError {
                            message: format!("unknown escape `\\{escaped}`"),
                            span: self.span(start, line, column),
                        });
                    }
                    None => break,
                }
            } else if let Some(character) = self.advance() {
                value.push(character);
            }
        }

        if !terminated {
            self.errors.push(LexError {
                message: "unterminated string literal".into(),
                span: self.span(start, line, column),
            });
        } else {
            self.add_simple(TokenKind::String(value), start, line, column);
        }
    }

    fn scan_number(&mut self, start: usize, line: usize, column: usize) {
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
        }
        if self.peek() == Some('.')
            && self
                .peek_next()
                .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }

        let lexeme = self.lexeme(start);
        match lexeme.parse::<f64>() {
            Ok(number) => self.add_simple(TokenKind::Number(number), start, line, column),
            Err(_) => self.errors.push(LexError {
                message: format!("invalid number `{lexeme}`"),
                span: self.span(start, line, column),
            }),
        }
    }

    fn scan_identifier(&mut self, start: usize, line: usize, column: usize) {
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }
        let lexeme = self.lexeme(start);
        let kind = match lexeme.as_str() {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "return" => TokenKind::Return,
            _ => TokenKind::Identifier(lexeme),
        };
        self.add_simple(kind, start, line, column);
    }

    fn add_simple(&mut self, kind: TokenKind, start: usize, line: usize, column: usize) {
        self.tokens.push(Token {
            kind,
            lexeme: self.lexeme(start),
            span: self.span(start, line, column),
        });
    }

    fn span(&self, start: usize, line: usize, column: usize) -> Span {
        Span {
            start,
            end: self.current,
            line,
            column,
        }
    }

    fn lexeme(&self, start: usize) -> String {
        self.chars[start..self.current].iter().collect()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.chars.get(self.current).copied();
        if let Some(character) = character {
            self.current += 1;
            if character == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        character
    }

    fn take_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.current).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.current + 1).copied()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.chars.len()
    }
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_a_small_program() {
        let tokens = tokenize(r#"let answer = 40 + 2; print("ok");"#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[3].kind, TokenKind::Number(value) if value == 40.0));
        assert!(matches!(tokens[9].kind, TokenKind::String(ref value) if value == "ok"));
    }
}
