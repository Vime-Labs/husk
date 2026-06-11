use crate::scope::{FnSignature, Scope, SemanticError, Symbol, TypeInfo};
use husk_lexer::Span;
use husk_parser::ast::*;

/// Contexto de análise — em que tipo de bloco estamos
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ctx {
    Fn,
    Route,
    Middleware,
}

/// Analisador semântico: verifica tipos, escopos e regras da linguagem
pub struct Checker {
    global: Scope,
    errors: Vec<SemanticError>,
}

impl Checker {
    pub fn new() -> Self {
        let mut global = Scope::new();
        // Built-in functions disponíveis em qualquer escopo
        for builtin in &["json", "text", "status", "set_ctx"] {
            let _ = global.declare(
                builtin,
                Symbol::Function(FnSignature {
                    params: vec![],
                    return_types: vec![],
                }),
                &Span { line: 0, col: 0 },
            );
        }
        Self {
            global,
            errors: Vec::new(),
        }
    }

    /// Executa a análise completa de um programa
    pub fn check(&mut self, program: &Program) -> &[SemanticError] {
        self.collect_top_level(program);

        for item in &program.items {
            match item {
                Item::FnDef(f) => self.check_fn(f),
                Item::RouteDef(r) => self.check_route(r),
                Item::MiddlewareDef(m) => self.check_middleware(m),
                _ => {}
            }
        }

        &self.errors
    }

    // ---- Coleta de definições top-level ----

    fn collect_top_level(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::FnDef(f) => {
                    let sig = FnSignature {
                        params: f
                            .params
                            .iter()
                            .map(|p| (p.name.clone(), TypeInfo::from_ast(&p.ty)))
                            .collect(),
                        return_types: return_types_from_ast(&f.return_type),
                    };
                    if let Err(e) = self.global.declare(&f.name, Symbol::Function(sig), &f.span) {
                        self.errors.push(e);
                    }
                }
                Item::StructDef(s) => {
                    if let Err(e) =
                        self.global
                            .declare(&s.name, Symbol::Struct(s.fields.clone()), &s.span)
                    {
                        self.errors.push(e);
                    }
                }
                Item::MiddlewareDef(m) => {
                    if let Err(e) = self.global.declare(&m.name, Symbol::Middleware, &m.span) {
                        self.errors.push(e);
                    }
                }
                Item::Import(imp) => {
                    let _ = self.global.declare(&imp.alias, Symbol::Module, &imp.span);
                }
                Item::RouteDef(_) => {} // rotas são verificadas na segunda passada
            }
        }
    }

    // ---- Verificação de corpos ----

    fn check_fn(&mut self, f: &FnDef) {
        let mut scope = self.global.child();

        for p in &f.params {
            let ty = TypeInfo::from_ast(&p.ty);
            if let Err(e) = scope.declare(&p.name, Symbol::Variable(ty), &Span { line: 0, col: 0 })
            {
                self.errors.push(e);
            }
        }

        let declared_returns = return_types_from_ast(&f.return_type);
        let inferred_returns = self.check_block(&f.body, &mut scope, Ctx::Fn);

        // Verifica compatibilidade dos returns com o tipo declarado
        if !declared_returns.is_empty() {
            for ret_types in &inferred_returns {
                for (i, inferred) in ret_types.iter().enumerate() {
                    if i < declared_returns.len() {
                        self.check_type_compat(&declared_returns[i], inferred, &f.span);
                    }
                }
                if ret_types.len() > declared_returns.len() {
                    self.errors.push(SemanticError::new(
                        format!(
                            "retorno tem mais valores que o declarado ({})",
                            declared_returns.len()
                        ),
                        f.span.clone(),
                    ));
                    break;
                }
            }
        }
    }

    fn check_route(&mut self, r: &RouteDef) {
        for mw in &r.middlewares {
            match self.global.lookup(mw) {
                Some(Symbol::Middleware) => {}
                Some(_) => {
                    self.errors.push(SemanticError::new(
                        format!("'{}' não é um middleware", mw),
                        r.span.clone(),
                    ));
                }
                None => {
                    self.errors.push(SemanticError::new(
                        format!("middleware '{}' não definido", mw),
                        r.span.clone(),
                    ));
                }
            }
        }

        let mut scope = self.global.child();
        for seg in &r.path.segments {
            if let PathSegment::Param(name) = seg {
                let _ = scope.declare(name, Symbol::Variable(TypeInfo::String), &r.span);
            }
        }

        self.check_block(&r.body, &mut scope, Ctx::Route);
    }

    fn check_middleware(&mut self, m: &MiddlewareDef) {
        let mut scope = self.global.child();
        self.check_block(&m.body, &mut scope, Ctx::Middleware);
    }

    // ---- Blocos ----

    fn check_block(&mut self, block: &Block, scope: &mut Scope, ctx: Ctx) -> Vec<Vec<TypeInfo>> {
        let mut return_types = Vec::new();
        for stmt in &block.stmts {
            return_types.extend(self.check_stmt(stmt, scope, ctx));
        }
        return_types
    }

    // ---- Statements ----

    fn check_stmt(&mut self, stmt: &Stmt, scope: &mut Scope, ctx: Ctx) -> Vec<Vec<TypeInfo>> {
        match stmt {
            Stmt::Return(exprs) => {
                let types: Vec<TypeInfo> = exprs
                    .iter()
                    .map(|e| self.check_expr(e, scope, ctx))
                    .collect();
                if ctx == Ctx::Route && types.len() > 1 {
                    self.errors.push(SemanticError::new(
                        "return em rota deve ter no máximo 1 expressão (use json/text/status para respostas)",
                        Span { line: 0, col: 0 },
                    ));
                }
                vec![types]
            }
            Stmt::Let(l) => {
                let value_ty = self.check_expr(&l.value, scope, ctx);

                if let Some(annotated) = &l.ty {
                    let annotated_ty = TypeInfo::from_ast(annotated);
                    self.check_type_compat(&annotated_ty, &value_ty, &Span { line: 0, col: 0 });
                    scope.declare_or_shadow(&l.name, Symbol::Variable(annotated_ty));
                } else {
                    scope.declare_or_shadow(&l.name, Symbol::Variable(value_ty));
                }
                vec![]
            }
            Stmt::LetMulti(l) => {
                let _value_ty = self.check_expr(&l.value, scope, ctx);

                if l.names.len() != 2 {
                    self.errors.push(SemanticError::new(
                        "let multi deve ter exatamente 2 variáveis (valor, err)",
                        Span { line: 0, col: 0 },
                    ));
                }

                // Segunda variável do multi-retorno é sempre error
                for (i, name) in l.names.iter().enumerate() {
                    let ty = if i == 1 {
                        TypeInfo::Error
                    } else {
                        TypeInfo::Unknown
                    };
                    scope.declare_or_shadow(name, Symbol::Variable(ty));
                }

                vec![]
            }
            Stmt::If(i) => {
                let cond_ty = self.check_expr(&i.condition, scope, ctx);
                if !matches!(cond_ty, TypeInfo::Bool) {
                    self.errors.push(SemanticError::new(
                        format!(
                            "condição do if deve ser bool, encontrado {}",
                            cond_ty.name()
                        ),
                        Span { line: 0, col: 0 },
                    ));
                }

                let mut then_scope = scope.child();
                let then_returns = self.check_block(&i.then_block, &mut then_scope, ctx);
                let else_returns = if let Some(else_block) = &i.else_block {
                    let mut else_scope = scope.child();
                    self.check_block(else_block, &mut else_scope, ctx)
                } else {
                    vec![]
                };

                let mut all = then_returns;
                all.extend(else_returns);
                all
            }
            Stmt::Expr(e) => {
                self.check_expr(e, scope, ctx);
                vec![]
            }
            Stmt::TryLet(t) => {
                // let x = expr?  — verifica que é chamada de função
                let _ = self.check_expr(&t.call, scope, ctx);

                // Declara a variável com tipo Unknown (vem do retorno da função)
                scope.declare_or_shadow(&t.name, Symbol::Variable(TypeInfo::Unknown));

                // Verifica que o status code é válido
                if let Some(code) = t.status_code {
                    if code < 100 || code > 599 {
                        self.errors.push(SemanticError::new(
                            format!("status code HTTP inválido: {}", code),
                            Span { line: 0, col: 0 },
                        ));
                    }
                }

                vec![]
            }
        }
    }

    // ---- Expressões ----

    fn check_expr(&mut self, expr: &Expr, scope: &Scope, ctx: Ctx) -> TypeInfo {
        match expr {
            Expr::Lit(lit) => TypeInfo::from_lit(lit),
            Expr::Nil => TypeInfo::Error,
            Expr::Ident(name) => self.check_ident(name, scope, ctx),
            Expr::Call(call) => self.check_call(call, scope, ctx),
            Expr::FieldAccess(obj, field) => self.check_field_access(obj, field, scope, ctx),
            Expr::Index(obj, idx) => self.check_index(obj, idx, scope, ctx),
            Expr::BinOp(l, op, r) => self.check_binop(l, op, r, scope, ctx),
            Expr::Unary(op, e) => self.check_unary(op, e, scope, ctx),
            Expr::MapLit(_) => TypeInfo::Map,
            Expr::StructInit(s) => self.check_struct_init(s, scope, ctx),
        }
    }

    fn check_ident(&mut self, name: &str, scope: &Scope, ctx: Ctx) -> TypeInfo {
        if name == "next" {
            if ctx != Ctx::Middleware {
                self.errors.push(SemanticError::new(
                    "'next' só pode ser usado dentro de um middleware",
                    Span { line: 0, col: 0 },
                ));
            }
            return TypeInfo::Unknown;
        }

        match scope.lookup(name) {
            Some(Symbol::Variable(ty)) => ty.clone(),
            Some(Symbol::Function(_)) => TypeInfo::Unknown,
            Some(Symbol::Struct(_)) => {
                self.errors.push(SemanticError::new(
                    format!("'{}' é um struct e não pode ser usado como expressão", name),
                    Span { line: 0, col: 0 },
                ));
                TypeInfo::Unknown
            }
            Some(Symbol::Middleware) => TypeInfo::Unknown,
            Some(Symbol::Module) => TypeInfo::Unknown,
            None => {
                if name == "req" && ctx == Ctx::Route {
                    return TypeInfo::Map;
                }
                self.errors.push(SemanticError::new(
                    format!("'{}' não foi declarado neste escopo", name),
                    Span { line: 0, col: 0 },
                ));
                TypeInfo::Unknown
            }
        }
    }

    fn check_call(&mut self, call: &CallExpr, scope: &Scope, ctx: Ctx) -> TypeInfo {
        if let Expr::Ident(name) = call.callee.as_ref() {
            if name == "next" {
                if ctx != Ctx::Middleware {
                    self.errors.push(SemanticError::new(
                        "'next()' só pode ser usado dentro de um middleware",
                        Span { line: 0, col: 0 },
                    ));
                }
                return TypeInfo::Unknown;
            }
        }

        // Verifica argumentos primeiro
        for arg in &call.args {
            self.check_expr(arg, scope, ctx);
        }

        // Se for chamada via módulo (alias.metodo), verifica alias
        if let Expr::FieldAccess(receiver, _method) = call.callee.as_ref() {
            if let Expr::Ident(alias) = receiver.as_ref() {
                match scope.lookup(alias) {
                    None | Some(Symbol::Module) => {}
                    Some(_) => {
                        self.errors.push(SemanticError::new(
                            format!("'{}' não é um módulo", alias),
                            Span { line: 0, col: 0 },
                        ));
                    }
                }
            }
            return TypeInfo::Unknown;
        }

        self.infer_call_return(call, scope)
            .unwrap_or(TypeInfo::Unknown)
    }

    fn infer_call_return(&mut self, call: &CallExpr, scope: &Scope) -> Option<TypeInfo> {
        let fn_name = match call.callee.as_ref() {
            Expr::Ident(name) => name.clone(),
            _ => return None,
        };

        match fn_name.as_str() {
            "json" | "text" | "status" | "set_ctx" => return Some(TypeInfo::Unknown),
            _ => {}
        }

        match scope.lookup(&fn_name) {
            Some(Symbol::Function(sig)) => {
                if call.args.len() != sig.params.len() {
                    self.errors.push(SemanticError::new(
                        format!(
                            "'{}' espera {} argumento(s), recebeu {}",
                            fn_name,
                            sig.params.len(),
                            call.args.len()
                        ),
                        Span { line: 0, col: 0 },
                    ));
                }

                for (i, arg) in call.args.iter().enumerate() {
                    if i < sig.params.len() {
                        let arg_ty = self.check_expr(arg, scope, Ctx::Fn);
                        self.check_type_compat(
                            &sig.params[i].1,
                            &arg_ty,
                            &Span { line: 0, col: 0 },
                        );
                    }
                }

                if sig.return_types.len() == 1 {
                    Some(sig.return_types[0].clone())
                } else {
                    Some(TypeInfo::Unknown)
                }
            }
            Some(_) => {
                self.errors.push(SemanticError::new(
                    format!("'{}' não é uma função", fn_name),
                    Span { line: 0, col: 0 },
                ));
                None
            }
            None => {
                self.errors.push(SemanticError::new(
                    format!("função '{}' não definida", fn_name),
                    Span { line: 0, col: 0 },
                ));
                None
            }
        }
    }

    fn check_field_access(&mut self, obj: &Expr, field: &str, scope: &Scope, ctx: Ctx) -> TypeInfo {
        // req.params.campo (só válido dentro de rotas)
        if let Expr::FieldAccess(inner, sub) = obj {
            if let Expr::Ident(name) = inner.as_ref() {
                if name == "req" && sub == "params" {
                    if ctx != Ctx::Route {
                        self.errors.push(SemanticError::new(
                            "'req' só está disponível dentro de rotas",
                            Span { line: 0, col: 0 },
                        ));
                    }
                    return TypeInfo::String;
                }
            }
        }

        if let Expr::Ident(name) = obj {
            if name == "req" {
                if ctx != Ctx::Route {
                    self.errors.push(SemanticError::new(
                        "'req' só está disponível dentro de rotas",
                        Span { line: 0, col: 0 },
                    ));
                }
                return TypeInfo::Unknown;
            }

            if matches!(scope.lookup(name), Some(Symbol::Module)) {
                return TypeInfo::Unknown;
            }
        }

        // err.message → string (permitido em Error e Unknown)
        if field == "message" {
            let obj_ty = self.check_expr(obj, scope, ctx);
            if !matches!(obj_ty, TypeInfo::Error | TypeInfo::Unknown) {
                self.errors.push(SemanticError::new(
                    format!(
                        ".message só é válido em valores do tipo error, encontrado '{}'",
                        obj_ty.name()
                    ),
                    Span { line: 0, col: 0 },
                ));
            }
            return TypeInfo::String;
        }

        // Acesso a campo de struct
        let obj_ty = self.check_expr(obj, scope, ctx);
        if let TypeInfo::Struct(struct_name) = &obj_ty {
            if let Some(Symbol::Struct(fields)) = scope.lookup(struct_name) {
                for sf in fields {
                    if sf.name == field {
                        return TypeInfo::from_ast(&sf.ty);
                    }
                }
                self.errors.push(SemanticError::new(
                    format!("struct '{}' não tem campo '{}'", struct_name, field),
                    Span { line: 0, col: 0 },
                ));
            }
        }

        TypeInfo::Unknown
    }

    fn check_index(&mut self, obj: &Expr, idx: &Expr, scope: &Scope, ctx: Ctx) -> TypeInfo {
        // req.headers["X"], req.query["X"], req.body["X"]
        if let Expr::FieldAccess(inner, field) = obj {
            if let Expr::Ident(name) = inner.as_ref() {
                if name == "req" {
                    self.check_expr(idx, scope, ctx);
                    return match field.as_str() {
                        "headers" | "query" | "body" => TypeInfo::String,
                        _ => TypeInfo::Unknown,
                    };
                }
            }
        }

        let obj_ty = self.check_expr(obj, scope, ctx);
        self.check_expr(idx, scope, ctx);

        if let TypeInfo::List(inner) = &obj_ty {
            return *inner.clone();
        }
        if matches!(obj_ty, TypeInfo::Map) {
            return TypeInfo::Unknown;
        }

        TypeInfo::Unknown
    }

    fn check_binop(
        &mut self,
        left: &Expr,
        op: &BinOp,
        right: &Expr,
        scope: &Scope,
        ctx: Ctx,
    ) -> TypeInfo {
        let left_ty = self.check_expr(left, scope, ctx);
        let right_ty = self.check_expr(right, scope, ctx);

        match op {
            BinOp::And | BinOp::Or => {
                self.check_type_bool(&left_ty, "&&/||");
                self.check_type_bool(&right_ty, "&&/||");
                TypeInfo::Bool
            }
            BinOp::Eq | BinOp::NotEq => {
                self.check_types_compat(&left_ty, &right_ty);
                TypeInfo::Bool
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                self.check_type_numeric(&left_ty, "comparação");
                self.check_type_numeric(&right_ty, "comparação");
                TypeInfo::Bool
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // String + String é concatenação
                if matches!(op, BinOp::Add)
                    && matches!(&left_ty, TypeInfo::String)
                    && matches!(&right_ty, TypeInfo::String)
                {
                    return TypeInfo::String;
                }
                self.check_type_numeric(&left_ty, "aritmético");
                self.check_type_numeric(&right_ty, "aritmético");

                if matches!(left_ty, TypeInfo::Float) || matches!(right_ty, TypeInfo::Float) {
                    TypeInfo::Float
                } else {
                    TypeInfo::Int
                }
            }
        }
    }

    fn check_unary(&mut self, op: &UnaryOp, expr: &Expr, scope: &Scope, ctx: Ctx) -> TypeInfo {
        let inner_ty = self.check_expr(expr, scope, ctx);

        match op {
            UnaryOp::Not => {
                if !matches!(inner_ty, TypeInfo::Bool | TypeInfo::Unknown) {
                    self.errors.push(SemanticError::new(
                        "! só pode ser usado em expressões bool",
                        Span { line: 0, col: 0 },
                    ));
                }
                TypeInfo::Bool
            }
            UnaryOp::Neg => {
                if !inner_ty.is_numeric() {
                    self.errors.push(SemanticError::new(
                        "- só pode ser usado em expressões numéricas",
                        Span { line: 0, col: 0 },
                    ));
                }
                inner_ty
            }
        }
    }

    fn check_struct_init(&mut self, s: &StructInit, scope: &Scope, ctx: Ctx) -> TypeInfo {
        match scope.lookup(&s.name) {
            Some(Symbol::Struct(fields)) => {
                for (field_name, field_expr) in &s.fields {
                    let field_ty = self.check_expr(field_expr, scope, ctx);
                    let found = fields.iter().find(|f| f.name == *field_name);
                    match found {
                        Some(sf) => {
                            let expected = TypeInfo::from_ast(&sf.ty);
                            self.check_type_compat(&expected, &field_ty, &Span { line: 0, col: 0 });
                        }
                        None => {
                            self.errors.push(SemanticError::new(
                                format!("struct '{}' não tem campo '{}'", s.name, field_name),
                                Span { line: 0, col: 0 },
                            ));
                        }
                    }
                }

                for sf in fields {
                    if !s.fields.iter().any(|(n, _)| n == &sf.name) {
                        self.errors.push(SemanticError::new(
                            format!(
                                "struct '{}' requer campo '{}' na inicialização",
                                s.name, sf.name
                            ),
                            Span { line: 0, col: 0 },
                        ));
                    }
                }

                TypeInfo::Struct(s.name.clone())
            }
            Some(_) => {
                self.errors.push(SemanticError::new(
                    format!("'{}' não é um struct", s.name),
                    Span { line: 0, col: 0 },
                ));
                TypeInfo::Unknown
            }
            None => {
                self.errors.push(SemanticError::new(
                    format!("struct '{}' não definido", s.name),
                    Span { line: 0, col: 0 },
                ));
                TypeInfo::Unknown
            }
        }
    }

    // ---- Helpers de tipo ----

    fn check_type_compat(&mut self, expected: &TypeInfo, got: &TypeInfo, span: &Span) {
        if matches!(got, TypeInfo::Unknown) {
            return;
        }
        if expected != got {
            self.errors.push(SemanticError::new(
                format!(
                    "esperado tipo '{}', encontrado '{}'",
                    expected.name(),
                    got.name()
                ),
                span.clone(),
            ));
        }
    }

    fn check_types_compat(&mut self, a: &TypeInfo, b: &TypeInfo) {
        if matches!(a, TypeInfo::Unknown) || matches!(b, TypeInfo::Unknown) {
            return;
        }
        if a != b {
            self.errors.push(SemanticError::new(
                format!("tipos incompatíveis: '{}' e '{}'", a.name(), b.name()),
                Span { line: 0, col: 0 },
            ));
        }
    }

    fn check_type_bool(&mut self, ty: &TypeInfo, op: &str) {
        if matches!(ty, TypeInfo::Unknown) {
            return;
        }
        if !matches!(ty, TypeInfo::Bool) {
            self.errors.push(SemanticError::new(
                format!(
                    "operador {} requer tipo bool, encontrado '{}'",
                    op,
                    ty.name()
                ),
                Span { line: 0, col: 0 },
            ));
        }
    }

    fn check_type_numeric(&mut self, ty: &TypeInfo, op: &str) {
        if matches!(ty, TypeInfo::Unknown) {
            return;
        }
        if !ty.is_numeric() {
            self.errors.push(SemanticError::new(
                format!(
                    "operador {} requer tipo numérico, encontrado '{}'",
                    op,
                    ty.name()
                ),
                Span { line: 0, col: 0 },
            ));
        }
    }
}

// ---- Helpers ----

fn return_types_from_ast(rt: &ReturnType) -> Vec<TypeInfo> {
    match rt {
        ReturnType::None => vec![],
        ReturnType::Single(t) => vec![TypeInfo::from_ast(t)],
        ReturnType::Tuple(types) => types.iter().map(|t| TypeInfo::from_ast(t)).collect(),
    }
}
