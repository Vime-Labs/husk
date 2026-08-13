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
        self.skip_whitespace();

        let span = self.span();

        if self.pos >= self.input.len() {
            return Ok(Token {
                kind: TokenKind::Eof,
                span,
            });
        }

        let ch = self.current();

        // go-block: a palavra "go" seguida de '{' captura o código Go bruto
        if (ch.is_alphabetic() || ch == '_') && self.starts_with_word("go") {
            if self.peek_significant_after_word() == Some('{') {
                self.advance(); // 'g'
                self.advance(); // 'o'
                let kind = self.lex_go_block()?;
                return Ok(Token { kind, span });
            }
        }

        // comentário de linha: //
        if ch == '/' && self.pos + 1 < self.input.len() && self.input[self.pos + 1] == '/' {
            return Ok(Token {
                kind: TokenKind::Comment(self.lex_comment()),
                span,
            });
        }

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
            "cors" => TokenKind::Cors,
            "next" => TokenKind::Next,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "struct" => TokenKind::Struct,
            "go" => TokenKind::Go,
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
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "retry" => TokenKind::Retry,
            "break" => TokenKind::Break,
            "schema" => TokenKind::Schema,
            "required" => TokenKind::Required,
            "model" => TokenKind::Model,
            _ => TokenKind::Ident(ident),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current().is_whitespace() {
            if self.current() == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn lex_comment(&mut self) -> String {
        self.advance(); // first /
        self.advance(); // second /
        let mut content = String::new();
        while self.pos < self.input.len() && self.current() != '\n' {
            content.push(self.current());
            self.advance();
        }
        content
    }

    fn peek_next(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    /// Verifica se a palavra (ident) atual é exatamente `word`, com fronteira
    /// de identificador após ela (ex.: "go" sim, "gorilla" não).
    fn starts_with_word(&self, word: &str) -> bool {
        let w: Vec<char> = word.chars().collect();
        if self.pos + w.len() > self.input.len() {
            return false;
        }
        for (i, c) in w.iter().enumerate() {
            if self.input[self.pos + i] != *c {
                return false;
            }
        }
        if let Some(&next) = self.input.get(self.pos + w.len()) {
            !(next.is_alphanumeric() || next == '_')
        } else {
            true
        }
    }

    /// Olha à frente (sem consumir) o próximo caractere significativo após a
    /// palavra atual, ignorando espaços e comentários.
    fn peek_significant_after_word(&self) -> Option<char> {
        let mut i = self.pos + 2; // depois de "go"
        let mut in_block_comment = false;
        while i < self.input.len() {
            let c = self.input[i];
            if in_block_comment {
                if c == '*' && i + 1 < self.input.len() && self.input[i + 1] == '/' {
                    i += 2;
                    in_block_comment = false;
                } else {
                    i += 1;
                }
                continue;
            }
            if c.is_whitespace() {
                i += 1;
                continue;
            }
            if c == '/' && i + 1 < self.input.len() && self.input[i + 1] == '/' {
                while i < self.input.len() && self.input[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if c == '/' && i + 1 < self.input.len() && self.input[i + 1] == '*' {
                in_block_comment = true;
                i += 2;
                continue;
            }
            return Some(c);
        }
        None
    }

    /// Pula espaços e comentários sem emitir tokens (usado antes de um go-block).
    fn skip_go_trivia(&mut self) {
        loop {
            self.skip_whitespace();
            if self.current_is('/') && self.peek_next() == Some('/') {
                while self.pos < self.input.len() && self.current() != '\n' {
                    self.advance();
                }
                continue;
            }
            if self.current_is('/') && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                while self.pos + 1 < self.input.len()
                    && !(self.current() == '*' && self.peek_next() == Some('/'))
                {
                    self.advance();
                }
                if self.pos + 1 < self.input.len() {
                    self.advance();
                    self.advance();
                }
                continue;
            }
            break;
        }
    }

    /// Captura o conteúdo bruto de um bloco `go { ... }` (sem as chaves externas).
    /// O scanner entende strings, raw strings e comentários do Go para não
    /// confundir chaves dentro de literais/comentários com o fim do bloco.
    fn lex_go_block(&mut self) -> Result<TokenKind, LexError> {
        self.skip_go_trivia();
        if !self.current_is('{') {
            return Err(LexError {
                message: "esperado '{' após 'go' para bloco Go".into(),
                span: self.span(),
            });
        }
        let start = self.pos;
        let mut depth: i64 = 0;
        let mut in_str = false;     // "..."
        let mut in_raw = false;     // `...`
        let mut in_char = false;    // '...'
        let mut in_line_c = false;  // //...
        let mut in_block_c = false; // /*...*/

        while self.pos < self.input.len() {
            let c = self.current();
            if in_line_c {
                if c == '\n' {
                    in_line_c = false;
                }
                self.advance();
                continue;
            }
            if in_block_c {
                if c == '*' && self.peek_next() == Some('/') {
                    self.advance();
                    self.advance();
                    in_block_c = false;
                } else {
                    self.advance();
                }
                continue;
            }
            if in_str {
                if c == '\\' {
                    self.advance();
                    if self.pos < self.input.len() {
                        self.advance();
                    }
                } else if c == '"' {
                    in_str = false;
                    self.advance();
                } else {
                    self.advance();
                }
                continue;
            }
            if in_raw {
                if c == '`' {
                    in_raw = false;
                }
                self.advance();
                continue;
            }
            if in_char {
                if c == '\\' {
                    self.advance();
                    if self.pos < self.input.len() {
                        self.advance();
                    }
                } else if c == '\'' {
                    in_char = false;
                    self.advance();
                } else {
                    self.advance();
                }
                continue;
            }
            match c {
                '/' if self.peek_next() == Some('/') => {
                    in_line_c = true;
                    self.advance();
                }
                '/' if self.peek_next() == Some('*') => {
                    in_block_c = true;
                    self.advance();
                    self.advance();
                }
                '"' => {
                    in_str = true;
                    self.advance();
                }
                '`' => {
                    in_raw = true;
                    self.advance();
                }
                '\'' => {
                    in_char = true;
                    self.advance();
                }
                '{' => {
                    depth += 1;
                    self.advance();
                }
                '}' => {
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        let inner: String = self.input[start + 1..self.pos - 1].iter().collect();
                        return Ok(TokenKind::GoBlock(inner.trim().to_string()));
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        Err(LexError {
            message: "bloco go não fechado: falta '}'".into(),
            span: self.span(),
        })
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
    fn test_comment_as_token() {
        let tokens = lex("let x = 1 // isso é um comentário\nlet y = 2");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Int(1),
                TokenKind::Comment(" isso é um comentário".into()),
                TokenKind::Let,
                TokenKind::Ident("y".into()),
                TokenKind::Eq,
                TokenKind::Int(2),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comment_multi_line() {
        let tokens = lex(
            "// primeiro\n// segundo\nlet x = 1",
        );
        assert_eq!(
            tokens,
            vec![
                TokenKind::Comment(" primeiro".into()),
                TokenKind::Comment(" segundo".into()),
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Int(1),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("fn return let if else for in try catch retry break schema required model route middleware next import as struct");
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
                TokenKind::Try,
                TokenKind::Catch,
                TokenKind::Retry,
                TokenKind::Break,
                TokenKind::Schema,
                TokenKind::Required,
                TokenKind::Model,
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

    #[test]
    fn test_go_import() {
        let tokens = lex(r#"go "lib/x.go" as x"#);
        assert_eq!(
            tokens,
            vec![
                TokenKind::Go,
                TokenKind::Str("lib/x.go".into()),
                TokenKind::As,
                TokenKind::Ident("x".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_go_block() {
        let tokens = lex("go {\n    func a() string {\n        return \"a\"\n    }\n}\n");
        assert_eq!(tokens.len(), 2, "esperado GoBlock + Eof, veio: {:?}", tokens);
        match &tokens[0] {
            TokenKind::GoBlock(src) => {
                assert!(src.contains("func a() string"));
                assert!(src.contains("return \"a\""));
            }
            other => panic!("esperado GoBlock, veio {:?}", other),
        }
    }

    #[test]
    fn test_go_block_ignora_chaves_em_strings_e_comentarios() {
        // chaves dentro de strings, raw strings e comentários não fecham o bloco
        let tokens = lex(
            "go {\n\tfunc f() string { return \"}\" + `{` } // fecha: }\n\t/* ainda aberto } aqui */\n}\n",
        );
        assert_eq!(tokens.len(), 2, "esperado GoBlock + Eof, veio: {:?}", tokens);
        match &tokens[0] {
            TokenKind::GoBlock(src) => {
                assert!(src.contains("func f() string"));
                assert!(src.contains("// fecha: }"));
                assert!(src.contains("/* ainda aberto } aqui */"));
            }
            other => panic!("esperado GoBlock, veio {:?}", other),
        }
    }

    #[test]
    fn test_go_block_com_espaco_e_comentario() {
        // go seguido de comentário antes de { também é bloco
        let tokens = lex("go // helper\n{\nfunc a() {}\n}\n");
        match &tokens[0] {
            TokenKind::GoBlock(src) => assert_eq!(src.trim(), "func a() {}"),
            other => panic!("esperado GoBlock, veio {:?}", other),
        }
    }

    #[test]
    fn test_go_ident_nao_e_bloco() {
        // "gorilla" e "go_" não são bloco go
        let tokens = lex("gorilla go_x");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Ident("gorilla".into()),
                TokenKind::Ident("go_x".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_go_block_nao_fechado_erro() {
        let err = Lexer::new("go {\nfunc a() {}").tokenize().unwrap_err();
        assert!(err.message.contains("não fechado"));
    }

    #[test]
    fn test_go_import_seguido_de_go_block() {
        // regressão: go import antes de um go block quebrava a captura do bloco
        let tokens = lex("go \"lib/x.go\" as x\n\ngo {\nfunc f() int { return 1 }\n}\n");
        assert_eq!(tokens.len(), 6, "esperado Go, Str, As, Ident, GoBlock, Eof → veio: {:?}", tokens);
        match &tokens[4] {
            TokenKind::GoBlock(src) => assert!(src.contains("func f() int")),
            other => panic!("esperado GoBlock no índice 4, veio {:?}", other),
        }
    }

    #[test]
    fn test_go_block_crlf() {
        // arquivos Windows com \r\n
        let tokens = lex("go \"lib/x.go\" as x\r\n\r\ngo {\r\n    func f() int {\r\n        return 1\r\n    }\r\n}\r\n");
        assert_eq!(tokens.len(), 6, "veio: {:?}", tokens);
        match &tokens[4] {
            TokenKind::GoBlock(src) => assert!(src.contains("func f() int")),
            other => panic!("esperado GoBlock no índice 4, veio {:?}", other),
        }
    }
}
