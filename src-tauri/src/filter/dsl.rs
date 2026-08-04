// Re-export AST types from the shared `expr` module so parser,
// evaluator, preset storage, and Tauri commands all see one
// representation.
pub use crate::filter::expr::{FilterExpr, FilterOp};

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
    /// Plain text search term (unquoted text not part of a field:op:value)
    Text(String),
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

                if word.eq_ignore_ascii_case("AND") {
                    tokens.push(Token::And);
                } else if word.eq_ignore_ascii_case("OR") {
                    tokens.push(Token::Or);
                } else if word.eq_ignore_ascii_case("NOT") {
                    tokens.push(Token::Not);
                } else if self.pos < self.input.len() && self.input[self.pos] == ':' {
                    self.pos += 1;
                    // Special-case `header:NAME:VALUE` triple syntax:
                    // after the first `:`, consume an IDENT for the header
                    // name and a second `:`, then run the normal op+value
                    // scan on the remainder. Plain `header:X` (no second
                    // colon) keeps the old shape and the parser routes it
                    // to the HeaderName variant with empty value.
                    let mut header_name: Option<String> = None;
                    if word == "header" {
                        let nstart = self.pos;
                        while self.pos < self.input.len()
                            && (self.input[self.pos].is_alphanumeric()
                                || self.input[self.pos] == '_'
                                || self.input[self.pos] == '-')
                        {
                            self.pos += 1;
                        }
                        let candidate: String = self.input[nstart..self.pos].iter().collect();
                        if !candidate.is_empty()
                            && self.pos < self.input.len()
                            && self.input[self.pos] == ':'
                        {
                            header_name = Some(candidate);
                            self.pos += 1; // consume the second `:`
                        } else {
                            // Rewind: no triple form, treat the consumed
                            // characters as the start of the value below.
                            self.pos = nstart;
                        }
                    }
                    let mut glob_prefix = false;
                    let op = match self.input.get(self.pos).copied() {
                        Some('*') => {
                            glob_prefix = true;
                            self.pos += 1;
                            FilterOp::Glob
                        }
                        Some('~') => {
                            self.pos += 1;
                            FilterOp::Regex
                        }
                        Some('>') => {
                            self.pos += 1;
                            if self.input.get(self.pos) == Some(&'=') {
                                self.pos += 1;
                                FilterOp::Gte
                            } else {
                                FilterOp::Gt
                            }
                        }
                        Some('<') => {
                            self.pos += 1;
                            if self.input.get(self.pos) == Some(&'=') {
                                self.pos += 1;
                                FilterOp::Lte
                            } else {
                                FilterOp::Lt
                            }
                        }
                        _ => FilterOp::Eq,
                    };
                    // Values consume the full token so regex characters and
                    // URL punctuation remain intact. Quoted values may contain
                    // whitespace; a closing quote is mandatory.
                    let mut value = if matches!(self.input.get(self.pos), Some('"' | '\'')) {
                        let quote = self.input[self.pos];
                        self.pos += 1;
                        let start = self.pos;
                        while self.pos < self.input.len() && self.input[self.pos] != quote {
                            self.pos += 1;
                        }
                        if self.pos == self.input.len() {
                            return Err("Unclosed quoted value".to_string());
                        }
                        let value: String = self.input[start..self.pos].iter().collect();
                        self.pos += 1;
                        value
                    } else {
                        let start = self.pos;
                        while self.pos < self.input.len()
                            && !self.input[self.pos].is_whitespace()
                            && self.input[self.pos] != ')'
                        {
                            self.pos += 1;
                        }
                        self.input[start..self.pos].iter().collect()
                    };
                    if glob_prefix {
                        value.insert(0, '*');
                    }
                    if value.is_empty() {
                        return Err("Expected value after operator".to_string());
                    }
                    tokens.push(Token::Field(word));
                    tokens.push(Token::Op(op));
                    if let Some(name) = header_name {
                        // Encode the triple as a synthetic
                        // `name\0value` so the parser can split it.
                        tokens.push(Token::Value(format!("{}\0{}", name, value)));
                    } else {
                        tokens.push(Token::Value(value));
                    }
                    continue;
                } else {
                    // Bare word without colon = plain text search
                    tokens.push(Token::Text(word));
                    continue;
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
    let expression = parser.parse_expr()?;
    if parser.peek() != &Token::EOF {
        return Err("Unexpected trailing token".to_string());
    }
    Ok(expression)
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

        loop {
            let explicit = self.peek() == &Token::And;
            let implicit = matches!(
                self.peek(),
                Token::Field(_) | Token::Text(_) | Token::LParen | Token::Not
            );
            if !explicit && !implicit {
                break;
            }
            if explicit {
                self.advance();
            }
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
                if name == "body" {
                    return Ok(FilterExpr::BodyText { op, value });
                }
                if name == "header" {
                    // Lexer may encode the `header:NAME:VALUE` triple
                    // as `NAME\0VALUE`; split here so the evaluator
                    // sees a clean HeaderName AST node.
                    if let Some(idx) = value.find('\0') {
                        let header_name = value[..idx].to_string();
                        let header_value = value[idx + 1..].to_string();
                        if header_name.is_empty() {
                            return Err("Empty header name".to_string());
                        }
                        return Ok(FilterExpr::HeaderName {
                            name: header_name,
                            op,
                            value: header_value,
                        });
                    }
                    return Ok(FilterExpr::HeaderName {
                        name: value.clone(),
                        op,
                        value: String::new(),
                    });
                }
                Ok(FilterExpr::Field {
                    field: name,
                    op,
                    value,
                })
            }
            Token::Text(text) => {
                let text = text.clone();
                self.advance();
                Ok(FilterExpr::Text(text))
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
    fn test_parse_header_field() {
        // `header:NAME` (single arg) parses to HeaderName with the
        // header name in the `name` slot and empty value — the
        // evaluator treats it as "header is present".
        let result = parse("header:content-type");
        assert!(result.is_ok());
        if let Ok(FilterExpr::HeaderName { name, op, value }) = result {
            assert_eq!(name, "content-type");
            assert_eq!(op, FilterOp::Eq);
            assert_eq!(value, "");
        } else {
            panic!("Expected HeaderName expr, got: {:?}", result);
        }
    }

    #[test]
    fn test_parse_header_field_triple() {
        // `header:NAME:VALUE` triple syntax parses to HeaderName with
        // both name and value populated.
        let result = parse("header:content-type:application/json");
        assert!(result.is_ok());
        if let Ok(FilterExpr::HeaderName { name, op, value }) = result {
            assert_eq!(name, "content-type");
            assert_eq!(op, FilterOp::Eq);
            assert_eq!(value, "application/json");
        } else {
            panic!("Expected HeaderName expr, got: {:?}", result);
        }
    }

    #[test]
    fn test_parse_body_field() {
        // The leading wildcard is both the glob operator marker and part of
        // the pattern so suffix/substring matching remains meaningful.
        let result = parse("body:*token*");
        assert!(result.is_ok());
        if let Ok(FilterExpr::BodyText { op, value }) = result {
            assert_eq!(op, FilterOp::Glob);
            assert_eq!(value, "*token*");
        } else {
            panic!("Expected BodyText expr, got: {:?}", result);
        }
    }

    #[test]
    fn test_glob() {
        let result = parse("host:*example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_search() {
        let result = parse("api");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            match expr {
                FilterExpr::Text(t) => assert_eq!(t, "api"),
                _ => panic!("Expected Text variant"),
            }
        }
    }

    #[test]
    fn test_field_with_extended_chars() {
        // Test URLs in values
        let result = parse("path:/api/v1/users");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_column_and_text() {
        // method:POST followed by bare text "api"
        let result = parse("method:POST api");
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_params() {
        // URL with query params
        let result = parse("path:/api?foo=bar");
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_comparison() {
        assert!(matches!(
            parse("status:>400"),
            Ok(FilterExpr::Field {
                op: FilterOp::Gt,
                value,
                ..
            }) if value == "400"
        ));
    }

    #[test]
    fn test_comparison_op() {
        assert!(matches!(
            parse("status:>=400"),
            Ok(FilterExpr::Field {
                op: FilterOp::Gte,
                value,
                ..
            }) if value == "400"
        ));
    }

    #[test]
    fn test_not_expr() {
        let result = parse("NOT method:POST");
        assert!(result.is_ok());
    }

    // ==================== LEXER TOKENIZE TESTS ====================

    #[test]
    fn test_tokenize_basic_field() {
        let mut lexer = Lexer::new("method:GET");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4); // Field, Op, Value, EOF
        assert_eq!(tokens[0], Token::Field("method".to_string()));
        assert_eq!(tokens[1], Token::Op(FilterOp::Eq));
        assert_eq!(tokens[2], Token::Value("GET".to_string()));
        assert_eq!(tokens[3], Token::EOF);
    }

    #[test]
    fn test_tokenize_glob_pattern() {
        let mut lexer = Lexer::new("host:*example.com");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], Token::Field("host".to_string()));
        assert_eq!(tokens[1], Token::Op(FilterOp::Glob));
        assert_eq!(tokens[2], Token::Value("*example.com".to_string()));
        assert_eq!(tokens[3], Token::EOF);
    }

    #[test]
    fn test_tokenize_regex_pattern() {
        let mut lexer = Lexer::new("path:~^/api/");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[1], Token::Op(FilterOp::Regex));
        assert_eq!(tokens[2], Token::Value("^/api/".to_string()));
    }

    #[test]
    fn test_parse_regex_field() {
        let result = parse("path:~^/api/");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tokenize_and_operator() {
        let mut lexer = Lexer::new("method:GET AND host:example.com");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 8); // Field, Op, Value, And, Field, Op, Value, EOF
        assert_eq!(tokens[3], Token::And);
    }

    #[test]
    fn test_tokenize_or_operator() {
        let mut lexer = Lexer::new("method:GET OR method:POST");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[3], Token::Or);
    }

    #[test]
    fn test_tokenize_not_operator() {
        let mut lexer = Lexer::new("NOT method:GET");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Not);
        assert_eq!(tokens[1], Token::Field("method".to_string()));
    }

    #[test]
    fn test_tokenize_parentheses() {
        let mut lexer = Lexer::new("(method:GET)");
        let tokens = lexer.tokenize().unwrap();
        // Tokens: LParen, Field, Op, Value, RParen, EOF
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0], Token::LParen);
        assert_eq!(tokens[4], Token::RParen);
        assert_eq!(tokens[5], Token::EOF);
    }

    #[test]
    fn test_tokenize_bare_text() {
        let mut lexer = Lexer::new("api");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 2); // Text, EOF
        assert_eq!(tokens[0], Token::Text("api".to_string()));
    }

    #[test]
    fn test_tokenize_quoted_string_double() {
        let mut lexer = Lexer::new("path:\"foo bar\"");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[2], Token::Value("foo bar".to_string()));
    }

    #[test]
    fn test_tokenize_quoted_string_single() {
        let mut lexer = Lexer::new("host:'example.com'");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[2], Token::Value("example.com".to_string()));
    }

    #[test]
    fn test_tokenize_comparison_gt() {
        // Standalone comparison: status>400
        // The lexer treats "status" as a Text token (no colon), then > as Op(Gt)
        let mut lexer = Lexer::new("status>400");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4); // Text("status"), Op(Gt), Value("400"), EOF
        assert_eq!(tokens[0], Token::Text("status".to_string()));
        assert_eq!(tokens[1], Token::Op(FilterOp::Gt));
        assert_eq!(tokens[2], Token::Value("400".to_string()));
    }

    #[test]
    fn test_tokenize_comparison_lt() {
        let mut lexer = Lexer::new("status<500");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Text("status".to_string()));
        assert_eq!(tokens[1], Token::Op(FilterOp::Lt));
    }

    #[test]
    fn test_tokenize_comparison_gte() {
        let mut lexer = Lexer::new("status>=400");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Text("status".to_string()));
        assert_eq!(tokens[1], Token::Op(FilterOp::Gte));
        assert_eq!(tokens[2], Token::Value("400".to_string()));
    }

    #[test]
    fn test_tokenize_comparison_lte() {
        let mut lexer = Lexer::new("status<=500");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Text("status".to_string()));
        assert_eq!(tokens[1], Token::Op(FilterOp::Lte));
    }

    #[test]
    fn test_tokenize_url_chars_in_value() {
        let mut lexer = Lexer::new("path:/api/v1/users?foo=bar&baz=qux");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(
            tokens[2],
            Token::Value("/api/v1/users?foo=bar&baz=qux".to_string())
        );
    }

    #[test]
    fn test_tokenize_empty_value_error() {
        let mut lexer = Lexer::new("method:");
        let result = lexer.tokenize();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Expected value after operator");
    }

    #[test]
    fn test_tokenize_unclosed_quote_error() {
        let mut lexer = Lexer::new("path:\"foo");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    // ==================== PARSE FUNCTION TESTS ====================

    #[test]
    fn test_parse_or_expr() {
        let result = parse("method:GET OR method:POST");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            match expr {
                FilterExpr::Or(_, _) => {}
                _ => panic!("Expected Or variant"),
            }
        }
    }

    #[test]
    fn test_parse_complex_grouping() {
        let result = parse("(method:GET OR method:POST) AND host:example.com");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            match expr {
                FilterExpr::And(_, _) => {}
                _ => panic!("Expected And variant for top-level"),
            }
        }
    }

    #[test]
    fn test_parse_nested_parens() {
        let result = parse("((method:GET))");
        assert!(result.is_ok());
        if let Ok(expr) = result {
            match expr {
                FilterExpr::Group(inner) => match *inner {
                    FilterExpr::Group(_) => {}
                    _ => panic!("Expected nested Group"),
                },
                _ => panic!("Expected outer Group"),
            }
        }
    }

    #[test]
    fn test_parse_regex_field_missing_chars() {
        let result = parse("path:~^/api/");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_mixed_and_text() {
        let result = parse("method:POST api");
        assert!(result.is_ok());
        assert!(matches!(result, Ok(FilterExpr::And(_, _))));
    }

    #[test]
    fn test_parse_quoted_value() {
        let result = parse("path:\"hello world\"");
        assert!(matches!(
            result,
            Ok(FilterExpr::Field { value, .. }) if value == "hello world"
        ));
    }

    #[test]
    fn test_parse_unclosed_paren_error() {
        let result = parse("(method:GET");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_rejects_unconsumed_tokens() {
        assert!(parse("method:GET status:").is_err());
        assert!(parse("method:GET)").is_err());
    }

    #[test]
    fn test_parse_empty_input() {
        let result = parse("");
        assert!(result.is_err());
    }
}
