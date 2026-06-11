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
        for builtin in &["json", "text", "status"] {
            let _ = global.declare(
                builtin,
                Symbol::Function(FnSignature {
                    params: vec![],
                    return_types: vec![],
                }),
                &Span { line: 0, col: 0 },
            );
        }
        Self { global, errors: Vec::new() }
    }

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
                        params: f.params
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
                    if let Err(e) = self.global.declare(&s.name, Symbol::Struct(s.fields.clone()), &s.span) {
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
                _ => {}
            }
        }
    }

    // ---- Verificação de corpos ----

    fn check_fn(&mut self, f: &FnDef) {
        let mut scope = self.global.child();

        for p in &f.params {
            let ty = TypeInfo::from_ast(&p.ty);
            if let Err(e) = scope.declare(&p.name, Symbol::Variable(ty), &Span { line: 0, col: 0 }) {
                self.errors.push(e);
            }
        }

        let declared_returns = return_types_from_ast(&f.return_type);
        let inferred_returns = self.check_block(&f.body, &mut scope, Ctx::Fn);

        if !declared_returns.is_empty() {
            for inferred_group in &inferred_returns {
                for (i, inferred) in inferred_group.iter().enumerate() {
                    if i >= declared_returns.len() {
                        self.errors.push(SemanticError::new(
                            format!("retorno tem mais valores que o declarado ({})", declared_returns.len()),
                            f.span.clone(),
                        ));
                        break;
                    }
                    self.check_type_compat(&declared_returns[i], inferred, &f.span);
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
                let types: Vec<TypeInfo> = exprs.iter().map(|e| self.check_expr(e, scope, ctx)).collect();
                if ctx == Ctx::Route && types.len() > 1 {
                    self.errors.push(SemanticError::new(
                        "return em rota deve ter no máximo 1 expressão",
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
                    if let Err(e) = scope.declare(&l.name, Symbol::Variable(annotated_ty), &Span { line: 0, col: 0 }) {
                        self.errors.push(e);
                    }
                } else {
                    if let Err(e) = scope.declare(&l.name, Symbol::Variable(value_ty), &Span { line: 0, col: 0 }) {
                        self.errors.push(e);
                    }
                }
                vec![]
            }
            Stmt::LetMulti(l) => {
                self.check_expr(&l.value, scope, ctx);

                if l.names.len() != 2 {
                    self.errors.push(SemanticError::new(
                        "let multi deve ter exatamente 2 variáveis (valor, err)",
                        Span { line: 0, col: 0 },
                    ));
                }

                for name in &l.names {
                    let _ = scope.declare(name, Symbol::Variable(TypeInfo::Unknown), &Span { line: 0, col: 0 });
                }

                if let Expr::Call(call) = &l.value {
                    let ret_types = self.infer_call_return_types(call, scope);
                    for (i, name) in l.names.iter().enumerate() {
                        if let Some(types) = &ret_types {
                            if i < types.len() {
                                let _ = scope.declare(name, Symbol::Variable(types[i].clone()), &Span { line: 0, col: 0 });
                            }
                        }
                    }
                }

                vec![]
            }
            Stmt::If(i) => {
                let cond_ty = self.check_expr(&i.condition, scope, ctx);
                if !matches!(cond_ty, TypeInfo::Bool) {
                    self.errors.push(SemanticError::new(
                        format!("condição do if deve ser bool, encontrado {}", cond_ty.name()),
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

                let mut all_returns = then_returns;
                all_returns.extend(else_returns);
                all_returns
            }
            Stmt::Expr(e) => {
                self.check_expr(e, scope, ctx);
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

        for arg in &call.args {
            self.check_expr(arg, scope, ctx);
        }

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
        }

        self.infer_call_return(call, scope).unwrap_or(TypeInfo::Unknown)
    }

    fn infer_call_return(&mut self, call: &CallExpr, scope: &Scope) -> Option<TypeInfo> {
        let fn_name = match call.callee.as_ref() {
            Expr::Ident(name) => name.clone(),
            Expr::FieldAccess(_, method) => method.clone(),
            _ => return None,
        };

        match fn_name.as_str() {
            "json" | "text" | "status" => return Some(TypeInfo::Unknown),
            _ => {}
        }

        if matches!(call.callee.as_ref(), Expr::FieldAccess(..)) {
            return Some(TypeInfo::Unknown);
        }

        match scope.lookup(&fn_name) {
            Some(Symbol::Function(sig)) => {
                if call.args.len() != sig.params.len() {
                    self.errors.push(SemanticError::new(
                        format!("'{}' espera {} argumento(s), recebeu {}", fn_name, sig.params.len(), call.args.len()),
                        Span { line: 0, col: 0 },
                    ));
                }

                for (i, arg) in call.args.iter().enumerate() {
                    if i < sig.params.len() {
                        let arg_ty = self.check_expr(arg, scope, Ctx::Fn);
                        self.check_type_compat(&sig.params[i].1, &arg_ty, &Span { line: 0, col: 0 });
                    }
                }

                if sig.return_types.is_empty() {
                    Some(TypeInfo::Unknown)
                } else if sig.return_types.len() == 1 {
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

    /// Versão que retorna a lista de tipos para