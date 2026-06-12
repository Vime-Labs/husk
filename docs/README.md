# Documentação Husk

Referência da linguagem Husk. Cada arquivo cobre uma parte da linguagem.

| Documento                            | Conteúdo                                                   | Status  |
|--------------------------------------|------------------------------------------------------------|---------|
| [FUNCOES.md](FUNCOES.md)             | Definição, parâmetros, retorno, built-ins de conversão     | ✓ v1.0  |
| [ROTAS.md](ROTAS.md)                 | HTTP, timeout, rate_limit, CORS, graceful shutdown, panic  | ✓ v1.1  |
| [TIPOS.md](TIPOS.md)                 | Primitivos, structs, map, `[]tipo`, conversão de tipos     | ✓ v1.0  |
| [EXPRESSOES.md](EXPRESSOES.md)       | Operadores, if/else, for...in, try/catch, retry, circuit breaker, literais | ✓ v1.1  |
| [MODULOS.md](MODULOS.md)             | import, alias, stdlib, vendor, ciclo detection             | ✓ v1.0  |
| [MIDDLEWARES.md](MIDDLEWARES.md)     | Definição, `-> ctx`, contexto tipado, next()               | ✓ v0.6  |
| [ERROS.md](ERROS.md)                 | (valor, error), `erro()`, let multi, `?`, err.message, circuit breaker     | ✓ v1.1  |
| [STRUCTS.md](STRUCTS.md)             | Definição, instanciação, acesso a campos, JSON             | ✓ v0.3  |
| [SCHEMA.md](SCHEMA.md)               | Schema, validadores, validate(req.body), tipos, min/max    | ✓ v3.1  |
| [MODEL.md](MODEL.md)                 | Model/ORM, table, CRUD, Find/Where/Insert/Update/Delete    | ✓ v3.2  |
| [TESTES.md](TESTES.md)               | `husk test`, `assert_eq()`, descoberta de testes           | ✓ v1.0  |
| [CLI.md](CLI.md)                     | run/dev/build/test/check/fmt/add/install/new/lsp, source maps | ✓ v1.0  |
| [ANALISADOR.md](ANALISADOR.md)       | Análise semântica, built-ins, verificação de tipos/escopo  | ✓ v1.0  |
| [MIGRATIONS.md](MIGRATIONS.md)       | Migrations com goose, `husk migrate` commands              | ✓ v1.0  |
| [EDITORES.md](EDITORES.md)           | Extensão VS Code, LSP, syntax highlighting, GitHub Linguist | ✓ v0.8  |

## Módulos stdlib

| Módulo          | Descrição                                     |
|-----------------|-----------------------------------------------|
| `husk/env`      | Variáveis de ambiente, `.env` loading         |
| `husk/postgres` | PostgreSQL via pgx (queries, UUID automático) |
| `husk/crypto`   | bcrypt, HMAC-SHA256                           |
| `husk/jwt`      | Criação e verificação de JWT                  |
| `husk/log`      | Logging com níveis (debug, info, warn, error) |
| `husk/http`     | Cliente HTTP com suporte a multipart          |

## Dependências externas

Projetos podem declarar dependências em `husk.json` e instalá-las com `husk install`.
Os packages são clonados via git para `vendor/` com resolução transitiva de dependências.
