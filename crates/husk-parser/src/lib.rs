pub mod ast;
pub mod formatter;
mod parser;

pub use parser::{ParseError, Parser};

#[cfg(test)]
mod tests {
    use crate::{Parser, ast::*};
    use husk_lexer::Lexer;

    fn parse(src: &str) -> Program {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn test_fn_simples() {
        let prog = parse(
            r#"
fn greeting() {
    return "Hello, World!"
}
"#,
        );
        assert_eq!(prog.items.len(), 1);
        let Item::FnDef(f) = &prog.items[0] else {
            panic!("esperado FnDef")
        };
        assert_eq!(f.name, "greeting");
        assert!(f.params.is_empty());
        assert_eq!(f.body.stmts.len(), 1);
    }

    #[test]
    fn test_fn_com_params() {
        let prog = parse("fn soma(a int, b int) int { return a }");
        let Item::FnDef(f) = &prog.items[0] else {
            panic!()
        };
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert!(matches!(f.params[0].ty, Type::Int));
        assert!(matches!(f.return_type, ReturnType::Single(Type::Int)));
    }

    #[test]
    fn test_route_get_simples() {
        let prog = parse(
            r#"
route GET /hello {
    return greeting()
}
"#,
        );
        let Item::RouteDef(r) = &prog.items[0] else {
            panic!("esperado RouteDef")
        };
        assert_eq!(r.method, HttpMethod::Get);
        assert_eq!(r.path.to_string(), "/hello");
    }

    #[test]
    fn test_route_com_param() {
        let prog = parse("route GET /users/:id { return id }");
        let Item::RouteDef(r) = &prog.items[0] else {
            panic!()
        };
        assert_eq!(r.path.to_string(), "/users/{id}");
    }

    #[test]
    fn test_programa_completo() {
        let prog = parse(
            r#"
fn greeting() {
    return "Hello, World!"
}
route GET /hello {
    return greeting()
}
"#,
        );
        assert_eq!(prog.items.len(), 2);
        assert!(matches!(prog.items[0], Item::FnDef(_)));
        assert!(matches!(prog.items[1], Item::RouteDef(_)));
    }

    #[test]
    fn test_let_stmt() {
        let prog = parse("fn f() { let x = 42 }");
        let Item::FnDef(f) = &prog.items[0] else {
            panic!()
        };
        assert!(matches!(f.body.stmts[0], Stmt::Let(_)));
    }

    #[test]
    fn test_call_expr() {
        let prog = parse("fn f() { return soma(1, 2) }");
        let Item::FnDef(f) = &prog.items[0] else {
            panic!()
        };
        let Stmt::Return(exprs) = &f.body.stmts[0] else {
            panic!()
        };
        let Expr::Call(call) = &exprs[0] else {
            panic!()
        };
        assert!(matches!(*call.callee, Expr::Ident(ref n) if n == "soma"));
        assert_eq!(call.args.len(), 2);
    }

    #[test]
    fn test_if_com_negacao_nao_e_struct_init() {
        // Bug: `if !ok {` era interpretado como struct init `ok { ... }`
        let prog = parse(
            r#"
fn f() {
    if !ok {
        return 1
    }
}
"#,
        );
        assert_eq!(prog.items.len(), 1);
        let Item::FnDef(f) = &prog.items[0] else {
            panic!()
        };
        assert!(matches!(f.body.stmts[0], Stmt::If(_)));
    }

    #[test]
    fn test_struct_init_ainda_funciona() {
        // O lookahead não deve quebrar struct init legítimo
        let prog = parse(
            r#"
struct S { x int }
fn f() S { return S { x: 1 } }
"#,
        );
        let Item::FnDef(f) = &prog.items[1] else {
            panic!()
        };
        let Stmt::Return(exprs) = &f.body.stmts[0] else {
            panic!()
        };
        assert!(matches!(exprs[0], Expr::StructInit(_)));
    }

    #[test]
    fn test_if_com_ident_simples() {
        // `if cond {` — cond termina com ident, não deve confundir com struct init
        let prog = parse(
            r#"
fn f(ok bool) {
    if ok {
        return 1
    }
}
"#,
        );
        let Item::FnDef(f) = &prog.items[0] else {
            panic!()
        };
        assert!(matches!(f.body.stmts[0], Stmt::If(_)));
    }

    #[test]
    fn test_type_conversion_calls() {
        let prog = parse(
            r#"
fn f() float {
    return float("3.14")
}
fn s() string {
    return string(42)
}
"#,
        );
        let Item::FnDef(f1) = &prog.items[0] else { panic!() };
        let Stmt::Return(exprs1) = &f1.body.stmts[0] else { panic!() };
        assert!(matches!(&exprs1[0], Expr::Call(c) if matches!(*c.callee.clone(), Expr::Ident(n) if n == "float")));

        let Item::FnDef(f2) = &prog.items[1] else { panic!() };
        let Stmt::Return(exprs2) = &f2.body.stmts[0] else { panic!() };
        assert!(matches!(&exprs2[0], Expr::Call(c) if matches!(*c.callee.clone(), Expr::Ident(n) if n == "string")));
    }

    #[test]
    fn test_for_in_simples() {
        let prog = parse(
            r#"
fn f() {
    for item in items {
        return item
    }
}
"#,
        );
        let Item::FnDef(f) = &prog.items[0] else {
            panic!()
        };
        assert!(matches!(f.body.stmts[0], Stmt::ForIn(_)));
    }

    #[test]
    fn test_for_in_com_json() {
        let prog = parse(
            r#"
route GET /list {
    for item in items {
        return json(item)
    }
}
"#,
        );
        let Item::RouteDef(r) = &prog.items[0] else {
            panic!()
        };
        assert!(matches!(r.body.stmts[0], Stmt::ForIn(_)));
    }

    #[test]
    fn test_schema_simples() {
        let prog = parse(
            r#"
schema Usuario {
    nome: string required
    email: string required email()
    idade: int min(18) max(120)
}
"#,
        );
        assert_eq!(prog.items.len(), 1);
        let Item::SchemaDef(s) = &prog.items[0] else {
            panic!("esperado SchemaDef")
        };
        assert_eq!(s.name, "Usuario");
        assert_eq!(s.fields.len(), 3);
        assert_eq!(s.fields[0].name, "nome");
        assert!(matches!(s.fields[0].ty, Type::String));
        assert_eq!(s.fields[0].validators.len(), 1);
        assert!(matches!(s.fields[0].validators[0], Validator::Required));
        assert!(matches!(s.fields[1].validators[1], Validator::Email));
        assert!(matches!(s.fields[2].validators[0], Validator::Min(18)));
        assert!(matches!(s.fields[2].validators[1], Validator::Max(120)));
    }
}
