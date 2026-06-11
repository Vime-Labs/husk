mod checker;
mod scope;

pub use checker::Checker;
pub use scope::{FnSignature, Scope, SemanticError, Symbol, TypeInfo};

use husk_parser::ast::Program;

/// Função de conveniência: analisa um programa completo e retorna os erros
pub fn analyze(program: &Program) -> Vec<SemanticError> {
    let mut checker = Checker::new();
    let errors = checker.check(program);
    errors.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use husk_lexer::Lexer;
    use husk_parser::Parser;

    fn analyze_src(src: &str) -> Vec<SemanticError> {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        analyze(&program)
    }

    fn analyze_src_ok(src: &str) {
        let errors = analyze_src(src);
        if !errors.is_empty() {
            panic!("esperado nenhum erro semântico, encontrado:\n{:#?}", errors);
        }
    }

    // ---- Casos de sucesso ----

    #[test]
    fn test_fn_simples() {
        analyze_src_ok(
            r#"
fn soma(a int, b int) int {
    return a + b
}
route GET /soma {
    return soma(1, 2)
}
"#,
        );
    }

    #[test]
    fn test_route_com_json() {
        analyze_src_ok(
            r#"
route GET /ping {
    return json({ status: "ok" })
}
"#,
        );
    }

    #[test]
    fn test_route_com_params() {
        analyze_src_ok(
            r#"
route GET /users/:id {
    return id
}
"#,
        );
    }

    #[test]
    fn test_middleware_simples() {
        analyze_src_ok(
            r#"
middleware logger {
    next()
}
route GET /users [logger] {
    return "ok"
}
"#,
        );
    }

    #[test]
    fn test_struct_def() {
        analyze_src_ok(
            r#"
struct Usuario {
    id int
    nome string
}
fn criar() Usuario {
    return Usuario { id: 1, nome: "João" }
}
"#,
        );
    }

    #[test]
    fn test_let_com_tipos() {
        analyze_src_ok(
            r#"
fn f() {
    let x = 42
    let nome string = "João"
    let y = x + 1
}
"#,
        );
    }

    #[test]
    fn test_if_simples() {
        analyze_src_ok(
            r#"
fn f(x int) int {
    if x > 0 {
        return 1
    } else {
        return 0
    }
}
"#,
        );
    }

    #[test]
    fn test_concatenacao_string() {
        analyze_src_ok(
            r#"
fn saudacao(nome string) string {
    return "Olá, " + nome
}
"#,
        );
    }

    #[test]
    fn test_multi_retorno() {
        analyze_src_ok(
            r#"
fn dividir(a int, b int) (int, error) {
    return a / b, nil
}
fn usar() {
    let res, err = dividir(10, 2)
}
"#,
        );
    }

    #[test]
    fn test_req_access() {
        analyze_src_ok(
            r#"
route GET /users/:id {
    let id = req.params.id
    let token = req.headers["Authorization"]
    let q = req.query["q"]
    return id
}
"#,
        );
    }

    // ---- Casos de erro ----

    #[test]
    fn test_variavel_nao_declarada() {
        let errors = analyze_src("fn f() { return x }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("não foi declarado"));
    }

    #[test]
    fn test_funcao_nao_definida() {
        let errors = analyze_src("fn f() { return inexistente() }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("não definida"));
    }

    #[test]
    fn test_tipo_incompativel_let() {
        let errors = analyze_src(r#"fn f() { let x int = "string" }"#);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("esperado tipo"));
    }

    #[test]
    fn test_next_fora_de_middleware() {
        let errors = analyze_src("fn f() { next() }");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("middleware"));
    }

    #[test]
    fn test_req_fora_de_rota() {
        let errors = analyze_src(r#"fn f() { return req.params.id }"#);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_middleware_nao_definido() {
        let errors = analyze_src(r#"route GET /x [inexistente] { return "ok" }"#);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("não definido"));
    }

    #[test]
    fn test_duplicata_funcao() {
        let errors = analyze_src("fn a() {} fn a() {}");
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("já foi declarado"));
    }

    #[test]
    fn test_if_condicao_nao_bool() {
        let errors = analyze_src(r#"fn f() { if "string" { return 1 } }"#);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("bool"));
    }

    #[test]
    fn test_tipos_incompativeis_binop() {
        let errors = analyze_src(r#"fn f() { return true + 1 }"#);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_struct_campo_inexistente() {
        let errors = analyze_src(
            r#"
struct A { x int }
fn f() A { return A { y: 1 } }
"#,
        );
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("não tem campo"));
    }

    #[test]
    fn test_bool_comparacao() {
        // bool == bool é válido
        analyze_src_ok(
            r#"
fn f(a bool, b bool) bool {
    return a == b
}
"#,
        );
    }

    #[test]
    fn test_numero_negativo() {
        analyze_src_ok(
            r#"
fn f(x int) int {
    return -x
}
"#,
        );
    }

    #[test]
    fn test_not_bool() {
        analyze_src_ok(
            r#"
fn f(a bool) bool {
    return !a
}
"#,
        );
    }

    #[test]
    fn test_not_nao_bool() {
        let errors = analyze_src("fn f() { return !42 }");
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_erro_message() {
        analyze_src_ok(
            r#"
fn dividir(a int, b int) (int, error) {
    return a / b, nil
}
fn usar() {
    let res, err = dividir(10, 0)
    if err != nil {
        return err.message
    }
}
"#,
        );
    }

    #[test]
    fn test_programa_completo_example() {
        analyze_src_ok(
            r#"
fn greeting() {
    return "Hello, World!"
}
route GET /hello {
    return greeting()
}
"#,
        );
    }

    // ---- for...in ----

    #[test]
    fn test_for_in_lista() {
        analyze_src_ok(
            r#"
fn f(items []string) {
    for item in items {
        return item
    }
}
"#,
        );
    }

    #[test]
    fn test_for_in_nao_iteravel() {
        let errors = analyze_src(
            r#"
fn f(x int) {
    for item in x {
        return item
    }
}
"#,
        );
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("lista ou map"));
    }
}
