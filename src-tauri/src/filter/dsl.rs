use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    Field {
        field: String,
        op: FilterOp,
        value: String,
    },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Group(Box<FilterExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterOp {
    Eq,    // :
    Glob,  // :*
    Regex, // :~
    Gt,    // >
    Lt,    // <
    Gte,   // >=
    Lte,   // <=
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Field(String),
    Op(FilterOp),
    Value(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            let ch = self.input[self.pos];

            if ch.is_whitespace() {
                self.pos += 1;
                continue;
            }

            if ch == '(' {
                tokens.push(Token::LParen);
                self.pos += 1;
                continue;
            }

            if ch == ')' {
                tokens.push(Token::RParen);
                self.pos += 1;
                continue;
            }

            if ch.is_alphabetic() || ch == '_' {
                let start = self.pos;
                while self.pos < self.input.len()
                    && (self.input[self.pos].is_alphanumeric()
                        || self.input[self.pos] == '_'
                        || self.input[self.pos] == '.')
                {
                    self.pos += 1;
                }
                let word: String = self.input[start..self.pos].iter().collect();

                if word == "AND" {
                    tokens.push(Token::And);
                } else if word == "OR" {
                    tokens.push(Token::Or);
                } else if word == "NOT" {
                    tokens.push(Token::Not);
                } else if self.pos < self.input.len() && self.input[self.pos] == ':' {
                    self.pos += 1;
                    let op = if self.pos < self.input.len() && self.input[self.pos] == '*' {
                        self.pos += 1;
                        FilterOp::Glob
                    } else if self.pos < self.input.len() && self.input[self.pos] == '~' {
                        self.pos += 1;
                        FilterOp::Regex
                    } else {
                        FilterOp::Eq
                    };
                    // Consume the value after the operator
                    let start = self.pos;
                    while self.pos < self.input.len()
                        && (self.input[self.pos].is_alphanumeric()
                            || self.input[self.pos] == '_'
                            || self.input[self.pos] == '.'
                            || self.input[self.pos] == '-'
                            || self.input[self.pos] == '*')
                    {
                        self.pos += 1;
                    }
                    let value: String = self.input[start..self.pos].iter().collect();
                    if value.is_empty() {
                        return Err("Expected value after operator".to_string());
                    }
                    tokens.push(Token::Field(word));
                    tokens.push(Token::Op(op));
                    tokens.push(Token::Value(value));
                    continue;
                } else {
                    return Err(format!("Unexpected token: {}", word));
                }
                continue;
            }

            if ch == '>' || ch == '<' {
                let op = if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '=' {
                    self.pos += 1;
                    if ch == '>' {
                        FilterOp::Gte
                    } else {
                        FilterOp::Lte
                    }
                } else {
                    if ch == '>' {
                        FilterOp::Gt
                    } else {
                        FilterOp::Lt
                    }
                };
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos].is_numeric() {
                    self.pos += 1;
                }
                let value: String = self.input[start..self.pos].iter().collect();
                tokens.push(Token::Op(op));
                tokens.push(Token::Value(value));
                continue;
            }

            if ch == '"' || ch == '\'' {
                let quote = ch;
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos] != quote {
                    self.pos += 1;
                }
                let value: String = self.input[start..self.pos].iter().collect();
                self.pos += 1;
                tokens.push(Token::Value(value));
                continue;
            }

            return Err(format!("Unexpected character: {}", ch));
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }
}

pub fn parse(input: &str) -> Result<FilterExpr, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_expr()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_expr(&mut self) -> Result<FilterExpr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<FilterExpr, String> {
        let mut left = self.parse_and()?;

        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, String> {
        let mut left = self.parse_not()?;

        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_not()?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<FilterExpr, String> {
        if self.peek() == &Token::Not {
            self.advance();
            let expr = self.parse_not()?;
            return Ok(FilterExpr::Not(Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<FilterExpr, String> {
        match self.peek() {
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                if self.peek() != &Token::RParen {
                    return Err("Expected closing paren".to_string());
                }
                self.advance();
                Ok(FilterExpr::Group(Box::new(expr)))
            }
            Token::Field(name) => {
                let name = name.clone();
                self.advance();
                let op = match self.peek() {
                    Token::Op(op) => *op,
                    _ => return Err("Expected operator".to_string()),
                };
                self.advance();
                let value = match self.peek() {
                    Token::Value(v) => v.clone(),
                    _ => return Err("Expected value".to_string()),
                };
                self.advance();
                Ok(FilterExpr::Field {
                    field: name,
                    op,
                    value,
                })
            }
            _ => Err("Unexpected token".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field() {
        let result = parse("method:GET");
        assert!(result.is_ok());
    }

    #[test]
    fn test_and_expr() {
        let result = parse("method:GET AND status:200");
        assert!(result.is_ok());
    }

    #[test]
    fn test_glob() {
        let result = parse("host:*example.com");
        assert!(result.is_ok());
    }
}
