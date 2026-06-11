use crate::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        format_item(item, &mut out, 0);
        out.push('\n');
    }
    out
}

pub fn format_program_with_source(program: &Program, source: &str) -> String {
    let mut cursor = CommentCursor::new(source);
    let mut out = String::new();
    for (i, item) in program.items.iter().enumerate() {
        if i > 0 && cursor.peek() != Some(item_start_line(item)) {
            out.push('\n');
        }
        format_item_with_cursor(item, &mut cursor, &mut out, 0);
        out.push('\n');
    }
    cursor.emit_remaining(0, &mut out);
    out
}

fn item_start_line(item: &Item) -> usize {
    match item {
        Item::FnDef(f) => f.span.line,
        Item::RouteDef(r) => r.span.line,
        Item::StructDef(s) => s.span.line,
        Item::Import(i) => i.span.line,
        Item::MiddlewareDef(m) => m.span.line,
        Item::CorsDef(c) => c.span.line,
        Item::SchemaDef(s) => s.span.line,
        Item::ModelDef(m) => m.span.line,
    }
}

struct CommentCursor {
    lines: Vec<(usize, String)>,
    idx: usize,
}

impl CommentCursor {
    fn new(source: &str) -> Self {
        let mut lines = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(content) = trimmed.strip_prefix("//") {
                lines.push((i, content.to_string()));
            }
        }
        Self { lines, idx: 0 }
    }

    fn peek(&self) -> Option<usize> {
        self.lines.get(self.idx).map(|(line, _)| *line)
    }

    fn emit_before_line(&mut self, line: usize, indent: usize, out: &mut String) {
        let ind = indent_str(indent);
        // line is 1-indexed, internal lines are 0-indexed
        let target = line.saturating_sub(1);
        while self.idx < self.lines.len() && self.lines[self.idx].0 < target {
            out.push_str(&format!("{}//{}\n", ind, self.lines[self.idx].1));
            self.idx += 1;
        }
    }

    fn emit_remaining(&mut self, indent: usize, out: &mut String) {
        let ind = indent_str(indent);
        while self.idx < self.lines.len() {
            out.push_str(&format!("{}//{}\n", ind, self.lines[self.idx].1));
            self.idx += 1;
        }
    }
}

fn format_item(item: &Item, out: &mut String, indent: usize) {
    match item {
        Item::FnDef(f) => format_fn_def(f, out, indent),
        Item::RouteDef(r) => format_route_def(r, out, indent),
        Item::StructDef(s) => format_struct_def(s, out, indent),
        Item::Import(i) => format_import(i, out, indent),
        Item::MiddlewareDef(m) => format_middleware_def(m, out, indent),
        Item::CorsDef(c) => format_cors_def(c, out, indent),
        Item::SchemaDef(s) => format_schema_def(s, out, indent),
        Item::ModelDef(m) => format_model_def(m, out, indent),
    }
}

fn format_item_with_cursor(item: &Item, cursor: &mut CommentCursor, out: &mut String, indent: usize) {
    match item {
        Item::FnDef(f) => {
            cursor.emit_before_line(f.span.line, indent, out);
            format_fn_def(f, out, indent);
        }
        Item::RouteDef(r) => {
            cursor.emit_before_line(r.span.line, indent, out);
            format_route_def(r, out, indent);
        }
        Item::StructDef(s) => {
            cursor.emit_before_line(s.span.line, indent, out);
            format_struct_def(s, out, indent);
        }
        Item::Import(i) => {
            cursor.emit_before_line(i.span.line, indent, out);
            format_import(i, out, indent);
        }
        Item::MiddlewareDef(m) => {
            cursor.emit_before_line(m.span.line, indent, out);
            format_middleware_def(m, out, indent);
        }
        Item::CorsDef(c) => {
            cursor.emit_before_line(c.span.line, indent, out);
            format_cors_def(c, out, indent);
        }
        Item::SchemaDef(s) => {
            cursor.emit_before_line(s.span.line, indent, out);
            format_schema_def(s, out, indent);
        }
        Item::ModelDef(m) => {
            cursor.emit_before_line(m.span.line, indent, out);
            format_model_def(m, out, indent);
        }
    }
}

fn indent_str(level: usize) -> String {
    "    ".repeat(level)
}

fn format_fn_def(f: &FnDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}fn {}(", i, f.name));
    for (j, p) in f.params.iter().enumerate() {
        if j > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{} {}", p.name, format_type(&p.ty)));
    }
    out.push(')');
    match &f.return_type {
        ReturnType::None => {}
        ReturnType::Single(ty) => {
            out.push(' ');
            out.push_str(&format_type(ty));
        }
        ReturnType::Tuple(tys) => {
            out.push_str(" (");
            for (k, ty) in tys.iter().enumerate() {
                if k > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format_type(ty));
            }
            out.push(')');
        }
    }
    out.push_str(" {\n");
    format_block_with_cursor(&f.body, out, indent + 1);
    out.push_str(&format!("{}}}", i));
}

fn format_route_def(r: &RouteDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}route {} {}", i, format_http_method(&r.method), r.path.to_string()));
    if !r.middlewares.is_empty() || r.timeout_secs.is_some() || r.rate_limit.is_some() {
        out.push_str(" [");
        for (j, mw) in r.middlewares.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(mw);
        }
        if let Some(t) = r.timeout_secs {
            if !r.middlewares.is_empty() {
                out.push_str(", ");
            }
            out.push_str(&format!("timeout={}", t));
        }
        if let Some(rl) = r.rate_limit {
            if !r.middlewares.is_empty() || r.timeout_secs.is_some() {
                out.push_str(", ");
            }
            out.push_str(&format!("rate_limit={}", rl));
        }
        out.push(']');
    }
    if let Some(ref ctx) = r.ctx_var {
        out.push_str(&format!(" -> {}", ctx));
    }
    out.push_str(" {\n");
    format_block_with_cursor(&r.body, out, indent + 1);
    out.push_str(&format!("{}}}", i));
}

fn format_middleware_def(m: &MiddlewareDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}middleware {}", i, m.name));
    if let Some(ref ctx) = m.ctx_var {
        out.push_str(&format!(" -> {}", ctx));
    }
    out.push_str(" {\n");
    format_block_with_cursor(&m.body, out, indent + 1);
    out.push_str(&format!("{}}}", i));
}

fn format_struct_def(s: &StructDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}struct {} {{\n", i, s.name));
    for field in &s.fields {
        out.push_str(&format!("{}{} {}\n", indent_str(indent + 1), field.name, format_type(&field.ty)));
    }
    out.push_str(&format!("{}}}", i));
}

fn format_import(imp: &ImportDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    if imp.is_stdlib {
        out.push_str(&format!("{}import \"husk/{}\" as {}", i, imp.path, imp.alias));
    } else {
        out.push_str(&format!("{}import \"{}\" as {}", i, imp.path, imp.alias));
    }
    out.push('\n');
}

fn format_cors_def(c: &CorsDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}cors {{\n", i));
    if !c.origins.is_empty() {
        out.push_str(&format!("{}  origins: [", i));
        for (j, o) in c.origins.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("\"{}\"", o));
        }
        out.push_str("]\n");
    }
    if !c.methods.is_empty() {
        out.push_str(&format!("{}  methods: [", i));
        for (j, m) in c.methods.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("\"{}\"", m));
        }
        out.push_str("]\n");
    }
    if !c.headers.is_empty() {
        out.push_str(&format!("{}  headers: [", i));
        for (j, h) in c.headers.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("\"{}\"", h));
        }
        out.push_str("]\n");
    }
    out.push_str(&format!("{}}}", i));
}

fn format_schema_def(s: &SchemaDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}schema {} {{\n", i, s.name));
    for field in &s.fields {
        out.push_str(&format!("{}  {}: {}", i, field.name, format_type(&field.ty)));
        for v in &field.validators {
            match v {
                Validator::Required => out.push_str(" required"),
                Validator::Email => out.push_str(" email()"),
                Validator::Unique => out.push_str(" unique()"),
                Validator::Min(n) => out.push_str(&format!(" min({})", n)),
                Validator::Max(n) => out.push_str(&format!(" max({})", n)),
            }
        }
        out.push('\n');
    }
    out.push_str(&format!("{}}}", i));
}

fn format_model_def(m: &ModelDef, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    out.push_str(&format!("{}model {} {{\n", i, m.name));
    out.push_str(&format!("{}  table: \"{}\"\n", i, m.table));
    out.push_str(&format!("{}  fields: {{\n", i));
    for field in &m.fields {
        out.push_str(&format!("{}    {}: {}", i, field.name, format_type(&field.ty)));
        for v in &field.validators {
            match v {
                Validator::Required => out.push_str(" required"),
                Validator::Email => out.push_str(" email()"),
                Validator::Unique => out.push_str(" unique()"),
                Validator::Min(n) => out.push_str(&format!(" min({})", n)),
                Validator::Max(n) => out.push_str(&format!(" max({})", n)),
            }
        }
        out.push('\n');
    }
    out.push_str(&format!("{}  }}\n", i));
    out.push_str(&format!("{}}}", i));
}

fn format_block(block: &Block, out: &mut String, indent: usize) {
    for stmt in &block.stmts {
        format_stmt(stmt, out, indent);
        out.push('\n');
    }
}

fn format_block_with_cursor(block: &Block, out: &mut String, indent: usize) {
    for stmt in &block.stmts {
        format_stmt(stmt, out, indent);
        out.push('\n');
    }
}

fn format_stmt(stmt: &Stmt, out: &mut String, indent: usize) {
    let i = indent_str(indent);
    match stmt {
        Stmt::Return(exprs) => {
            out.push_str(&i);
            out.push_str("return");
            for (j, e) in exprs.iter().enumerate() {
                if j > 0 {
                    out.push_str(",");
                }
                out.push(' ');
                format_expr(e, out);
            }
        }
        Stmt::Let(ls) => {
            out.push_str(&i);
            out.push_str("let ");
            out.push_str(&ls.name);
            if let Some(ref ty) = ls.ty {
                out.push(' ');
                out.push_str(&format_type(ty));
            }
            out.push_str(" = ");
            format_expr(&ls.value, out);
        }
        Stmt::LetMulti(lm) => {
            out.push_str(&i);
            out.push_str("let ");
            for (j, name) in lm.names.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
            }
            out.push_str(" = ");
            format_expr(&lm.value, out);
        }
        Stmt::TryLet(tl) => {
            out.push_str(&i);
            out.push_str("let ");
            out.push_str(&tl.name);
            out.push_str(" = ");
            format_expr(&tl.call, out);
            out.push('?');
            if let Some(ref code) = tl.status_code {
                out.push_str(&format!(" {}", code));
            }
            if let Some(ref msg) = tl.message {
                out.push_str(&format!(" \"{}\"", msg));
            }
            if tl.circuit_breaker {
                out.push_str(" break");
            }
        }
        Stmt::If(if_stmt) => {
            out.push_str(&i);
            out.push_str("if ");
            format_expr(&if_stmt.condition, out);
            out.push_str(" {\n");
            format_block(&if_stmt.then_block, out, indent + 1);
            out.push_str(&format!("{}}}", i));
            if let Some(ref else_block) = if_stmt.else_block {
                out.push_str(" else {\n");
                format_block(else_block, out, indent + 1);
                out.push_str(&format!("{}}}", i));
            }
        }
        Stmt::ForIn(for_in) => {
            out.push_str(&i);
            out.push_str("for ");
            out.push_str(&for_in.item);
            out.push_str(" in ");
            format_expr(&for_in.collection, out);
            out.push_str(" {\n");
            format_block(&for_in.body, out, indent + 1);
            out.push_str(&format!("{}}}", i));
        }
        Stmt::TryCatch(tc) => {
            out.push_str(&i);
            out.push_str("try {\n");
            format_block(&tc.try_block, out, indent + 1);
            out.push_str(&format!("{}}} catch {} {{\n", i, tc.catch_var));
            format_block(&tc.catch_block, out, indent + 1);
            out.push_str(&format!("{}}}", i));
        }
        Stmt::Retry(r) => {
            out.push_str(&i);
            out.push_str(&format!("retry {} {} {{\n", r.attempts, r.delay_ms));
            format_block(&r.body, out, indent + 1);
            out.push_str(&format!("{}}}", i));
        }
        Stmt::Assign(a) => {
            out.push_str(&i);
            format_expr(&a.target, out);
            out.push_str(" = ");
            format_expr(&a.value, out);
        }
        Stmt::Expr(e) => {
            out.push_str(&i);
            format_expr(e, out);
        }
    }
}

fn format_expr(expr: &Expr, out: &mut String) {
    match expr {
        Expr::Lit(lit) => format_lit(lit, out),
        Expr::Nil => out.push_str("nil"),
        Expr::Ident(name) => out.push_str(name),
        Expr::Call(call) => {
            format_expr(&call.callee, out);
            out.push('(');
            for (i, arg) in call.args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(arg, out);
            }
            out.push(')');
        }
        Expr::FieldAccess(obj, field) => {
            format_expr(obj, out);
            out.push('.');
            out.push_str(field);
        }
        Expr::Index(arr, idx) => {
            format_expr(arr, out);
            out.push('[');
            format_expr(idx, out);
            out.push(']');
        }
        Expr::BinOp(left, op, right) => {
            format_expr(left, out);
            out.push(' ');
            format_binop(op, out);
            out.push(' ');
            format_expr(right, out);
        }
        Expr::Unary(op, expr) => {
            format_unaryop(op, out);
            format_expr(expr, out);
        }
        Expr::MapLit(map) => {
            out.push_str("{ ");
            for (i, (key, val)) in map.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(key);
                out.push_str(": ");
                format_expr(val, out);
            }
            out.push_str(" }");
        }
        Expr::StructInit(si) => {
            out.push_str(&si.name);
            out.push_str(" { ");
            for (i, (key, val)) in si.fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(key);
                out.push_str(": ");
                format_expr(val, out);
            }
            out.push_str(" }");
        }
        Expr::Try(try_expr) => {
            format_expr(&try_expr.expr, out);
            out.push('?');
            if let Some(ref code) = try_expr.status_code {
                out.push_str(&format!(" {}", code));
            }
            if let Some(ref msg) = try_expr.message {
                out.push_str(&format!(" \"{}\"", msg));
            }
            if try_expr.circuit_breaker {
                out.push_str(" break");
            }
        }
        Expr::Spread(inner) => {
            format_expr(inner, out);
            out.push_str("...");
        }
    }
}

fn format_lit(lit: &Lit, out: &mut String) {
    match lit {
        Lit::Int(n) => out.push_str(&n.to_string()),
        Lit::Float(f) => out.push_str(&f.to_string()),
        Lit::Str(s) => {
            out.push_str(&format!("\"{}\"", s));
        }
        Lit::Bool(b) => {
            if *b {
                out.push_str("true");
            } else {
                out.push_str("false");
            }
        }
    }
}

fn format_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Float => "float".to_string(),
        Type::String => "string".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Error => "error".to_string(),
        Type::Map => "map".to_string(),
        Type::List(inner) => format!("[]{}", format_type(inner)),
        Type::Named(name) => name.clone(),
    }
}

fn format_http_method(method: &HttpMethod) -> &str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
    }
}

fn format_binop(op: &BinOp, out: &mut String) {
    match op {
        BinOp::Add => out.push('+'),
        BinOp::Sub => out.push('-'),
        BinOp::Mul => out.push('*'),
        BinOp::Div => out.push('/'),
        BinOp::Mod => out.push('%'),
        BinOp::Eq => out.push_str("=="),
        BinOp::NotEq => out.push_str("!="),
        BinOp::Lt => out.push('<'),
        BinOp::Gt => out.push('>'),
        BinOp::LtEq => out.push_str("<="),
        BinOp::GtEq => out.push_str(">="),
        BinOp::And => out.push_str("&&"),
        BinOp::Or => out.push_str("||"),
    }
}

fn format_unaryop(op: &UnaryOp, out: &mut String) {
    match op {
        UnaryOp::Not => out.push('!'),
        UnaryOp::Neg => out.push('-'),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Parser, formatter::{format_program, format_program_with_source}};
    use husk_lexer::Lexer;

    fn parse(src: &str) -> crate::ast::Program {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn test_format_fn_simples() {
        let src = "fn ola() {\n    return \"Hello\"\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_fn_com_param() {
        let src = "fn soma(a int, b int) int {\n    return a + b\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_route_simples() {
        let src = "route GET /hello {\n    return greeting()\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_route_com_mw() {
        let src = "route GET /admin [auth] {\n    return \"admin\"\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_struct() {
        let src = "struct User {\n    name string\n    age int\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_if_else() {
        let src = "fn f(x int) {\n    if x > 0 {\n        return \"ok\"\n    } else {\n        return \"no\"\n    }\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_for_in() {
        let src = "fn f() {\n    for item in items {\n        return item\n    }\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_complexo() {
        let src = "fn f() (string, error) {\n    let result = db.query_one(\"select nome from users where id = $1\", id)?\n    return result.nome, nil\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_programa_multiplos_items() {
        let src = "fn greeting() {\n    return \"Hello, World!\"\n}\n\nroute GET /hello {\n    return greeting()\n}\n";
        let prog = parse(src);
        let formatted = format_program(&prog);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_comentarios() {
        let src = "// esta funcao faz algo\nfn hello() {\n    return \"oi\"\n}\n";
        let prog = parse(src);
        let formatted = format_program_with_source(&prog, src);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_comentarios_entre_funcoes() {
        let src = "fn a() {\n    return 1\n}\n\n// separador\nfn b() {\n    return 2\n}\n";
        let prog = parse(src);
        let formatted = format_program_with_source(&prog, src);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_sem_comentarios_mantem_igual() {
        let src = "fn a() {\n    return 1\n}\n\nfn b() {\n    return 2\n}\n";
        let prog = parse(src);
        let formatted = format_program_with_source(&prog, src);
        assert_eq!(formatted, src);
    }
}
