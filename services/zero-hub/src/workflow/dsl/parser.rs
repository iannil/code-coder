//! Expression parser for workflow DSL.
//!
//! Provides AST-based parsing for more complex expression handling.

use thiserror::Error;

/// Parse error types.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("Unexpected end of expression")]
    UnexpectedEnd,
    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),
}

/// Expression AST node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Null literal
    Null,
    /// Boolean literal
    Bool(bool),
    /// Number literal
    Number(f64),
    /// String literal
    String(String),
    /// Variable reference ($event.type)
    Variable(Vec<String>),
    /// Binary operation
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },
    /// Function call
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
    /// Array literal
    Array(Vec<Expr>),
    /// Object literal
    Object(Vec<(String, Expr)>),
    /// Conditional (ternary) expression
    Conditional {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // String
    Contains,
    StartsWith,
    EndsWith,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Neg,
}

/// Token types for lexing.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    // Variable
    Variable(String),
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    // Keywords
    Contains,
    StartsWith,
    EndsWith,
    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Question,
    Dot,
    // End
    Eof,
}

/// Tokenize an expression string.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Skip whitespace
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // String literals
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            tokens.push(Token::String(s));
            i += 1;
            continue;
        }

        // Numbers
        if c.is_ascii_digit() || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let start = i;
            if c == '-' {
                i += 1;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            let num: f64 = num_str
                .parse()
                .map_err(|_| ParseError::InvalidSyntax(format!("Invalid number: {}", num_str)))?;
            tokens.push(Token::Number(num));
            continue;
        }

        // Variables ($event.type)
        if c == '$' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
            let var: String = chars[start..i].iter().collect();
            tokens.push(Token::Variable(var));
            continue;
        }

        // Identifiers and keywords
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let token = match ident.as_str() {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                "null" => Token::Null,
                "contains" => Token::Contains,
                "startsWith" => Token::StartsWith,
                "endsWith" => Token::EndsWith,
                _ => Token::Variable(ident),
            };
            tokens.push(token);
            continue;
        }

        // Multi-character operators
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            let token = match two.as_str() {
                "==" => Some(Token::Eq),
                "!=" => Some(Token::Ne),
                "<=" => Some(Token::Le),
                ">=" => Some(Token::Ge),
                "&&" => Some(Token::And),
                "||" => Some(Token::Or),
                _ => None,
            };
            if let Some(t) = token {
                tokens.push(t);
                i += 2;
                continue;
            }
        }

        // Single-character tokens
        let token = match c {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '!' => Token::Not,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ',' => Token::Comma,
            ':' => Token::Colon,
            '?' => Token::Question,
            '.' => Token::Dot,
            _ => return Err(ParseError::UnexpectedToken(c.to_string())),
        };
        tokens.push(token);
        i += 1;
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

/// Parse a tokenized expression into an AST.
pub fn parse_expression(input: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;

        while matches!(self.current(), Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;

        while matches!(self.current(), Token::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_string_ops()?;

        loop {
            let op = match self.current() {
                Token::Eq => BinaryOperator::Eq,
                Token::Ne => BinaryOperator::Ne,
                Token::Lt => BinaryOperator::Lt,
                Token::Le => BinaryOperator::Le,
                Token::Gt => BinaryOperator::Gt,
                Token::Ge => BinaryOperator::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_string_ops()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_string_ops(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current() {
                Token::Contains => BinaryOperator::Contains,
                Token::StartsWith => BinaryOperator::StartsWith,
                Token::EndsWith => BinaryOperator::EndsWith,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOperator::Mul,
                Token::Slash => BinaryOperator::Div,
                Token::Percent => BinaryOperator::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.current() {
            Token::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOperator::Not,
                    expr: Box::new(expr),
                })
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOperator::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let expr = match self.current().clone() {
            Token::Null => {
                self.advance();
                Expr::Null
            }
            Token::Bool(b) => {
                self.advance();
                Expr::Bool(b)
            }
            Token::Number(n) => {
                self.advance();
                Expr::Number(n)
            }
            Token::String(s) => {
                self.advance();
                Expr::String(s)
            }
            Token::Variable(v) => {
                self.advance();
                let parts: Vec<String> = v.split('.').map(|s| s.to_string()).collect();
                Expr::Variable(parts)
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                if !matches!(self.current(), Token::RParen) {
                    return Err(ParseError::InvalidSyntax("Expected ')'".into()));
                }
                self.advance();
                expr
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !matches!(self.current(), Token::RBracket | Token::Eof) {
                    elements.push(self.parse_expr()?);
                    if matches!(self.current(), Token::Comma) {
                        self.advance();
                    }
                }
                if !matches!(self.current(), Token::RBracket) {
                    return Err(ParseError::InvalidSyntax("Expected ']'".into()));
                }
                self.advance();
                Expr::Array(elements)
            }
            Token::Eof => return Err(ParseError::UnexpectedEnd),
            other => return Err(ParseError::UnexpectedToken(format!("{:?}", other))),
        };

        Ok(expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("$event.type == \"push\"").unwrap();
        assert!(matches!(tokens[0], Token::Variable(_)));
        assert!(matches!(tokens[1], Token::Eq));
        assert!(matches!(tokens[2], Token::String(_)));
    }

    #[test]
    fn test_tokenize_numbers() {
        let tokens = tokenize("42 + 3.14").unwrap();
        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Plus);
        assert_eq!(tokens[2], Token::Number(3.14));
    }

    #[test]
    fn test_parse_comparison() {
        let expr = parse_expression("$event.count > 10").unwrap();
        if let Expr::BinaryOp { op, .. } = expr {
            assert_eq!(op, BinaryOperator::Gt);
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_parse_logical() {
        let expr = parse_expression("true && false").unwrap();
        if let Expr::BinaryOp { op, .. } = expr {
            assert_eq!(op, BinaryOperator::And);
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_parse_string_op() {
        let expr = parse_expression("$event.branch contains \"main\"").unwrap();
        if let Expr::BinaryOp { op, .. } = expr {
            assert_eq!(op, BinaryOperator::Contains);
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_parse_array() {
        let expr = parse_expression("[1, 2, 3]").unwrap();
        if let Expr::Array(elements) = expr {
            assert_eq!(elements.len(), 3);
        } else {
            panic!("Expected Array");
        }
    }
}
