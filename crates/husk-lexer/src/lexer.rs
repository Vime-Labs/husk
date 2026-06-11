use crate::token::{Span, Token, TokenKind};

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            input: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        let span = self.span();

        if self.pos >= self.input.len() {
            return Ok(Token {
                kind: TokenKind::Eof,
                span,
            });
        }

        let ch = self.current();

        let kind = match ch {
            '{' => {
                self.advance();
                TokenKind::LBrace
            }
            '}' => {
                self.advance();
                TokenKind::RBrace
            }
            '(' => {
                self.advance();
                TokenKind::LParen
            }
            ')' => {
                self.advance();
                TokenKind::RParen
            }
            '[' => {
                self.advance();
                TokenKind::LBracket
            }
            ']' => {
                self.advance();
                TokenKind::RBracket
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            '.' => {
                self.advance();
                if self.current_is('.') {
                    self.advance();
                    if self.current_is('.') {
                        self.advance();
                        TokenKind::DotDotDot
                    } else {
                        return Err(LexError::unexpected_char('.', span));
                    }
                } else {
                    TokenKind::Dot
                }
            }
            '%' => {
                self.advance();
                TokenKind::Percent
            }
            '*' => {
                self.advance();
                TokenKind::Star
            }
            '+' => {
                self.advance();
                TokenKind::Plus
            }
            '-' => {
                self.advance();
                TokenKind::Minus
            }
            '/' => {
                self.advance();
                TokenKind::Slash
            }
            '!' => {
                self.advance();
                if self.current_is('=') {
                    self.advance();
                    TokenKind::NotEq
                } else {
                    TokenKind::Bang
                }
            }
            '?' => {
                self.advance();
                TokenKind::Question
            }
            '=' => {
                self.advance();
                if self.current_is('=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            '<' => {
                self.advance();
                if self.current_is('=') {
                    self.advance();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                self.advance();
                if self.current_is('=') {
                    self.advance();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            '&' => {
                self.advance();
                if self.current_is('&') {
                    self.advance();
                    TokenKind::And
                } else {
                    return Err(LexError::unexpected_char('&', span));
                }
            }
            '|' => {
                self.advance();
                if self.current_is('|') {
                    self.advance();
                    TokenKind::Or
                } else {
                    return Err(LexError::unexpected_char('|', span));
                }
            }
            '"' => self.lex_string()?,
            c if c.is_ascii_digit() => self.lex_number()?,
            c if c.is_alphabetic() || c == '_' => self.lex_ident_or_keyword(),
            c => return Err(LexError::unexpected_char(c, span)),
        };

        Ok(Token { kind, span })
    }

    fn lex_string(&mut self) -> Result<TokenKind, LexError> {
        self.advance(); // abre aspas
        let mut s = String::new();
        loop {
            if self.pos >= self.input.len() {
                return Err(LexError {
                    message: "string não fechada".into(),
                    span: self.span(),
                });
            }
            match self.current() {
                '"' => {
                    self.advance();
                    break;
                }
                '\\' => {
                    self.advance();
                    let escaped = match self.current() {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        c => {
                            return Err(LexError {
                                message: format!("escape inválido: \\{}", c),
                                span: self.span(),
                            });
                        }
                    };
                    s.push(escaped);
                    self.advance();
                }
                c => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        Ok(TokenKind::Str(s))
    }

    fn lex_number(&mut self) -> Result<TokenKind, LexError> {
        let mut num = String::new();
        let mut is_float = false;

        while self.pos < self.input.len() && self.current().is_ascii_digit() {
            num.push(self.current());
            self.advance();
        }

        if self.pos < self.input.len() && self.current() == '.' {
            // garante que não é acesso a campo (ex: obj.campo)
            if self.pos + 1 < self.input.len() && self.input[self.pos + 1].is_ascii_digit() {
                is_float = true;
                num.push('.');
                self.advance();
                while self.pos < self.input.len() && self.current().is_ascii_digit() {
                    num.push(self.current());
                    self.advance();
                }
            }
        }

        if is_float {
            let f: f64 = num.parse().unwrap();
            Ok(TokenKind::Float(f))
        } else {
            let i: i64 = num.parse().unwrap();
            Ok(TokenKind::Int(i))
        }
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let mut ident = String::new();
        while self.pos < self.input.len()
            && (self.current().is_alphanumeric() || self.current() == '_')
        {
            ident.push(self.current());
            self.advance();
        }

        match ident.as_str() {
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "route" => TokenKind::Route,
            "middleware" => TokenKind::Middleware,
            "next" => TokenKind::Next,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "struct" => TokenKind::Struct,
            "GET" => TokenKind::Get,
            "POST" => TokenKind::Post,
            "PUT" => TokenKind::Put,
            "PATCH" => TokenKind::Patch,
            "DELETE" => TokenKind::Delete,
            "int" => TokenKind::TyInt,
            "string" => TokenKind::TyString,
            "bool" => TokenKind::TyBool,
            "float" => TokenKind::TyFloat,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            "nil" => TokenKind::Nil,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            _ => TokenKind::Ident(ident),
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // espaços e quebras de linha
            while self.pos < self.input.len() && self.current().is_whitespace() {
                if self.current() == '\n' {
                    self.line += 1;
                    self.col = 1;
                } else {
                    self.col += 1;
                }
                self.pos += 1;
            }

            // comentários de linha: //
            if self.pos + 1 < self.input.len()
                && self.input[self.pos] == '/'
                && self.input[self.pos + 1] == '/'
            {
                while self.pos < self.input.len() && self.current() != '\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn current(&self) -> char {
        self.input[self.pos]
    }

    fn current_is(&self, ch: char) -> bool {
        self.pos < self.input.len() && self.input[self.pos] == ch
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            col: self.col,
        }
    }
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl LexError {
    fn unexpected_char(ch: char, span: Span) -> Self {
        Self {
            message: format!("caractere inesperado: '{}'", ch),
            span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TokenKind;

    fn lex(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_route_hello() {
        let src = r#"
fn greeting() {
    return "Hello, World!"
}
route GET /hello {
    return greeting()
}
"#;
        let tokens = lex(src);
        assert!(tokens.contains(&TokenKind::Fn));
        assert!(tokens.contains(&TokenKind::Route));
        assert!(tokens.contains(&TokenKind::Get));
        assert!(tokens.contains(&TokenKind::Return));
        assert!(tokens.contains(&TokenKind::Str("Hello, World!".into())));
        assert!(tokens.contains(&TokenKind::Ident("greeting".into())));
    }

    #[test]
    fn test_operators() {
        let tokens = lex("== != <= >= && ||");
        assert_eq!(
            tokens,
            vec![
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_literals() {
        let tokens = lex("42 3.14 true false \"oi\"");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Int(42),
                TokenKind::Float(3.14),
                TokenKind::Bool(true),
                TokenKind::Bool(false),
                TokenKind::Str("oi".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comment_ignored() {
        let tokens = lex("let x = 1 // isso é um comentário\nlet y = 2");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Int(1),
                TokenKind::Let,
                TokenKind::Ident("y".into()),
                TokenKind::Eq,
                TokenKind::Int(2),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("fn return let if else for in route middleware next import as struct");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Fn,
                TokenKind::Return,
                TokenKind::Let,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::For,
                TokenKind::In,
                TokenKind::Route,
                TokenKind::Middleware,
                TokenKind::Next,
                TokenKind::Import,
                TokenKind::As,
                TokenKind::Struct,
                TokenKind::Eof,
            ]
        );
    }
}
