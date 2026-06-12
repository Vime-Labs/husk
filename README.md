# Husk

**Husk** é uma DSL (Domain Specific Language) para construção de **APIs REST** em Go. O código Husk é transpilado para Go puro, compilado com a toolchain nativa, e produz binários eficientes e estáticos.

```husk
route GET /ping {
    return json({ status: "ok" })
}
```

---

## Porquê uma DSL?

Go é excelente para construir APIs, mas o código CRUD acaba sempre no mesmo padrão — importar router, importar DB, escrever handlers, ligar middlewares, tratar erros, fazer parse de JSON. Husk elimina esta cerimónia com uma sintaxe declarativa e tipada, mantendo a interoperabilidade total com Go.

| Problema | Como Husk resolve |
|---|---|
| Boilerplate de rotas | `route GET /path { ... }` — sem handlers, sem router setup |
| Tratamento de erros | Operador `?` propaga erros com status HTTP |
| Middleware chaining | `[auth, logger]` inline nas rotas |
| Contexto tipado | `-> ctx` com struct gerada automaticamente |
| Validação de entrada | `schema` + `validate()` nativos |
| ORM | `model` gera CRUD automaticamente |

## Exemplo real

Um CRUD completo de utilizadores com autenticação JWT, validação e migração:

```husk
import "husk/postgres" as db
import "husk/jwt" as jwt

middleware autenticado -> ctx {
    let token = req.headers["Authorization"]
    let claims = jwt.verify(token)?
    ctx.usuario_id = claims.sub
    next()
}

route GET /usuarios [autenticado] -> ctx {
    let dados = db.query("SELECT id, nome, email FROM usuarios")?
    return json(dados)
}

route POST /usuarios [autenticado, timeout=10s] -> ctx {
    let dados = validate(req.body, UsuarioSchema)?
    let id = db.insert("usuarios", dados)?
    return status(201) + json({ id: id })
}

route GET /usuarios/:id<int> [autenticado] -> ctx {
    let usuario = db.query_one(
        "SELECT id, nome, email FROM usuarios WHERE id = ?",
        req.params.id
    )? 404 "Usuario nao encontrado"
    return json(usuario)
}
```

## Como funciona

```
.husk → [ Lexer → Parser → Analisador → Codegen ] → .go → go build → binário
```

O transpiler é escrito em Rust (dividido em 4 crates independentes) e produz código Go idiomático usando o router [`chi`](https://github.com/go-chi/chi). O desenvolvedor nunca vê o código Go gerado — erros de compilação são traduzidos automaticamente para as linhas `.husk` originais via source maps.

### Binário único e enxuto

```
$ husk build server.husk
$ ./server
```

Sem runtime, sem VM, sem dependencies. Apenas um binário Go compilado estaticamente.

## Documentação

| Documento | Conteúdo |
|---|---|
| [ARQUITETURA.md](ARQUITETURA.md) | Design decisions, escopo, limites da DSL |
| [ROADMAP.md](ROADMAP.md) | Estado actual e próximos passos |
| [docs/](docs/) | Referência completa da linguagem (rotas, funções, tipos, erros, middlewares, schemas, models, testes, CLI, LSP) |

## Instalação

```bash
# Compilar o transpiler (precisa de Rust)
git clone https://github.com/Vime-Labs/husk
cd husk
make install
```

Depois:

```bash
husk new meu-projeto
cd meu-projeto
husk dev main.husk
```

## Dependências externas

Declare pacotes em `husk.json` e instale com `husk install`:

```json
{
  "name": "meu-app",
  "dependencies": {
    "framework": {
      "git": "https://github.com/vime/husk-framework",
      "ref": "v0.1.0"
    }
  }
}
```

```bash
husk install          # clona para vendor/ + resolve transitivas
husk get <git-url>    # adiciona ao husk.json e instala
```

Os packages são incluídos automaticamente em tempo de compilação — sem alterações no código.

## Módulos stdlib

| Módulo          | Descrição                                     |
|-----------------|-----------------------------------------------|
| `husk/env`      | Variáveis de ambiente, `.env` loading         |
| `husk/postgres` | PostgreSQL via pgx (UUID→string automático)   |
| `husk/crypto`   | bcrypt + HMAC-SHA256                          |
| `husk/jwt`      | Criação e verificação de JWT (HS256 + RS256)  |
| `husk/log`      | Logging com níveis                            |
| `husk/http`     | Cliente HTTP com suporte a multipart          |
| `husk/s3`       | Object storage S3 (minio-go, Signature V4)    |

## Estado

Husk está em uso em produção na Vime Sistemas para construção de APIs REST financeiras. A linguagem é considerada estável para o seu escopo actual e recebe melhorias contínuas.

### Suporte do GitHub Linguist

Husk tem suporte de syntax highlighting no GitHub via TextMate grammar, pendente de aceitação no [Linguist](https://github.com/github-linguist/linguist).

- Grammar: [Vime-Labs/husk-grammar](https://github.com/Vime-Labs/husk-grammar)
- Extension: `.husk`
- Scope: `source.husk`

## Licença

MIT
