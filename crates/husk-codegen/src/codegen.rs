use husk_parser::ast::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
}

impl CodegenError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Fn,
    Route,
    Middleware,
}

pub struct Codegen {
    go_imports: BTreeSet<String>,
    /// alias de módulos do usuário: `alias.fn()` → `fn()`
    user_aliases: HashSet<String>,
    /// alias de módulos stdlib: `alias.fn()` → `alias_fn()`
    stdlib_aliases: HashSet<String>,
    /// Contador para nomes únicos de variáveis _huskCtx
    ctx_var_counter: Cell<usize>,
    /// Contador para nomes únicos de variáveis __tryN
    try_var_counter: Cell<usize>,
    /// Preâmbulos pendentes gerados por expr? dentro de expressões
    pending_try: RefCell<Vec<String>>,
    /// Registro de parâmetros de funções: nome → [(param_name, type)]
    fn_params: HashMap<String, Vec<(String, Type)>>,
    /// Path da rota atual (para typed params via :id<int>)
    current_route_path: RefCell<Option<RoutePath>>,
    /// Nome da variável de contexto atual (definido via -> ctx)
    current_ctx_name: RefCell<Option<String>>,
    /// Nome do middleware atual (para namespacing do ctx)
    current_middleware_name: RefCell<Option<String>>,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            go_imports: BTreeSet::new(),
            user_aliases: HashSet::new(),
            stdlib_aliases: HashSet::new(),
            ctx_var_counter: Cell::new(0),
            try_var_counter: Cell::new(0),
            pending_try: RefCell::new(Vec::new()),
            fn_params: HashMap::new(),
            current_route_path: RefCell::new(None),
            current_ctx_name: RefCell::new(None),
            current_middleware_name: RefCell::new(None),
        }
    }

    pub fn generate(&mut self, program: &Program) -> Result<String, CodegenError> {
        self.ctx_var_counter.set(0);
        self.try_var_counter.set(0);

        // Registra parâmetros de todas as funções (para spread body...)
        for item in &program.items {
            if let Item::FnDef(f) = item {
                let params: Vec<(String, Type)> = f
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect();
                self.fn_params.insert(f.name.clone(), params);
            }
        }

        for item in &program.items {
            if let Item::Import(imp) = item {
                if imp.is_stdlib {
                    self.stdlib_aliases.insert(imp.alias.clone());
                } else {
                    self.user_aliases.insert(imp.alias.clone());
                }
            }
        }

        self.collect_go_imports(program);

        let mut body = String::new();

        for item in &program.items {
            match item {
                Item::StructDef(s) => body.push_str(&self.gen_struct_def(s)?),
                Item::FnDef(f) => body.push_str(&self.gen_fn(f)?),
                Item::MiddlewareDef(m) => body.push_str(&self.gen_middleware(m)?),
                Item::RouteDef(_) => {}
                Item::Import(_) => {}
            }
        }

        body.push_str(&self.gen_main(program)?);

        let mut file = String::new();
        file.push_str("package main\n\n");
        file.push_str(&self.gen_imports());
        file.push_str(&body);

        Ok(file)
    }

    // --- coleta de imports Go ---

    fn collect_go_imports(&mut self, program: &Program) {
        self.go_imports.insert("fmt".into());
        self.go_imports.insert("log".into());
        self.go_imports.insert("net/http".into());

        let has_routes = program.items.iter().any(|i| matches!(i, Item::RouteDef(_)));
        if has_routes {
            self.go_imports.insert("github.com/go-chi/chi/v5".into());
        }

        for item in &program.items {
            match item {
                Item::FnDef(f) => self.scan_block_imports(&f.body, Ctx::Fn),
                Item::RouteDef(r) => {
                    if block_uses_body(&r.body) {
                        self.go_imports.insert("encoding/json".into());
                    }
                    // Verifica se a rota tem parâmetros tipados (ex: :id<int>)
                    for seg in &r.path.segments {
                        if let PathSegment::Param(_name, Some(ty)) = seg {
                            match ty {
                                Type::Int | Type::Float => {
                                    self.go_imports.insert("strconv".into());
                                }
                                _ => {}
                            }
                        }
                    }
                    self.scan_block_imports(&r.body, Ctx::Route);
                }
                Item::MiddlewareDef(m) => self.scan_block_imports(&m.body, Ctx::Middleware),
                _ => {}
            }
        }

        // .env loading usa os e strings
        let has_routes = program.items.iter().any(|i| matches!(i, Item::RouteDef(_)));
        if has_routes {
            self.go_imports.insert("os".into());
            self.go_imports.insert("strings".into());
        }
    }

    fn scan_block_imports(&mut self, block: &Block, ctx: Ctx) {
        for stmt in &block.stmts {
            self.scan_stmt_imports(stmt, ctx);
        }
    }

    fn scan_stmt_imports(&mut self, stmt: &Stmt, ctx: Ctx) {
        match stmt {
            Stmt::Return(exprs) => {
                if ctx == Ctx::Route {
                    if let Some(e) = exprs.first() {
                        self.scan_route_return_imports(e);
                    }
                } else {
                    for e in exprs {
                        self.scan_expr_imports(e);
                    }
                }
            }
            Stmt::Expr(e) => self.scan_expr_imports(e),
            Stmt::Let(l) => self.scan_expr_imports(&l.value),
            Stmt::LetMulti(l) => self.scan_expr_imports(&l.value),
            Stmt::TryLet(t) => {
                // TryLet usa json e encoding/json
                self.go_imports.insert("encoding/json".into());
                self.scan_expr_imports(&t.call);
            }
            Stmt::If(i) => {
                self.scan_block_imports(&i.then_block, ctx);
                if let Some(eb) = &i.else_block {
                    self.scan_block_imports(eb, ctx);
                }
            }
            Stmt::Assign(a) => {
                self.scan_expr_imports(&a.target);
                self.scan_expr_imports(&a.value);
                // ctx.field = value usa context.WithValue
                if let Expr::FieldAccess(obj, _field) = a.target.as_ref() {
                    if let Expr::Ident(_name) = obj.as_ref() {
                        self.go_imports.insert("context".into());
                    }
                }
            }
        }
    }

    fn scan_route_return_imports(&mut self, expr: &Expr) {
        match expr {
            Expr::Call(call) if is_builtin(&call.callee, "json") => {
                self.go_imports.insert("encoding/json".into());
                for arg in &call.args {
                    self.scan_expr_imports(arg);
                }
            }
            Expr::Call(call) if is_builtin(&call.callee, "status") => {
                self.go_imports.insert("encoding/json".into());
                if let Some(body) = call.args.get(1) {
                    self.scan_route_return_imports(body);
                }
            }
            other => {
                self.scan_expr_imports(other);
            }
        }
    }

    fn scan_expr_imports(&mut self, expr: &Expr) {
        match expr {
            Expr::Call(call) => {
                if is_builtin(&call.callee, "set_ctx") {
                    self.go_imports.insert("context".into());
                }
                if is_builtin(&call.callee, "parse_int") {
                    self.go_imports.insert("strconv".into());
                }
                if is_builtin(&call.callee, "require_role") {
                    self.go_imports.insert("encoding/json".into());
                }
                if is_builtin(&call.callee, "require_field") {
                    self.go_imports.insert("encoding/json".into());
                }
                // String built-ins
                if matches!(
                    call.callee.as_ref(),
                    Expr::Ident(n)
                        if matches!(n.as_str(),
                            "contains" | "starts_with" | "replace"
                            | "split" | "trim" | "upper" | "lower")
                ) {
                    self.go_imports.insert("strings".into());
                }
                // Math built-ins
                if matches!(
                    call.callee.as_ref(),
                    Expr::Ident(n) if matches!(n.as_str(), "abs" | "sqrt")
                ) {
                    self.go_imports.insert("math".into());
                }
                for arg in &call.args {
                    self.scan_expr_imports(arg);
                }
            }
            // req.ctx["X"] — leitura do contexto
            Expr::Index(obj, _) => {
                if let Expr::FieldAccess(inner, field) = obj.as_ref() {
                    if let Expr::Ident(name) = inner.as_ref() {
                        if name == "req" && field == "ctx" {
                            self.go_imports.insert("context".into());
                        }
                    }
                }
                self.scan_expr_imports(obj);
            }
            Expr::BinOp(l, _, r) => {
                self.scan_expr_imports(l);
                self.scan_expr_imports(r);
            }
            Expr::Unary(_, e) | Expr::FieldAccess(e, _) => self.scan_expr_imports(e),
            Expr::Try(t) => {
                self.scan_expr_imports(&t.expr);
                self.go_imports.insert("encoding/json".into());
            }
            Expr::Spread(e) => self.scan_expr_imports(e),
            _ => {}
        }
    }

    fn gen_imports(&self) -> String {
        if self.go_imports.is_empty() {
            return String::new();
        }
        let mut s = String::from("import (\n");
        let std_pkgs: Vec<_> = self
            .go_imports
            .iter()
            .filter(|p| !p.contains('.'))
            .collect();
        let ext_pkgs: Vec<_> = self.go_imports.iter().filter(|p| p.contains('.')).collect();
        for pkg in &std_pkgs {
            s.push_str(&format!("\t\"{}\"\n", pkg));
        }
        if !std_pkgs.is_empty() && !ext_pkgs.is_empty() {
            s.push('\n');
        }
        for pkg in &ext_pkgs {
            s.push_str(&format!("\t\"{}\"\n", pkg));
        }
        s.push_str(")\n\n");
        s
    }

    // --- structs ---

    fn gen_struct_def(&self, s: &StructDef) -> Result<String, CodegenError> {
        let mut out = format!("type {} struct {{\n", s.name);
        for field in &s.fields {
            out.push_str(&format!(
                "\t{} {} `json:\"{}\"`\n",
                capitalize(&field.name),
                go_type(&field.ty),
                field.name,
            ));
        }
        out.push_str("}\n\n");
        Ok(out)
    }

    // --- funções ---

    fn gen_fn(&self, f: &FnDef) -> Result<String, CodegenError> {
        let params = f
            .params
            .iter()
            .map(|p| format!("{} {}", p.name, go_type(&p.ty)))
            .collect::<Vec<_>>()
            .join(", ");

        let ret = match &f.return_type {
            ReturnType::None => infer_fn_return(&f.body)
                .map(|t| format!(" {}", t))
                .unwrap_or_default(),
            ReturnType::Single(t) => format!(" {}", go_type(t)),
            ReturnType::Tuple(types) => {
                let ts = types
                    .iter()
                    .map(|t| go_type(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" ({ts})")
            }
        };

        let mut s = format!("func {}({}){} {{\n", f.name, params, ret);
        s.push_str(&self.gen_block(&f.body, Ctx::Fn, 1)?);
        s.push_str("}\n\n");
        Ok(s)
    }

    // --- middlewares ---

    fn gen_middleware(&self, m: &MiddlewareDef) -> Result<String, CodegenError> {
        *self.current_middleware_name.borrow_mut() = Some(m.name.clone());
        *self.current_ctx_name.borrow_mut() = m.ctx_var.clone();

        let mut s = format!(
            "func {}(next http.Handler) http.Handler {{\n\
             \treturn http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {{\n",
            m.name
        );
        s.push_str(&self.gen_block(&m.body, Ctx::Middleware, 2)?);
        s.push_str("\t})\n}\n\n");

        *self.current_middleware_name.borrow_mut() = None;
        *self.current_ctx_name.borrow_mut() = None;
        Ok(s)
    }

    // --- main + rotas ---

    fn gen_main(&self, program: &Program) -> Result<String, CodegenError> {
        let mut s = String::from("func main() {\n");

        // --- Auto-load .env ---
        s.push_str("\t// Carrega .env automaticamente (procura em .env e backend/.env)\n");
        s.push_str("\tfor _, envPath := range []string{\".env\", \"backend/.env\"} {\n");
        s.push_str("\t\tif data, err := os.ReadFile(envPath); err == nil {\n");
        s.push_str("\t\t\tfor _, line := range strings.Split(string(data), \"\\n\") {\n");
        s.push_str("\t\t\t\tline = strings.TrimSpace(line)\n");
        s.push_str("\t\t\t\tif line == \"\" || strings.HasPrefix(line, \"#\") {\n");
        s.push_str("\t\t\t\t\tcontinue\n");
        s.push_str("\t\t\t\t}\n");
        s.push_str("\t\t\t\tif parts := strings.SplitN(line, \"=\", 2); len(parts) == 2 {\n");
        s.push_str("\t\t\t\t\tkey := strings.TrimSpace(parts[0])\n");
        s.push_str("\t\t\t\t\tval := strings.TrimSpace(parts[1])\n");
        s.push_str("\t\t\t\t\tif os.Getenv(key) == \"\" {\n");
        s.push_str("\t\t\t\t\t\tos.Setenv(key, val)\n");
        s.push_str("\t\t\t\t\t}\n");
        s.push_str("\t\t\t\t}\n");
        s.push_str("\t\t\t}\n");
        s.push_str("\t\t}\n");
        s.push_str("\t}\n\n");

        s.push_str("\tr := chi.NewRouter()\n\n");

        for item in &program.items {
            if let Item::RouteDef(route) = item {
                s.push_str(&self.gen_route_registration(route)?);
            }
        }

        s.push_str("\n\tfmt.Println(\"husk: servidor rodando em http://localhost:8080\")\n");
        s.push_str("\tlog.Fatal(http.ListenAndServe(\":8080\", r))\n}\n");
        Ok(s)
    }

    fn gen_route_registration(&self, route: &RouteDef) -> Result<String, CodegenError> {
        *self.current_route_path.borrow_mut() = Some(route.path.clone());
        *self.current_ctx_name.borrow_mut() = route.ctx_var.clone();

        let method = match route.method {
            HttpMethod::Get => "Get",
            HttpMethod::Post => "Post",
            HttpMethod::Put => "Put",
            HttpMethod::Patch => "Patch",
            HttpMethod::Delete => "Delete",
        };
        let path = route.path.to_string();

        // r.With(mw1, mw2).Get(...) ou r.Get(...)
        let router = if route.middlewares.is_empty() {
            "r".to_string()
        } else {
            format!("r.With({})", route.middlewares.join(", "))
        };

        let mut s = format!(
            "\t{}.{}(\"{}\", func(w http.ResponseWriter, r *http.Request) {{\n",
            router, method, path
        );

        // Typed params: :id<int> → gera variável pré-parseada
        // (import strconv já foi adicionado na fase de scan)
        let mut has_typed_param = false;
        for seg in &route.path.segments {
            if let PathSegment::Param(name, Some(ty)) = seg {
                has_typed_param = true;
                match ty {
                    Type::Int => {
                        s.push_str(&format!(
                            "\t\t_huskParam_{}, _ := strconv.Atoi(chi.URLParam(r, \"{}\"))\n",
                            name, name
                        ));
                    }
                    Type::Float => {
                        s.push_str(&format!(
                            "\t\t_huskParam_{}, _ := strconv.ParseFloat(chi.URLParam(r, \"{}\"), 64)\n",
                            name, name
                        ));
                    }
                    _ => {
                        s.push_str(&format!(
                            "\t\t_huskParam_{} := chi.URLParam(r, \"{}\")\n",
                            name, name
                        ));
                    }
                }
            }
        }
        if has_typed_param {
            s.push('\n');
        }

        if block_uses_body(&route.body) {
            s.push_str("\t\tvar _huskBody map[string]interface{}\n");
            s.push_str("\t\tjson.NewDecoder(r.Body).Decode(&_huskBody)\n");
        }
        s.push_str(&self.gen_block(&route.body, Ctx::Route, 2)?);
        s.push_str("\t})\n");

        *self.current_route_path.borrow_mut() = None;
        *self.current_ctx_name.borrow_mut() = None;
        Ok(s)
    }

    // --- bloco e statements ---

    fn gen_block(&self, block: &Block, ctx: Ctx, indent: usize) -> Result<String, CodegenError> {
        let mut s = String::new();
        let pad = "\t".repeat(indent);
        let mut declared: HashSet<String> = HashSet::new();

        for stmt in &block.stmts {
            // Emite preâmbulos pendentes de expr? antes do statement
            self.flush_pending_try(&mut s, &pad);

            let generated = self.gen_stmt(stmt, ctx, indent, &mut declared)?;
            for line in generated.lines() {
                s.push_str(&pad);
                s.push_str(line);
                s.push('\n');
            }
        }
        // Garante que não sobrou nada pendente
        self.flush_pending_try(&mut s, &pad);
        Ok(s)
    }

    fn is_db_exec(&self, expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Expr::FieldAccess(receiver, method) = call.callee.as_ref() {
                if let Expr::Ident(alias) = receiver.as_ref() {
                    return self.stdlib_aliases.contains(alias.as_str()) && method == "exec";
                }
            }
        }
        false
    }

    fn gen_stmt(
        &self,
        stmt: &Stmt,
        ctx: Ctx,
        indent: usize,
        declared: &mut HashSet<String>,
    ) -> Result<String, CodegenError> {
        // Captura qualquer preâmbulo pendente gerado pelo statement
        let result = match stmt {
            Stmt::Return(exprs) => self.gen_return(exprs, ctx),
            Stmt::Let(l) => {
                let op = if l.name == "_" || declared.contains(&l.name) {
                    "="
                } else {
                    ":="
                };
                if l.name != "_" {
                    declared.insert(l.name.clone());
                }
                let expr = self.gen_expr(&l.value, ctx)?;
                // db.exec returns (interface{}, error); when assigning to single var,
                // prefix with _, to ignore the sql.Result
                let prefix = if self.is_db_exec(&l.value) { "_, " } else { "" };
                Ok(format!("{}{} {} {}", prefix, l.name, op, expr))
            }
            Stmt::LetMulti(l) => {
                // Go exige ao menos um nome novo para :=; se todos já existem, usa =
                let all_declared = l.names.iter().all(|n| declared.contains(n));
                let op = if all_declared { "=" } else { ":=" };
                for n in &l.names {
                    declared.insert(n.clone());
                }
                Ok(format!(
                    "{} {} {}",
                    l.names.join(", "),
                    op,
                    self.gen_expr(&l.value, ctx)?
                ))
            }
            Stmt::If(i) => self.gen_if(i, ctx, indent),
            Stmt::TryLet(t) => self.gen_try_let(t, ctx),
            Stmt::Assign(a) => self.gen_assign(a, ctx),
            Stmt::Expr(e) => self.gen_expr(e, ctx),
        };

        // Se gerou preâmbulos (expr?), combina com o resultado
        let mut buf = String::new();
        self.flush_pending_try(&mut buf, "");
        let stmt = result?;
        if buf.is_empty() {
            Ok(stmt)
        } else {
            Ok(format!("{}{}", buf, stmt))
        }
    }

    /// Gera código para atribuição: ctx.field = value
    /// A importação de "context" é adicionada na fase de scan (scan_stmt_imports)
    fn gen_assign(&self, a: &AssignStmt, ctx: Ctx) -> Result<String, CodegenError> {
        // ctx.field = value  dentro de middleware com current_ctx_name
        if let Expr::FieldAccess(obj, field) = a.target.as_ref() {
            if let Expr::Ident(var_name) = obj.as_ref() {
                if let Some(ref ctx_name) = *self.current_ctx_name.borrow() {
                    if var_name == ctx_name {
                        let val = self.gen_expr(&a.value, ctx)?;
                        let n = self.ctx_var_counter.get() + 1;
                        self.ctx_var_counter.set(n);
                        let ctx_var = format!("_huskCtx{}", n);
                        // Namespace a chave com o nome da variável de contexto
                        let ctx_key = format!("__husk_ctx_{}", field);
                        return Ok(format!(
                            "{} := context.WithValue(r.Context(), \"{}\", {})\nr = r.WithContext({})",
                            ctx_var, ctx_key, val, ctx_var
                        ));
                    }
                }
            }
        }
        // Fallback: geração genérica
        let target = self.gen_expr(&a.target, ctx)?;
        let val = self.gen_expr(&a.value, ctx)?;
        Ok(format!("{} = {}", target, val))
    }

    fn gen_return(&self, exprs: &[Expr], ctx: Ctx) -> Result<String, CodegenError> {
        match ctx {
            Ctx::Fn => {
                let vals = exprs
                    .iter()
                    .map(|e| self.gen_expr(e, ctx))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("return {}", vals))
            }
            // Middleware e Route têm acesso a w/r e devem escrever resposta HTTP
            Ctx::Route | Ctx::Middleware => {
                let response = self.gen_route_response(&exprs[0])?;
                Ok(format!("{}\nreturn", response))
            }
        }
    }

    fn gen_route_response(&self, expr: &Expr) -> Result<String, CodegenError> {
        match expr {
            Expr::Call(call) if is_builtin(&call.callee, "json") => {
                let arg = call
                    .args
                    .first()
                    .ok_or_else(|| CodegenError::new("json() requer um argumento"))?;
                Ok(format!(
                    "w.Header().Set(\"Content-Type\", \"application/json\")\njson.NewEncoder(w).Encode({})",
                    self.gen_expr(arg, Ctx::Route)?
                ))
            }
            Expr::Call(call) if is_builtin(&call.callee, "text") => {
                let arg = call
                    .args
                    .first()
                    .ok_or_else(|| CodegenError::new("text() requer um argumento"))?;
                Ok(format!(
                    "fmt.Fprint(w, {})",
                    self.gen_expr(arg, Ctx::Route)?
                ))
            }
            Expr::Call(call) if is_builtin(&call.callee, "status") => {
                let code = call
                    .args
                    .first()
                    .ok_or_else(|| CodegenError::new("status() requer o código HTTP"))?;
                let mut s = format!("w.WriteHeader({})", self.gen_expr(code, Ctx::Route)?);
                if let Some(body) = call.args.get(1) {
                    s.push('\n');
                    s.push_str(&self.gen_route_response(body)?);
                }
                Ok(s)
            }
            Expr::MapLit(m) => {
                // Objeto literal em resposta de rota → JSON automático
                Ok(format!(
                    "w.Header().Set(\"Content-Type\", \"application/json\")\njson.NewEncoder(w).Encode({})",
                    self.gen_map_lit(m, Ctx::Route)?
                ))
            }
            other => {
                // Qualquer expressão em rota é serializada como JSON automaticamente
                // Use text() para texto puro
                Ok(format!(
                    "w.Header().Set(\"Content-Type\", \"application/json\")\njson.NewEncoder(w).Encode({})",
                    self.gen_expr(other, Ctx::Route)?
                ))
            }
        }
    }

    fn gen_if(&self, i: &IfStmt, ctx: Ctx, indent: usize) -> Result<String, CodegenError> {
        let inner_pad = "\t".repeat(indent + 1);
        let outer_pad = "\t".repeat(indent);

        let cond = self.gen_expr(&i.condition, ctx)?;
        let then_body = self.gen_block(&i.then_block, ctx, indent + 1)?;

        // remove o pad externo já que gen_stmt vai adicioná-lo
        let then_body = then_body
            .lines()
            .map(|l| l.strip_prefix(&inner_pad).unwrap_or(l))
            .collect::<Vec<_>>()
            .join(&format!("\n{}", inner_pad));

        let mut s = format!(
            "if {} {{\n{}{}\n{}}}",
            cond, inner_pad, then_body, outer_pad
        );

        if let Some(else_block) = &i.else_block {
            let else_body = self.gen_block(else_block, ctx, indent + 1)?;
            let else_body = else_body
                .lines()
                .map(|l| l.strip_prefix(&inner_pad).unwrap_or(l))
                .collect::<Vec<_>>()
                .join(&format!("\n{}", inner_pad));
            s.push_str(&format!(
                " else {{\n{}{}\n{}}}",
                inner_pad, else_body, outer_pad
            ));
        }

        Ok(s)
    }

    /// let x = expr? [status] ["msg"]  — try: propaga erro como resposta HTTP
    fn gen_try_let(&self, t: &TryLetStmt, ctx: Ctx) -> Result<String, CodegenError> {
        let call_go = self.gen_expr(&t.call, ctx)?;
        let err_var = "__try_err";

        let msg = if let Some(msg) = &t.message {
            format!("\"{}\"", msg.replace('"', "\\\"").replace('\n', "\\n"))
        } else {
            format!("{}.Error()", err_var)
        };

        let code = t.status_code.unwrap_or(500);

        let mut s = format!(
            "{}, {} := {}\nif {} != nil {{",
            t.name, err_var, call_go, err_var
        );
        s.push_str(&format!("\n\tw.WriteHeader({})", code));
        s.push_str("\n\tw.Header().Set(\"Content-Type\", \"application/json\")");
        s.push_str(&format!(
            "\n\tjson.NewEncoder(w).Encode(map[string]interface{{}}{{\
\
             \t\t\"erro\": {},\
\
             \t}})",
            msg
        ));
        s.push_str("\n\treturn\n}\n");
        Ok(s)
    }

    /// Emite preâmbulos pendentes de expr? no buffer de saída
    fn flush_pending_try(&self, s: &mut String, pad: &str) {
        let pending = self.pending_try.borrow_mut().drain(..).collect::<Vec<_>>();
        for preamble in pending {
            for line in preamble.lines() {
                s.push_str(pad);
                s.push_str(line);
                s.push('\n');
            }
        }
    }

    // --- expressões ---

    fn gen_expr(&self, expr: &Expr, ctx: Ctx) -> Result<String, CodegenError> {
        match expr {
            Expr::Lit(lit) => Ok(gen_lit(lit)),
            Expr::Nil => Ok("nil".into()),
            Expr::Ident(name) => Ok(name.clone()),
            Expr::Call(call) => self.gen_call(call, ctx),
            Expr::FieldAccess(obj, f) => self.gen_field_access(obj, f, ctx),
            Expr::Index(obj, idx) => self.gen_index(obj, idx, ctx),
            Expr::BinOp(l, op, r) => Ok(format!(
                "{} {} {}",
                self.gen_expr(l, ctx)?,
                go_binop(op),
                self.gen_expr(r, ctx)?
            )),
            Expr::Unary(op, e) => {
                let go_op = match op {
                    UnaryOp::Not => "!",
                    UnaryOp::Neg => "-",
                };
                Ok(format!("{}{}", go_op, self.gen_expr(e, ctx)?))
            }
            Expr::MapLit(m) => self.gen_map_lit(m, ctx),
            Expr::StructInit(s) => self.gen_struct_init(s, ctx),
            Expr::Try(t) => self.gen_try_expr(t, ctx),
            Expr::Spread(e) => self.gen_expr(e, ctx),
        }
    }

    /// expr?  — gera preâmbulo e retorna nome da variável temporária
    fn gen_try_expr(&self, t: &TryExpr, ctx: Ctx) -> Result<String, CodegenError> {
        let n = self.try_var_counter.get() + 1;
        self.try_var_counter.set(n);
        let val_var = format!("__try{}_val", n);
        let err_var = format!("__try{}_err", n);

        let call_go = self.gen_expr(&t.expr, ctx)?;
        let code = t.status_code.unwrap_or(500);
        let msg = if let Some(msg) = &t.message {
            format!("\"{}\"", msg.replace('"', "\\\"").replace('\n', "\\n"))
        } else {
            format!("{}.Error()", err_var)
        };

        let preamble = format!(
            "{}, {} := {}\n\
             if {} != nil {{\n\
             \tw.WriteHeader({})\n\
             \tw.Header().Set(\"Content-Type\", \"application/json\")\n\
             \tjson.NewEncoder(w).Encode(map[string]interface{{}}{{\
\
             \t\t\"erro\": {},\
\
             \t}})\n\
             \treturn\n\
             }}",
            val_var, err_var, call_go, err_var, code, msg
        );

        self.pending_try.borrow_mut().push(preamble);
        Ok(val_var)
    }

    fn gen_field_access(&self, obj: &Expr, field: &str, ctx: Ctx) -> Result<String, CodegenError> {
        // req.params.campo → chi.URLParam(r, "campo") ou _huskParam_campo (se tipado)
        if let Expr::FieldAccess(inner, sub) = obj {
            if let Expr::Ident(name) = inner.as_ref() {
                if name == "req" && sub == "params" {
                    // Verifica se o parâmetro tem tipo anotado (ex: :id<int>)
                    if let Some(ref path) = *self.current_route_path.borrow() {
                        if let Some(_ty) = path.param_type(field) {
                            return Ok(format!("_huskParam_{}", field));
                        }
                    }
                    return Ok(format!("chi.URLParam(r, \"{}\")", field));
                }
            }
        }

        // ctx_var.field  — acesso ao contexto tipado (quando -> ctx está ativo)
        // Dentro de middleware ou rota que declarou -> ctx_var
        if let Expr::Ident(name) = obj {
            if let Some(ref ctx_name) = *self.current_ctx_name.borrow() {
                if name == ctx_name {
                    let ctx_key = format!("__husk_ctx_{}", field);
                    return Ok(format!("r.Context().Value(\"{}\").(string)", ctx_key));
                }
            }
        }

        // req.headers → tratado em gen_index
        // req.body → o JSON decodificado (_huskBody)
        // req.X → outros acessos diretos ao *http.Request
        if let Expr::Ident(name) = obj {
            if name == "req" {
                if field == "body" {
                    return Ok("_huskBody".into());
                }
                // req.method, req.url, etc. — acesso direto ao r
                return Ok(format!("r.{}", capitalize(field)));
            }
            if self.user_aliases.contains(name.as_str())
                || self.stdlib_aliases.contains(name.as_str())
            {
                return Ok(format!("{}.{}", name, field));
            }
        }

        // err.message → err.Error()
        if field == "message" {
            return Ok(format!("{}.Error()", self.gen_expr(obj, ctx)?));
        }

        // campo de struct do usuário → capitaliza
        Ok(format!(
            "{}.{}",
            self.gen_expr(obj, ctx)?,
            capitalize(field)
        ))
    }

    fn gen_index(&self, obj: &Expr, idx: &Expr, ctx: Ctx) -> Result<String, CodegenError> {
        // req.headers["X"] → r.Header.Get("X")
        // req.query["X"]   → r.URL.Query().Get("X")
        // req.body["X"]    → _huskBody["X"].(string)
        // req.ctx["X"]     → r.Context().Value("X")
        if let Expr::FieldAccess(inner, field) = obj {
            if let Expr::Ident(name) = inner.as_ref() {
                if name == "req" {
                    let key = self.gen_expr(idx, ctx)?;
                    return Ok(match field.as_str() {
                        "headers" => format!("r.Header.Get({})", key),
                        "query" => format!("r.URL.Query().Get({})", key),
                        "body" => format!("_huskBody[{}].(string)", key),
                        "ctx" => format!("r.Context().Value({})", key),
                        _ => format!("r.{}[{}]", capitalize(field), key),
                    });
                }
            }
        }
        // Para map index access (ex: user["field"]), adiciona type assertion .(string)
        // Isso é necessário porque map[string]interface{} retorna interface{}
        Ok(format!(
            "{}[{}].(string)",
            self.gen_expr(obj, ctx)?,
            self.gen_expr(idx, ctx)?
        ))
    }

    fn gen_call(&self, call: &CallExpr, ctx: Ctx) -> Result<String, CodegenError> {
        // next() dentro de middleware → next.ServeHTTP(w, r)
        if let Expr::Ident(name) = call.callee.as_ref() {
            if name == "next" && ctx == Ctx::Middleware {
                return Ok("next.ServeHTTP(w, r)".into());
            }
        }

        // set_ctx("key", valor) → ctx := context.WithValue(r.Context(), "key", valor)
        //                         r = r.WithContext(ctx)
        if is_builtin(&call.callee, "set_ctx") {
            let key = call
                .args
                .first()
                .ok_or_else(|| CodegenError::new("set_ctx() requer uma chave string"))?;
            let val = call
                .args
                .get(1)
                .ok_or_else(|| CodegenError::new("set_ctx() requer um valor"))?;
            let key_go = self.gen_expr(key, ctx)?;
            let val_go = self.gen_expr(val, ctx)?;
            let n = self.ctx_var_counter.get() + 1;
            self.ctx_var_counter.set(n);
            let ctx_var = format!("_huskCtx{}", n);
            return Ok(format!(
                "{} := context.WithValue(r.Context(), {}, {})\nr = r.WithContext({})",
                ctx_var, key_go, val_go, ctx_var
            ));
        }

        // parse_int(expr) → strconv.Atoi(expr)
        if is_builtin(&call.callee, "parse_int") {
            let arg = call
                .args
                .first()
                .ok_or_else(|| CodegenError::new("parse_int() requer um argumento"))?;
            let arg_go = self.gen_expr(arg, ctx)?;
            return Ok(format!("strconv.Atoi({})", arg_go));
        }

        // require_role(actual_value, expected_role) ou require_role(actual_value, expected_role, "mensagem")
        // Compara o valor real com o esperado: se diferente, 403
        // Ex: require_role(ctx.role, "master")
        if is_builtin(&call.callee, "require_role") {
            let actual = call.args.get(0).ok_or_else(|| {
                CodegenError::new("require_role() requer o valor real como 1o argumento")
            })?;
            let expected = call.args.get(1).ok_or_else(|| {
                CodegenError::new("require_role() requer o valor esperado como 2o argumento")
            })?;
            let actual_go = self.gen_expr(actual, ctx)?;
            let expected_go = self.gen_expr(expected, ctx)?;
            let msg = call
                .args
                .get(2)
                .map(|m| self.gen_expr(m, ctx))
                .transpose()?
                .unwrap_or_else(|| "\"Acesso restrito\"".into());
            return Ok(format!(
                "if {} != {} {{\n\
                 \tw.WriteHeader(403)\n\
                 \tw.Header().Set(\"Content-Type\", \"application/json\")\n\
                 \tjson.NewEncoder(w).Encode(map[string]interface{{}}{{\
\
                 \t\t\"erro\": {},\
\
                 \t}})\n\
                 \treturn\n\
                 }}",
                actual_go, expected_go, msg
            ));
        }

        // require_field("nome") ou require_field("nome", "mensagem")
        if is_builtin(&call.callee, "require_field") {
            let field = call
                .args
                .first()
                .ok_or_else(|| CodegenError::new("require_field() requer um nome de campo"))?;
            let field_go = self.gen_expr(field, ctx)?;
            let msg = call
                .args
                .get(1)
                .map(|m| self.gen_expr(m, ctx))
                .transpose()?
                .unwrap_or_else(|| format!("\"Campo obrigatório: \" + {}", field_go));
            return Ok(format!(
                "if {} == \"\" || _huskBody[{}] == nil {{\n\
                 \tw.WriteHeader(400)\n\
                 \tw.Header().Set(\"Content-Type\", \"application/json\")\n\
                 \tjson.NewEncoder(w).Encode(map[string]interface{{}}{{\
\
                 \t\t\"erro\": {},\
\
                 \t}})\n\
                 \treturn\n\
                 }}",
                field_go, field_go, msg
            ));
        }

        // String built-ins
        if let Expr::Ident(name) = call.callee.as_ref() {
            match name.as_str() {
                "len" => {
                    let arg = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("len() requer um argumento"))?;
                    return Ok(format!("len({})", self.gen_expr(arg, ctx)?));
                }
                "contains" => {
                    let s = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("contains() requer 2 argumentos"))?;
                    let sub = call
                        .args
                        .get(1)
                        .ok_or_else(|| CodegenError::new("contains() requer 2 argumentos"))?;
                    return Ok(format!(
                        "strings.Contains({}, {})",
                        self.gen_expr(s, ctx)?,
                        self.gen_expr(sub, ctx)?
                    ));
                }
                "starts_with" => {
                    let s = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("starts_with() requer 2 argumentos"))?;
                    let p = call
                        .args
                        .get(1)
                        .ok_or_else(|| CodegenError::new("starts_with() requer 2 argumentos"))?;
                    return Ok(format!(
                        "strings.HasPrefix({}, {})",
                        self.gen_expr(s, ctx)?,
                        self.gen_expr(p, ctx)?
                    ));
                }
                "replace" => {
                    let s = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("replace() requer 3 argumentos"))?;
                    let old = call
                        .args
                        .get(1)
                        .ok_or_else(|| CodegenError::new("replace() requer 3 argumentos"))?;
                    let new = call
                        .args
                        .get(2)
                        .ok_or_else(|| CodegenError::new("replace() requer 3 argumentos"))?;
                    return Ok(format!(
                        "strings.Replace({}, {}, {}, -1)",
                        self.gen_expr(s, ctx)?,
                        self.gen_expr(old, ctx)?,
                        self.gen_expr(new, ctx)?
                    ));
                }
                "split" => {
                    let s = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("split() requer 2 argumentos"))?;
                    let sep = call
                        .args
                        .get(1)
                        .ok_or_else(|| CodegenError::new("split() requer 2 argumentos"))?;
                    return Ok(format!(
                        "strings.Split({}, {})",
                        self.gen_expr(s, ctx)?,
                        self.gen_expr(sep, ctx)?
                    ));
                }
                "trim" => {
                    let arg = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("trim() requer um argumento"))?;
                    return Ok(format!("strings.TrimSpace({})", self.gen_expr(arg, ctx)?));
                }
                "upper" => {
                    let arg = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("upper() requer um argumento"))?;
                    return Ok(format!("strings.ToUpper({})", self.gen_expr(arg, ctx)?));
                }
                "lower" => {
                    let arg = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("lower() requer um argumento"))?;
                    return Ok(format!("strings.ToLower({})", self.gen_expr(arg, ctx)?));
                }
                "abs" => {
                    let arg = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("abs() requer um argumento"))?;
                    return Ok(format!("math.Abs({})", self.gen_expr(arg, ctx)?));
                }
                "sqrt" => {
                    let arg = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("sqrt() requer um argumento"))?;
                    return Ok(format!("math.Sqrt({})", self.gen_expr(arg, ctx)?));
                }
                "min" => {
                    let a = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("min() requer 2 argumentos"))?;
                    let b = call
                        .args
                        .get(1)
                        .ok_or_else(|| CodegenError::new("min() requer 2 argumentos"))?;
                    return Ok(format!(
                        "min({}, {})",
                        self.gen_expr(a, ctx)?,
                        self.gen_expr(b, ctx)?
                    ));
                }
                "max" => {
                    let a = call
                        .args
                        .first()
                        .ok_or_else(|| CodegenError::new("max() requer 2 argumentos"))?;
                    let b = call
                        .args
                        .get(1)
                        .ok_or_else(|| CodegenError::new("max() requer 2 argumentos"))?;
                    return Ok(format!(
                        "max({}, {})",
                        self.gen_expr(a, ctx)?,
                        self.gen_expr(b, ctx)?
                    ));
                }
                _ => {}
            }
        }

        // alias.metodo(args)
        // stdlib: env.get(x) → env_get(x)
        // usuário: usuarios.listar() → listar()
        if let Expr::FieldAccess(receiver, method) = call.callee.as_ref() {
            if let Expr::Ident(alias) = receiver.as_ref() {
                let fn_name = if self.stdlib_aliases.contains(alias.as_str()) {
                    format!("{}_{}", alias, method)
                } else if self.user_aliases.contains(alias.as_str()) {
                    format!("{}_{}", alias, method)
                } else {
                    method.clone()
                };
                let params = self.fn_params.get(&fn_name).map(|p| p.as_slice());
                let args = self.gen_args(&call.args, ctx, params)?;
                if self.stdlib_aliases.contains(alias.as_str()) {
                    return Ok(format!("{}_{}({})", alias, method, args));
                }
                if self.user_aliases.contains(alias.as_str()) {
                    return Ok(format!("{}_{}({})", alias, method, args));
                }
            }
        }

        let fn_name = if let Expr::Ident(name) = call.callee.as_ref() {
            self.fn_params.get(name).map(|p| p.as_slice())
        } else {
            None
        };
        let args = self.gen_args(&call.args, ctx, fn_name)?;
        let callee = self.gen_expr(&call.callee, ctx)?;
        Ok(format!("{}({})", callee, args))
    }

    fn gen_args(
        &self,
        args: &[Expr],
        ctx: Ctx,
        params: Option<&[(String, Type)]>,
    ) -> Result<String, CodegenError> {
        let mut out = Vec::new();
        for arg in args {
            if let Expr::Spread(spread) = arg {
                let map_expr = self.gen_expr(spread, ctx)?;
                if let Some(params) = params {
                    // spread expands only remaining params (after already-provided args)
                    let remaining = &params[out.len()..];
                    for (pname, pty) in remaining.iter() {
                        let cast = match pty {
                            Type::Int => ".(int)",
                            Type::Float => ".(float64)",
                            Type::Bool => ".(bool)",
                            _ => ".(string)",
                        };
                        out.push(format!("{}[\"{}\"]{}", map_expr, pname, cast));
                    }
                }
            } else {
                out.push(self.gen_expr(arg, ctx)?);
            }
        }
        Ok(out.join(", "))
    }

    fn gen_map_lit(&self, m: &MapLit, ctx: Ctx) -> Result<String, CodegenError> {
        let fields = m
            .fields
            .iter()
            .map(|(k, v)| Ok(format!("\"{}\": {}", k, self.gen_expr(v, ctx)?)))
            .collect::<Result<Vec<_>, CodegenError>>()?
            .join(", ");
        Ok(format!("map[string]interface{{}}{{{}}}", fields))
    }

    fn gen_struct_init(&self, s: &StructInit, ctx: Ctx) -> Result<String, CodegenError> {
        let fields = s
            .fields
            .iter()
            .map(|(k, v)| Ok(format!("{}: {}", capitalize(k), self.gen_expr(v, ctx)?)))
            .collect::<Result<Vec<_>, CodegenError>>()?
            .join(", ");
        Ok(format!("{}{{{}}}", s.name, fields))
    }
}

// --- helpers puros ---

fn gen_lit(lit: &Lit) -> String {
    match lit {
        Lit::Int(n) => n.to_string(),
        Lit::Float(f) => format!("{}", f),
        Lit::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Lit::Bool(b) => b.to_string(),
    }
}

fn go_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".into(),
        Type::Float => "float64".into(),
        Type::String => "string".into(),
        Type::Bool => "bool".into(),
        Type::Error => "error".into(),
        Type::Map => "map[string]interface{}".into(),
        Type::List(inner) => format!("[]{}", go_type(inner)),
        Type::Named(n) => n.clone(),
    }
}

fn go_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn is_builtin(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Ident(n) if n == name)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn block_uses_body(block: &Block) -> bool {
    block.stmts.iter().any(|s| stmt_uses_body(s))
}

fn stmt_uses_body(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(exprs) => exprs.iter().any(expr_uses_body),
        Stmt::Let(l) => expr_uses_body(&l.value),
        Stmt::LetMulti(l) => expr_uses_body(&l.value),
        Stmt::Expr(e) => expr_uses_body(e),
        Stmt::If(i) => {
            block_uses_body(&i.then_block) || i.else_block.as_ref().map_or(false, block_uses_body)
        }
        Stmt::TryLet(t) => expr_uses_body(&t.call),
        Stmt::Assign(a) => expr_uses_body(&a.target) || expr_uses_body(&a.value),
    }
}

fn expr_uses_body(expr: &Expr) -> bool {
    match expr {
        Expr::Index(obj, _) => {
            if let Expr::FieldAccess(inner, field) = obj.as_ref() {
                if let Expr::Ident(name) = inner.as_ref() {
                    return name == "req" && field == "body";
                }
            }
            false
        }
        Expr::Call(c) => {
            // require_field usa _huskBody internamente
            if let Expr::Ident(name) = c.callee.as_ref() {
                if name == "require_field" {
                    return true;
                }
            }
            expr_uses_body(&c.callee) || c.args.iter().any(expr_uses_body)
        }
        Expr::FieldAccess(e, field) => {
            // req.body (sem index) também conta como uso do body
            if let Expr::Ident(name) = e.as_ref() {
                if name == "req" && field == "body" {
                    return true;
                }
            }
            expr_uses_body(e)
        }
        Expr::BinOp(l, _, r) => expr_uses_body(l) || expr_uses_body(r),
        Expr::Unary(_, e) => expr_uses_body(e),
        Expr::Try(t) => expr_uses_body(&t.expr),
        Expr::Spread(e) => expr_uses_body(e),
        _ => false,
    }
}

fn infer_fn_return(block: &Block) -> Option<&'static str> {
    for stmt in &block.stmts {
        if let Stmt::Return(exprs) = stmt {
            if let Some(expr) = exprs.first() {
                return Some(match expr {
                    Expr::Lit(Lit::Str(_)) => "string",
                    Expr::Lit(Lit::Int(_)) => "int",
                    Expr::Lit(Lit::Float(_)) => "float64",
                    Expr::Lit(Lit::Bool(_)) => "bool",
                    _ => "interface{}",
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use husk_lexer::Lexer;
    use husk_parser::Parser;

    fn codegen(src: &str) -> String {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        Codegen::new().generate(&program).unwrap()
    }

    #[test]
    fn test_programa_completo() {
        let go = codegen(
            r#"
fn greeting() {
    return "Hello, World!"
}
route GET /hello {
    return greeting()
}
"#,
        );
        assert!(go.contains("func greeting() string {"));
        assert!(go.contains("r.Get(\"/hello\""));
        assert!(go.contains("json.NewEncoder(w).Encode(greeting())"));
    }

    #[test]
    fn test_route_com_json() {
        let go = codegen(r#"route GET /ping { return json({ status: "ok" }) }"#);
        assert!(go.contains("json.NewEncoder(w).Encode"));
        assert!(go.contains("\"status\": \"ok\""));
    }

    #[test]
    fn test_struct_def() {
        let go = codegen("struct Usuario {\nid int\nnome string\n}");
        assert!(go.contains("type Usuario struct {"));
        assert!(go.contains("Id int"));
        assert!(go.contains("Nome string"));
    }

    #[test]
    fn test_tipo_map_e_list() {
        let go = codegen(
            r#"
fn buscar(email string) (map, error) {
    return nil, nil
}

fn listar() ([]map, error) {
    return nil, nil
}
"#,
        );
        assert!(go.contains("func buscar(email string) (map[string]interface{}, error)"));
        assert!(go.contains("func listar() ([]map[string]interface{}, error)"));
    }

    #[test]
    fn test_if_simples() {
        let go = codegen(
            r#"
fn checar(x int) string {
    if x == 0 {
        return "zero"
    }
    return "outro"
}
"#,
        );
        assert!(go.contains("if x == 0 {"));
        assert!(go.contains("return \"zero\""));
    }

    #[test]
    fn test_if_else() {
        let go = codegen(
            r#"
fn checar(x int) string {
    if x == 0 {
        return "zero"
    } else {
        return "outro"
    }
}
"#,
        );
        assert!(go.contains("} else {"));
    }

    #[test]
    fn test_nil() {
        let go = codegen(
            r#"
fn buscar(id int) (string, error) {
    return "ok", nil
}
"#,
        );
        assert!(go.contains("(string, error)"));
        assert!(go.contains("return \"ok\", nil"));
    }

    #[test]
    fn test_let_multi() {
        let go = codegen(
            r#"
fn buscar(id int) (string, error) {
    return "x", nil
}
fn f() string {
    let val, err = buscar(1)
    return val
}
"#,
        );
        assert!(go.contains("val, err := buscar(1)"));
    }

    #[test]
    fn test_shadowing_simples() {
        let go = codegen(
            r#"
fn f() string {
    let err = "primeiro"
    let err = "segundo"
    return err
}
"#,
        );
        // primeira declaração: :=, segunda: =
        assert!(go.contains("err := \"primeiro\""));
        assert!(go.contains("err = \"segundo\""));
    }

    #[test]
    fn test_shadowing_multi() {
        let go = codegen(
            r#"
fn buscar(x int) (string, error) { return "x", nil }
fn f() string {
    let a, err = buscar(1)
    let b, err = buscar(2)
    return a
}
"#,
        );
        // primeira: := (a e err são novos)
        assert!(go.contains("a, err := buscar(1)"));
        // segunda: := (b é novo, mesmo err já existindo — Go aceita)
        assert!(go.contains("b, err := buscar(2)"));
    }

    #[test]
    fn test_shadowing_multi_todos_existentes() {
        let go = codegen(
            r#"
fn buscar(x int) (string, error) { return "x", nil }
fn f() string {
    let a, err = buscar(1)
    let a, err = buscar(2)
    return a
}
"#,
        );
        assert!(go.contains("a, err := buscar(1)"));
        // ambos já declarados → =
        assert!(go.contains("a, err = buscar(2)"));
    }

    #[test]
    fn test_req_params() {
        let go = codegen(
            r#"
route GET /users/:id {
    let id = req.params.id
    return id
}
"#,
        );
        assert!(go.contains("chi.URLParam(r, \"id\")"));
    }

    #[test]
    fn test_req_headers() {
        let go = codegen(
            r#"
route GET /secure {
    let token = req.headers["Authorization"]
    return token
}
"#,
        );
        assert!(go.contains("r.Header.Get(\"Authorization\")"));
    }

    #[test]
    fn test_req_body() {
        let go = codegen(
            r#"
route POST /login {
    let email = req.body["email"]
    let senha = req.body["senha"]
    return json({ ok: true })
}
"#,
        );
        assert!(go.contains("var _huskBody map[string]interface{}"));
        assert!(go.contains("json.NewDecoder(r.Body).Decode(&_huskBody)"));
        assert!(go.contains("_huskBody[\"email\"]"));
        assert!(go.contains("_huskBody[\"senha\"]"));
        assert!(go.contains("\"encoding/json\""));
    }

    #[test]
    fn test_req_query() {
        let go = codegen(
            r#"
route GET /search {
    let q = req.query["q"]
    return q
}
"#,
        );
        assert!(go.contains("r.URL.Query().Get(\"q\")"));
    }

    #[test]
    fn test_middleware() {
        let go = codegen(
            r#"
middleware autenticado {
    next()
}
route GET /perfil [autenticado] {
    return "ok"
}
"#,
        );
        assert!(go.contains("func autenticado(next http.Handler) http.Handler {"));
        assert!(go.contains("next.ServeHTTP(w, r)"));
        assert!(go.contains("r.With(autenticado).Get(\"/perfil\""));
    }

    #[test]
    fn test_req_ctx_escrita_e_leitura() {
        let go = codegen(
            r#"
middleware autenticado {
    set_ctx("user_id", "42")
    next()
}
route GET /perfil [autenticado] {
    let uid = req.ctx["user_id"]
    return uid
}
"#,
        );
        // escrita no middleware
        assert!(go.contains("_huskCtx1 := context.WithValue(r.Context(), \"user_id\", \"42\")"));
        assert!(go.contains("r = r.WithContext(_huskCtx1)"));
        // leitura na rota
        assert!(go.contains("r.Context().Value(\"user_id\")"));
        // import context adicionado
        assert!(go.contains("\"context\""));
    }

    #[test]
    fn test_erro_message() {
        let go = codegen(
            r#"
fn f() string {
    let val, err = buscar(1)
    if err != nil {
        return err.message
    }
    return val
}
"#,
        );
        assert!(go.contains("err.Error()"));
        assert!(go.contains("err != nil"));
    }

    #[test]
    fn test_middleware_return_status_json() {
        // Middleware com return status/json deve gerar resposta HTTP, não return Go
        let go = codegen(
            r#"
middleware auth {
    let token = req.headers["Authorization"]
    if token == "" {
        return status(401, json({ erro: "token ausente" }))
    }
    next()
}
route GET /perfil [auth] {
    return "ok"
}
"#,
        );
        // Deve gerar w.WriteHeader + json.NewEncoder em vez de "return status("
        assert!(
            !go.contains("return status("),
            "não deve gerar 'return status' literal"
        );
        assert!(go.contains("w.WriteHeader(401)"));
        assert!(go.contains("json.NewEncoder(w).Encode"));
        assert!(go.contains("next.ServeHTTP(w, r)"));
    }

    #[test]
    fn test_req_body_type_assertion() {
        // req.body["x"] deve gerar type assertion .(string)
        let go = codegen(
            r#"
route POST /login {
    let email = req.body["email"]
    return email
}
"#,
        );
        assert!(go.contains("_huskBody[\"email\"].(string)"));
    }

    #[test]
    fn test_try_let_simples() {
        let go = codegen(
            r#"
route GET /user/:id {
    let nome = buscar(1)?
    return nome
}
"#,
        );
        // Gera preâmbulo com __try1_val, __try1_err
        assert!(go.contains("__try1_val, __try1_err := buscar(1)"));
        assert!(go.contains("if __try1_err != nil"));
        assert!(go.contains("w.WriteHeader(500)"));
        assert!(go.contains("\"erro\": __try1_err.Error()"));
        assert!(go.contains("nome := __try1_val"));
    }

    #[test]
    fn test_try_let_com_status() {
        let go = codegen(
            r#"
route GET /user/:id {
    let nome = buscar(1)? 404
    return nome
}
"#,
        );
        assert!(go.contains("w.WriteHeader(404)"));
        assert!(go.contains("nome := __try1_val"));
    }

    #[test]
    fn test_try_let_com_mensagem() {
        let go = codegen(
            r#"
route GET /user/:id {
    let nome = buscar(1)? 400 "Usuário não encontrado"
    return nome
}
"#,
        );
        assert!(go.contains("w.WriteHeader(400)"));
        assert!(go.contains("\"erro\": \"Usuário não encontrado\""));
    }

    #[test]
    fn test_try_expressao_aninhada() {
        // expr? dentro de argumentos de função
        let go = codegen(
            r#"
fn buscar(id int) string { return "x" }
route GET /user/:id {
    let x = buscar(parse_int("42")?)?
    return x
}
"#,
        );
        // Preâmbulos vêm ANTES da assignment
        // Inner (parse_int) é executado primeiro no Go → __try2 (contador incrementa na ordem inversa)
        assert!(go.contains("__try2_val, __try2_err := strconv.Atoi"));
        assert!(go.contains("__try1_val, __try1_err := buscar"));
        assert!(go.contains("x := __try1_val"));
        // __try2 (inner, executado primeiro) deve vir antes de __try1 no código
        let pos2 = go.find("__try2_val, __try2_err").unwrap();
        let pos1 = go.find("__try1_val, __try1_err").unwrap();
        assert!(
            pos2 < pos1,
            "inner try (__try2) deve vir antes do outer (__try1) no código Go"
        );
    }

    #[test]
    fn test_status_com_map_lit_implica_json() {
        // return status(401, { erro: "x" })  → JSON automático, sem json() explícito
        let go = codegen(
            r#"
route GET /secure {
    return status(401, { erro: "não autorizado" })
}
"#,
        );
        assert!(go.contains("w.WriteHeader(401)"));
        assert!(go.contains("json.NewEncoder(w).Encode"));
        assert!(go.contains("\"erro\": \"não autorizado\""));
    }

    #[test]
    fn test_return_map_lit_direto_implica_json() {
        // return { chave: "valor" }  → JSON automático
        let go = codegen(
            r#"
route GET /ping {
    return { status: "ok" }
}
"#,
        );
        assert!(go.contains("json.NewEncoder(w).Encode"));
        assert!(go.contains("\"status\": \"ok\""));
    }

    #[test]
    fn test_req_body_como_variavel() {
        // let body = req.body  → _huskBody, não r.Body
        let go = codegen(
            r#"
route POST /dados {
    let body = req.body
    return body["nome"]
}
"#,
        );
        assert!(go.contains("body := _huskBody"));
        assert!(!go.contains("body := r.Body"));
        assert!(go.contains("body[\"nome\"].(string)"));
    }

    #[test]
    fn test_parse_int_builtin() {
        let go = codegen(
            r#"
route GET /convert {
    let n = parse_int("42")?
    return n
}
"#,
        );
        assert!(go.contains("strconv.Atoi(\"42\")"));
        assert!(go.contains("\"strconv\""));
        assert!(go.contains("__try1_val, __try1_err := strconv.Atoi(\"42\")"));
    }

    #[test]
    fn test_spread_body() {
        let go = codegen(
            r#"
fn criar(nome string, idade int) map { return nil }
route POST /criar {
    let body = req.body
    let x = criar(body...)?
    return x
}
"#,
        );
        assert!(go.contains("criar(body[\"nome\"].(string), body[\"idade\"].(int))"));
    }

    #[test]
    fn test_string_math_builtins() {
        let go = codegen(
            r#"
fn f() {
    let a = len("abc")
    let b = contains("abc", "a")
    let c = starts_with("abc", "a")
    let d = replace("a", "a", "b")
    let e = trim(" x ")
    let f = upper("a")
    let g = lower("A")
    let h = abs(-1.0)
    let i = sqrt(4.0)
    let j = min(1, 2)
    let k = max(1, 2)
}
"#,
        );
        assert!(go.contains("len(\"abc\")"));
        assert!(go.contains("strings.Contains"));
        assert!(go.contains("strings.HasPrefix"));
        assert!(go.contains("strings.Replace"));
        assert!(go.contains("strings.TrimSpace"));
        assert!(go.contains("strings.ToUpper"));
        assert!(go.contains("strings.ToLower"));
        assert!(go.contains("math.Abs"));
        assert!(go.contains("math.Sqrt"));
        assert!(go.contains("min(1, 2)"));
        assert!(go.contains("max(1, 2)"));
        assert!(go.contains("\"strings\""));
        assert!(go.contains("\"math\""));
    }

    #[test]
    fn test_split_builtin() {
        let go = codegen(r#"fn f() { let x = split("a,b", ",") }"#);
        assert!(go.contains("strings.Split"));
    }

    #[test]
    fn test_require_field() {
        let go = codegen(
            r#"
route POST /criar {
    require_field("nome")
    return { ok: true }
}
"#,
        );
        assert!(go.contains("_huskBody[\"nome\"] == nil"));
        assert!(go.contains("w.WriteHeader(400)"));
        assert!(go.contains("\"erro\""));
    }

    #[test]
    fn test_require_field_com_mensagem() {
        let go = codegen(
            r#"
route POST /criar {
    require_field("email", "Email é obrigatório")
    return { ok: true }
}
"#,
        );
        assert!(go.contains("_huskBody[\"email\"] == nil"));
        assert!(go.contains("\"Email é obrigatório\""));
    }

    #[test]
    fn test_require_role() {
        let go = codegen(
            r#"
route GET /admin {
    require_role("admin", "master")
    return "ok"
}
"#,
        );
        // require_role compara o 1o argumento (real) com o 2o (esperado)
        assert!(go.contains("\"admin\" != \"master\""));
        assert!(go.contains("w.WriteHeader(403)"));
        assert!(go.contains("\"erro\": \"Acesso restrito\""));
    }

    #[test]
    fn test_require_role_com_mensagem() {
        let go = codegen(
            r#"
route GET /admin {
    require_role(role, "admin", "Só admin mesmo")
    return "ok"
}
"#,
        );
        assert!(go.contains("role"));
        assert!(go.contains("\"admin\""));
        assert!(go.contains("\"erro\": \"Só admin mesmo\""));
    }

    #[test]
    fn test_route_param_typed_int() {
        let go = codegen(
            r#"
route GET /api/clientes/:id<int> {
    let x = req.params.id
    return x
}
"#,
        );
        assert!(go.contains("_huskParam_id, _ := strconv.Atoi(chi.URLParam(r, \"id\"))"));
        assert!(go.contains("x := _huskParam_id"));
        assert!(go.contains("\"strconv\""));
    }

    #[test]
    fn test_route_param_untyped_string() {
        let go = codegen(
            r#"
route GET /users/:slug {
    let s = req.params.slug
    return s
}
"#,
        );
        // Param sem tipo continua gerando chi.URLParam
        assert!(go.contains("chi.URLParam(r, \"slug\")"));
        assert!(!go.contains("_huskParam_slug"));
    }

    #[test]
    fn test_middleware_ctx_assignment() {
        let go = codegen(
            r#"
middleware autenticado -> ctx {
    ctx.role = "admin"
    ctx.user_id = "42"
    next()
}
route GET /perfil [autenticado] -> ctx {
    let r = ctx.role
    return r
}
"#,
        );
        // Middleware gera context.WithValue para ctx.role
        assert!(go.contains("context.WithValue(r.Context(), \"__husk_ctx_role\""));
        assert!(go.contains("context.WithValue(r.Context(), \"__husk_ctx_user_id\""));
        assert!(go.contains("r = r.WithContext("));
    }
}
