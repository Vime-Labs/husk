use crate::ast::*;
use husk_lexer::{Span, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    fn new(msg: impl Into<String>, span: Span) -> Self {
        Self {
            message: msg.into(),
            span,
        }
    }
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let tokens: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !matches!(t.kind, husk_lexer::TokenKind::Comment(_)))
            .collect();
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.at_eof() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        match self.current_kind() {
            TokenKind::Fn => Ok(Item::FnDef(self.parse_fn()?)),
            TokenKind::Route => Ok(Item::RouteDef(self.parse_route()?)),
            TokenKind::Struct => Ok(Item::StructDef(self.parse_struct()?)),
            TokenKind::Import => Ok(Item::Import(self.parse_import()?)),
            TokenKind::Middleware => Ok(Item::MiddlewareDef(self.parse_middleware()?)),
            TokenKind::Cors => Ok(Item::CorsDef(self.parse_cors()?)),
            _ => Err(ParseError::new(
                format!(
                    "esperado item top-level (fn/route/struct/import/middleware/cors), encontrado {:?}",
                    self.current_kind()
                ),
                self.current_span(),
            )),
        }
    }

    // fn nome(param tipo, ...) tipo_retorno { ... }
    fn parse_fn(&mut self) -> Result<FnDef, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let return_type = self.parse_return_type()?;
        let body = self.parse_block()?;
        Ok(FnDef {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RParen | TokenKind::Eof) {
            let name = self.expect_ident()?;
            let ty = self.parse_type()?;
            params.push(Param { name, ty });
            if matches!(self.current_kind(), TokenKind::Comma) {
                self.advance();
            }
        }
        Ok(params)
    }

    /// Parseia o tipo de retorno após os parâmetros:
    /// - (vazio) → ReturnType::None
    /// - `tipo`  → ReturnType::Single
    /// - `(tipo, error)` → ReturnType::Tuple
    fn parse_return_type(&mut self) -> Result<ReturnType, ParseError> {
        if matches!(self.current_kind(), TokenKind::LBrace) {
            return Ok(ReturnType::None);
        }
        if matches!(self.current_kind(), TokenKind::LParen) {
            self.advance();
            let mut types = Vec::new();
            while !matches!(self.current_kind(), TokenKind::RParen | TokenKind::Eof) {
                types.push(self.parse_type()?);
                if matches!(self.current_kind(), TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen)?;
            return Ok(ReturnType::Tuple(types));
        }
        if self.current_is_type() {
            return Ok(ReturnType::Single(self.parse_type()?));
        }
        Ok(ReturnType::None)
    }

    fn parse_struct(&mut self) -> Result<StructDef, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RBrace | TokenKind::Eof) {
            let field_name = self.expect_ident()?;
            let ty = self.parse_type()?;
            fields.push(StructField {
                name: field_name,
                ty,
            });
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StructDef { name, fields, span })
    }

    fn parse_import(&mut self) -> Result<ImportDef, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Import)?;
        let path = match self.current_kind().clone() {
            TokenKind::Str(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError::new(
                    format!(
                        "esperado caminho string após 'import', encontrado {:?}",
                        self.current_kind()
                    ),
                    self.current_span(),
                ));
            }
        };
        self.expect(TokenKind::As)?;
        let alias = self.expect_ident()?;
        let is_stdlib = path.starts_with("husk/");
        Ok(ImportDef {
            path,
            alias,
            is_stdlib,
            span,
        })
    }

    // middleware nome { corpo }
    fn parse_middleware(&mut self) -> Result<MiddlewareDef, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Middleware)?;
        let name = self.expect_ident()?;
        // contexto opcional: -> ctx_var
        let ctx_var = self.parse_arrow_ident()?;
        let body = self.parse_block()?;
        Ok(MiddlewareDef {
            name,
            ctx_var,
            body,
            span,
        })
    }

    // cors { origins: [...] methods: [...] headers: [...] }
    fn parse_cors(&mut self) -> Result<CorsDef, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Cors)?;
        self.expect(TokenKind::LBrace)?;

        let mut origins: Vec<String> = Vec::new();
        let mut methods: Vec<String> = Vec::new();
        let mut headers: Vec<String> = Vec::new();

        while !matches!(self.current_kind(), TokenKind::RBrace | TokenKind::Eof) {
            let field = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let values = self.parse_string_array()?;
            match field.as_str() {
                "origins" => origins = values,
                "methods" => methods = values,
                "headers" => headers = values,
                other => {
                    return Err(ParseError::new(
                        format!(
                            "campo cors desconhecido: '{}' (esperado: origins, methods, headers)",
                            other
                        ),
                        self.current_span(),
                    ))
                }
            }
        }

        self.expect(TokenKind::RBrace)?;
        Ok(CorsDef {
            origins,
            methods,
            headers,
            span,
        })
    }

    fn parse_string_array(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(TokenKind::LBracket)?;
        let mut values = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RBracket | TokenKind::Eof) {
            match self.current_kind().clone() {
                TokenKind::Str(s) => {
                    self.advance();
                    values.push(s);
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "esperado string no array cors, encontrado {:?}",
                            self.current_kind()
                        ),
                        self.current_span(),
                    ))
                }
            }
            if matches!(self.current_kind(), TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBracket)?;
        Ok(values)
    }

    // route GET /caminho [mw1, mw2] [-> ctx_var] { ... }
    fn parse_route(&mut self) -> Result<RouteDef, ParseError> {
        let span = self.current_span();
        self.expect(TokenKind::Route)?;

        let method = match self.current_kind() {
            TokenKind::Get => {
                self.advance();
                HttpMethod::Get
            }
            TokenKind::Post => {
                self.advance();
                HttpMethod::Post
            }
            TokenKind::Put => {
                self.advance();
                HttpMethod::Put
            }
            TokenKind::Patch => {
                self.advance();
                HttpMethod::Patch
            }
            TokenKind::Delete => {
                self.advance();
                HttpMethod::Delete
            }
            _ => {
                return Err(ParseError::new(
                    format!("esperado método HTTP, encontrado {:?}", self.current_kind()),
                    self.current_span(),
                ));
            }
        };

        let path = self.parse_route_path()?;

        // middlewares opcionais: [mw1, mw2]
        let middlewares = if matches!(self.current_kind(), TokenKind::LBracket) {
            self.advance();
            let mut mws = Vec::new();
            while !matches!(self.current_kind(), TokenKind::RBracket | TokenKind::Eof) {
                mws.push(self.expect_ident()?);
                if matches!(self.current_kind(), TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
            mws
        } else {
            Vec::new()
        };

        // contexto opcional: -> ctx_var (pode vir sem middlewares também)
        let ctx_var = self.parse_arrow_ident()?;

        let body = self.parse_block()?;
        Ok(RouteDef {
            method,
            path,
            middlewares,
            ctx_var,
            body,
            span,
        })
    }

    fn parse_route_path(&mut self) -> Result<RoutePath, ParseError> {
        let mut segments = Vec::new();
        loop {
            if !matches!(self.current_kind(), TokenKind::Slash) {
                break;
            }
            self.advance();
            match self.current_kind().clone() {
                TokenKind::LBrace | TokenKind::LBracket | TokenKind::Eof => break,
                TokenKind::Ident(name) => {
                    self.advance();
                    segments.push(PathSegment::Literal(name));
                }
                TokenKind::Colon => {
                    self.advance();
                    let n = self.expect_ident()?;
                    // :name<type> — tipo opcional
                    let ty = if matches!(self.current_kind(), TokenKind::Lt) {
                        self.advance();
                        let t = self.parse_type()?;
                        self.expect(TokenKind::Gt)?;
                        Some(t)
                    } else {
                        None
                    };
                    segments.push(PathSegment::Param(n, ty));
                }
                TokenKind::Int(n) => {
                    let lit = n.to_string();
                    self.advance();
                    segments.push(PathSegment::Literal(lit));
                }
                _ => break,
            }
        }
        Ok(RoutePath { segments })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RBrace | TokenKind::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.current_kind() {
            TokenKind::Return => {
                self.advance();
                let mut exprs = vec![self.parse_expr()?];
                while matches!(self.current_kind(), TokenKind::Comma) {
                    self.advance();
                    exprs.push(self.parse_expr()?);
                }
                Ok(Stmt::Return(exprs))
            }
            TokenKind::Let => self.parse_let(),
            TokenKind::If => Ok(Stmt::If(self.parse_if()?)),
            TokenKind::For => Ok(Stmt::ForIn(self.parse_for_in()?)),
            _ => {
                let expr = self.parse_expr()?;
                // ctx.field = value  — assignment
                if matches!(self.current_kind(), TokenKind::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    Ok(Stmt::Assign(AssignStmt {
                        target: Box::new(expr),
                        value,
                    }))
                } else {
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Let)?;
        let first = self.expect_ident()?;

        // let a, b = expr  (multi-atribuição)
        if matches!(self.current_kind(), TokenKind::Comma) {
            self.advance();
            let mut names = vec![first];
            loop {
                names.push(self.expect_ident()?);
                if !matches!(self.current_kind(), TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr()?;
            return Ok(Stmt::LetMulti(LetMultiStmt { names, value }));
        }

        // let nome tipo = expr  ou  let nome = expr
        let ty = if self.current_is_type() {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let(LetStmt {
            name: first,
            ty,
            value,
        }))
    }

    fn parse_for_in(&mut self) -> Result<ForInStmt, ParseError> {
        self.expect(TokenKind::For)?;
        let item = self.expect_ident()?;
        self.expect(TokenKind::In)?;
        let collection = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(ForInStmt {
            item,
            collection,
            body,
        })
    }

    fn parse_if(&mut self) -> Result<IfStmt, ParseError> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_block = if matches!(self.current_kind(), TokenKind::Else) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(IfStmt {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while matches!(self.current_kind(), TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while matches!(self.current_kind(), TokenKind::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinOp(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.current_kind() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            left = Expr::BinOp(Box::new(left), op, Box::new(self.parse_comparison()?));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.current_kind() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            left = Expr::BinOp(Box::new(left), op, Box::new(self.parse_additive()?));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.current_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            left = Expr::BinOp(Box::new(left), op, Box::new(self.parse_multiplicative()?));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.current_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            left = Expr::BinOp(Box::new(left), op, Box::new(self.parse_unary()?));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            TokenKind::Bang => {
                self.advance();
                Ok(Expr::Unary(UnaryOp::Not, Box::new(self.parse_unary()?)))
            }
            TokenKind::Minus => {
                self.advance();
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(self.parse_unary()?)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.current_kind() {
                TokenKind::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(TokenKind::RParen)?;
                    expr = Expr::Call(CallExpr {
                        callee: Box::new(expr),
                        args,
                    });
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    expr = Expr::FieldAccess(Box::new(expr), field);
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                TokenKind::LBrace => {
                    if let Expr::Ident(name) = &expr {
                        if self.lookahead_is_struct_init() {
                            let name = name.clone();
                            self.advance(); // consome {
                            let fields = self.parse_field_list()?;
                            self.expect(TokenKind::RBrace)?;
                            expr = Expr::StructInit(StructInit { name, fields });
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                TokenKind::Question => {
                    self.advance(); // consome ?
                    let status_code = if let TokenKind::Int(n) = self.current_kind() {
                        let n = *n;
                        self.advance();
                        Some(n)
                    } else {
                        None
                    };
                    let message = if let TokenKind::Str(s) = self.current_kind().clone() {
                        self.advance();
                        Some(s)
                    } else {
                        None
                    };
                    expr = Expr::Try(TryExpr {
                        expr: Box::new(expr),
                        status_code,
                        message,
                    });
                }
                TokenKind::DotDotDot => {
                    self.advance(); // consome ...
                    expr = Expr::Spread(Box::new(expr));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RParen | TokenKind::Eof) {
            args.push(self.parse_expr()?);
            if matches!(self.current_kind(), TokenKind::Comma) {
                self.advance();
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind().clone() {
            TokenKind::Int(n) => {
                self.advance();
                Ok(Expr::Lit(Lit::Int(n)))
            }
            TokenKind::Float(f) => {
                self.advance();
                Ok(Expr::Lit(Lit::Float(f)))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Lit(Lit::Str(s)))
            }
            TokenKind::Bool(b) => {
                self.advance();
                Ok(Expr::Lit(Lit::Bool(b)))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            // next é keyword mas usado como expressão dentro de middleware
            TokenKind::Next => {
                self.advance();
                Ok(Expr::Ident("next".into()))
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            TokenKind::TyFloat => {
                self.advance();
                Ok(Expr::Ident("float".into()))
            }
            TokenKind::TyString => {
                self.advance();
                Ok(Expr::Ident("string".into()))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBrace => {
                self.advance();
                let fields = self.parse_field_list()?;
                self.expect(TokenKind::RBrace)?;
                Ok(Expr::MapLit(MapLit { fields }))
            }
            _ => Err(ParseError::new(
                format!("expressão inesperada: {:?}", self.current_kind()),
                self.current_span(),
            )),
        }
    }

    fn parse_field_list(&mut self) -> Result<Vec<(String, Expr)>, ParseError> {
        let mut fields = Vec::new();
        while !matches!(self.current_kind(), TokenKind::RBrace | TokenKind::Eof) {
            let key = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let val = self.parse_expr()?;
            fields.push((key, val));
            if matches!(self.current_kind(), TokenKind::Comma) {
                self.advance();
            }
        }
        Ok(fields)
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        // []tipo — lista
        if matches!(self.current_kind(), TokenKind::LBracket) {
            self.advance(); // [
            self.expect(TokenKind::RBracket)?; // ]
            let inner = self.parse_type()?;
            return Ok(Type::List(Box::new(inner)));
        }
        match self.current_kind().clone() {
            TokenKind::TyInt => {
                self.advance();
                Ok(Type::Int)
            }
            TokenKind::TyFloat => {
                self.advance();
                Ok(Type::Float)
            }
            TokenKind::TyString => {
                self.advance();
                Ok(Type::String)
            }
            TokenKind::TyBool => {
                self.advance();
                Ok(Type::Bool)
            }
            TokenKind::Ident(name) if name == "error" => {
                self.advance();
                Ok(Type::Error)
            }
            TokenKind::Ident(name) if name == "map" => {
                self.advance();
                Ok(Type::Map)
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Type::Named(name))
            }
            _ => Err(ParseError::new(
                format!("tipo esperado, encontrado {:?}", self.current_kind()),
                self.current_span(),
            )),
        }
    }

    // --- helpers ---

    fn current_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }
    fn current_span(&self) -> Span {
        self.tokens[self.pos].span.clone()
    }
    fn at_eof(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn advance(&mut self) {
        if !self.at_eof() {
            self.pos += 1;
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.current_kind() == &kind {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                format!("esperado {:?}, encontrado {:?}", kind, self.current_kind()),
                self.current_span(),
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        if let TokenKind::Ident(name) = self.current_kind().clone() {
            self.advance();
            Ok(name)
        } else {
            Err(ParseError::new(
                format!(
                    "esperado identificador, encontrado {:?}",
                    self.current_kind()
                ),
                self.current_span(),
            ))
        }
    }

    fn current_is_type(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::TyInt
                | TokenKind::TyFloat
                | TokenKind::TyString
                | TokenKind::TyBool
                | TokenKind::Ident(_)
                | TokenKind::LBracket
        )
    }

    /// Lookahead: token no índice `pos + offset` sem avançar
    /// Tenta parsear -> ident (arrow seguido de identificador)
    /// Retorna Some(nome) se encontrou, None caso contrário
    fn parse_arrow_ident(&mut self) -> Result<Option<String>, ParseError> {
        if matches!(self.current_kind(), TokenKind::Minus) && matches!(self.peek(1), TokenKind::Gt)
        {
            self.advance(); // consume Minus
            self.advance(); // consume Gt
            let name = self.expect_ident()?;
            Ok(Some(name))
        } else {
            Ok(None)
        }
    }

    fn peek(&self, offset: usize) -> &TokenKind {
        let idx = self.pos + offset;
        if idx < self.tokens.len() {
            &self.tokens[idx].kind
        } else {
            &TokenKind::Eof
        }
    }

    /// Verifica se o `{` atual é início de inicialização de struct
    /// (evita conflito com bloco de if/else/for/rota)
    /// Só é struct init se depois de `{` vier:
    ///   - `ident:` (campo nomeado)
    ///   - `}` (struct vazio)
    fn lookahead_is_struct_init(&self) -> bool {
        debug_assert!(matches!(self.current_kind(), TokenKind::LBrace));

        let after_lbrace = self.peek(1);

        // StructName {} — struct vazio
        if matches!(after_lbrace, TokenKind::RBrace) {
            return true;
        }

        // StructName { campo: valor, ... }
        if let TokenKind::Ident(_) = after_lbrace {
            if matches!(self.peek(2), TokenKind::Colon) {
                return true;
            }
        }

        false
    }
}
