//! Recursive-descent parser and the first stable Cobra AST.

use cobra_lexer_quantum::{Span, Token, TokenKind};

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        initializer: Expr,
        span: Span,
    },
    Expression {
        expression: Expr,
        span: Span,
    },
    Block {
        statements: Vec<Stmt>,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Variable(String),
    Unary {
        operator: UnaryOp,
        right: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, Vec<ParseError>> {
    Parser::new(tokens).parse()
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
        }
    }

    fn parse(mut self) -> Result<Program, Vec<ParseError>> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            match self.statement() {
                Ok(statement) => statements.push(statement),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }
        }
        if self.errors.is_empty() {
            Ok(Program { statements })
        } else {
            Err(self.errors)
        }
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.matches_variant(TokenKind::Let) {
            return self.let_statement();
        }
        if self.matches_variant(TokenKind::Fn) {
            return self.function_statement();
        }
        if self.matches_variant(TokenKind::If) {
            return self.if_statement();
        }
        if self.matches_variant(TokenKind::While) {
            return self.while_statement();
        }
        if self.matches_variant(TokenKind::Return) {
            return self.return_statement();
        }
        if self.matches_variant(TokenKind::LeftBrace) {
            let start = self.previous().span;
            let statements = self.block()?;
            return Ok(Stmt::Block {
                statements,
                span: start,
            });
        }

        let expression = self.expression()?;
        let span = self.current_span();
        self.consume_variant(TokenKind::Semicolon, "expected `;` after expression")?;
        Ok(Stmt::Expression { expression, span })
    }

    fn let_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span;
        let name = self.identifier("expected a variable name after `let`")?;
        self.consume_variant(TokenKind::Equal, "expected `=` after variable name")?;
        let initializer = self.expression()?;
        self.consume_variant(
            TokenKind::Semicolon,
            "expected `;` after variable declaration",
        )?;
        Ok(Stmt::Let {
            name,
            initializer,
            span: start,
        })
    }

    fn function_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span;
        let name = self.identifier("expected a function name after `fn`")?;
        self.consume_variant(TokenKind::LeftParen, "expected `(` after function name")?;
        let mut params = Vec::new();
        if !self.check_variant(TokenKind::RightParen) {
            loop {
                params.push(self.identifier("expected a parameter name")?);
                if !self.matches_variant(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume_variant(TokenKind::RightParen, "expected `)` after parameters")?;
        self.consume_variant(TokenKind::LeftBrace, "expected `{` before function body")?;
        let body = self.block()?;
        Ok(Stmt::Function {
            name,
            params,
            body,
            span: start,
        })
    }

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span;
        let condition = self.expression()?;
        self.consume_variant(TokenKind::LeftBrace, "expected `{` after if condition")?;
        let then_branch = self.block()?;
        let else_branch = if self.matches_variant(TokenKind::Else) {
            self.consume_variant(TokenKind::LeftBrace, "expected `{` after `else`")?;
            Some(self.block()?)
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span: start,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span;
        let condition = self.expression()?;
        self.consume_variant(TokenKind::LeftBrace, "expected `{` after while condition")?;
        let body = self.block()?;
        Ok(Stmt::While {
            condition,
            body,
            span: start,
        })
    }

    fn return_statement(&mut self) -> Result<Stmt, ParseError> {
        let span = self.previous().span;
        let value = if self.check_variant(TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.consume_variant(TokenKind::Semicolon, "expected `;` after return value")?;
        Ok(Stmt::Return { value, span })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        while !self.check_variant(TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.statement()?);
        }
        self.consume_variant(TokenKind::RightBrace, "expected `}` after block")?;
        Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expression = self.comparison()?;
        loop {
            let operator = if self.matches_variant(TokenKind::EqualEqual) {
                Some(BinaryOp::Equal)
            } else if self.matches_variant(TokenKind::BangEqual) {
                Some(BinaryOp::NotEqual)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.comparison()?;
            expression = Expr::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expression = self.term()?;
        loop {
            let operator = if self.matches_variant(TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.matches_variant(TokenKind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.matches_variant(TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.matches_variant(TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.term()?;
            expression = Expr::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expression = self.factor()?;
        loop {
            let operator = if self.matches_variant(TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.matches_variant(TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.factor()?;
            expression = Expr::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expression = self.unary()?;
        loop {
            let operator = if self.matches_variant(TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.matches_variant(TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else if self.matches_variant(TokenKind::Percent) {
                Some(BinaryOp::Remainder)
            } else {
                None
            };
            let Some(operator) = operator else { break };
            let right = self.unary()?;
            expression = Expr::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.matches_variant(TokenKind::Bang) {
            return Ok(Expr::Unary {
                operator: UnaryOp::Not,
                right: Box::new(self.unary()?),
            });
        }
        if self.matches_variant(TokenKind::Minus) {
            return Ok(Expr::Unary {
                operator: UnaryOp::Negate,
                right: Box::new(self.unary()?),
            });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expression = self.primary()?;
        loop {
            if !self.matches_variant(TokenKind::LeftParen) {
                break;
            }
            let mut arguments = Vec::new();
            if !self.check_variant(TokenKind::RightParen) {
                loop {
                    arguments.push(self.expression()?);
                    if !self.matches_variant(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume_variant(TokenKind::RightParen, "expected `)` after arguments")?;
            expression = Expr::Call {
                callee: Box::new(expression),
                arguments,
            };
        }
        Ok(expression)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(Expr::Literal(Literal::Number(value))),
            TokenKind::String(value) => Ok(Expr::Literal(Literal::String(value))),
            TokenKind::True => Ok(Expr::Literal(Literal::Bool(true))),
            TokenKind::False => Ok(Expr::Literal(Literal::Bool(false))),
            TokenKind::Identifier(name) => Ok(Expr::Variable(name)),
            TokenKind::LeftParen => {
                let expression = self.expression()?;
                self.consume_variant(TokenKind::RightParen, "expected `)` after expression")?;
                Ok(expression)
            }
            _ => Err(ParseError {
                message: "expected an expression".into(),
                span: token.span,
            }),
        }
    }

    fn identifier(&mut self, message: &str) -> Result<String, ParseError> {
        let token = self.advance().clone();
        if let TokenKind::Identifier(name) = token.kind {
            Ok(name)
        } else {
            Err(ParseError {
                message: message.into(),
                span: token.span,
            })
        }
    }

    fn consume_variant(&mut self, expected: TokenKind, message: &str) -> Result<(), ParseError> {
        if self.matches_variant(expected) {
            Ok(())
        } else {
            Err(ParseError {
                message: message.into(),
                span: self.current_span(),
            })
        }
    }

    fn matches_variant(&mut self, expected: TokenKind) -> bool {
        if self.check_variant(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_variant(&self, expected: TokenKind) -> bool {
        same_variant(&self.peek().kind, &expected)
    }

    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }
            if matches!(
                self.peek().kind,
                TokenKind::Let
                    | TokenKind::Fn
                    | TokenKind::If
                    | TokenKind::While
                    | TokenKind::Return
            ) {
                return;
            }
            self.advance();
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn current_span(&self) -> Span {
        self.peek().span
    }

    fn is_at_end(&self) -> bool {
        same_variant(&self.peek().kind, &TokenKind::Eof)
    }
}

fn same_variant(left: &TokenKind, right: &TokenKind) -> bool {
    std::mem::discriminant(left) == std::mem::discriminant(right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cobra_lexer_quantum::tokenize;

    #[test]
    fn parses_functions_and_control_flow() {
        let source = r#"
            fn twice(value) { return value * 2; }
            let answer = twice(21);
            if answer == 42 { print("meaning"); }
        "#;
        let program = parse(tokenize(source).unwrap()).unwrap();
        assert_eq!(program.statements.len(), 3);
    }
}
