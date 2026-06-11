# Documentação Husk

Referência da linguagem Husk. Cada arquivo cobre uma parte da linguagem.

| Documento                            | Conteúdo                                                   | Status  |
|--------------------------------------|------------------------------------------------------------|---------|
| [FUNCOES.md](FUNCOES.md)             | Definição, parâmetros, retorno, built-ins de conversão     | ✓ v1.0  |
| [ROTAS.md](ROTAS.md)                 | HTTP, timeout, rate_limit, CORS, graceful shutdown, panic  | ✓ v1.1  |
| [TIPOS.md](TIPOS.md)                 | Primitivos, structs, map, `[]tipo`, conversão de tipos     | ✓ v1.0  |
| [EXPRESSOES.md](EXPRESSOES.md)       | Operadores, if/else, for...in, try/catch, retry, circuit breaker, literais | ✓ v1.1  |
| [MODULOS.md](MODULOS.md)             | import, alias, stdlib, ciclo detection                     | ✓ v1.0  |
| [MIDDLEWARES.md](MIDDLEWARES.md)     | Definição, `-> ctx`, contexto tipado, next()               | ✓ v0.6  |
| [ERROS.md](ERROS.md)                 | (valor, error), `erro()`, let multi, `?`, err.message, circuit breaker     | ✓ v1.1  |
| [STRUCTS.md](STRUCTS.md)             | Definição, instanciação, acesso a campos, JSON             | ✓ v0.3  |
| [SCHEMA.md](SCHEMA.md)               | Schema, validadores, validate(req.body), tipos, min/max    | ✓ v3.1  |
| [TESTES.md](TESTES.md)               | `husk test`, `assert_eq()`, descoberta de testes           | ✓ v1.0  |
| [CLI.md](CLI.md)                     | run/dev/build/test/check/fmt/add/new/lsp, source maps      | ✓ v1.0  |
| [ANALISADOR.md](ANALISADOR.md)       | Análise semântica, built-ins, verificação de tipos/escopo  | ✓ v1.0  |
