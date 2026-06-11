pub mod ast;
mod parser;

pub use parser::{Parser, ParseError};

#[cfg(test)]
mod tests {
    use husk_lexer::Lexer;
    use crate::{Parser, ast::*};

    fn parse(src: &str) -> Program {
        let tokens = Lexer::new(src).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn test_fn_simples() {
        let prog = parse(r#"
fn greeting() {
    return "Hello, World!"
}
"#);
        assert_eq!(prog.items.len(), 1);
        let Item::FnDef(f) = &prog.items[0] else { panic!("esperado FnDef") };
        assert_eq!(f.name, "greeting");
        assert!(f.params.is_empty());
        assert_eq!(f.body.stmts.len(), 1);
    }

    #[test]
    fn test_fn_com_params() {
        let prog = parse("fn soma(a int, b int) int { return a }");
        let Item::FnDef(f) = &prog.items[0] else { panic!() };
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert!(matches!(f.params[0].ty, Type::Int));
        assert!(matches!(f.return_type, ReturnType::Single(Type::Int)));
    }

    #[test]
    fn test_route_get_simples() {
        let prog = parse(r#"
route GET /hello {
    return greeting()
}
"#);
        let Item::RouteDef(r) = &prog.items[0] else { panic!("esperado RouteDef") };
        assert_eq!(r.method, HttpMethod::Get);
        assert_eq!(r.path.to_string(), "/hello");
    }

    #[test]
    fn test_route_com_param() {
        let prog = parse("route GET /users/:id { return id }");
        let Item::RouteDef(r) = &prog.items[0] else { panic!() };
        assert_eq!(r.path.to_string(), "/users/{id}");
    }

    #[test]
    fn test_programa_completo() {
        let prog = parse(r#"
fn greeting() {
    return "Hello, World!"
}
route GET /hello {
    return greeting()
}
"#);
        assert_eq!(prog.items.len(), 2);
        assert!(matches!(prog.items[0], Item::FnDef(_)));
        assert!(matches!(prog.items[1], Item::RouteDef(_)));
    }

    #[test]
    fn test_let_stmt() {
        let prog = parse("fn f() { let x = 42 }");
        let Item::FnDef(f) = &prog.items[0] else { panic!() };
        assert!(matches!(f.body.stmts[0], Stmt::Let(_)));
    }

    #[test]
    fn test_call_expr() {
        let prog = parse("fn f() { return soma(1, 2) }");
        let Item::FnDef(f) = &prog.items[0] else { panic!() };
        let Stmt::Return(exprs) = &f.body.stmts[0] else { panic!() };
        let Expr::Call(call) = &exprs[0] else { panic!() };
        assert!(matches!(*call.callee, Expr::Ident(ref n) if n == "soma"));
        assert_eq!(call.args.len(), 2);
    }
}
