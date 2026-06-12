# Módulos

Husk tem dois tipos de módulo: **módulos do projeto** (arquivos `.husk`) e **módulos da stdlib** (prefixo `husk/`).

---

## Módulos do projeto

Cada arquivo `.husk` é um módulo. Módulos exportam funções e structs para outros arquivos via `import`.

```husk
import "./caminho/do/modulo" as alias
```

O caminho é relativo ao arquivo que está importando. A extensão `.husk` é opcional.

### Exemplo

`usuarios.husk`:
```husk
struct Usuario {
    id   int
    nome string
}

fn listar() string {
    return "João, Maria, Pedro"
}
```

`main.husk`:
```husk
import "./usuarios" as usuarios

route GET /usuarios {
    return usuarios.listar()
}
```

Chamadas com alias de módulo do projeto têm o prefixo removido: `usuarios.listar()` → `listar()` no Go gerado.

### O que é exportado

| Construto     | Exportado? |
|---------------|------------|
| `fn`          | sim        |
| `struct`      | sim        |
| `route`       | não — rotas só existem no arquivo raiz |
| `middleware`  | não — middlewares só existem no arquivo raiz |
| `import`      | não — imports não são transitivos |

---

## Módulos da stdlib

A stdlib fornece adaptadores prontos para as tarefas mais comuns em web servers.

```husk
import "husk/env"      as env
import "husk/postgres" as db
import "husk/crypto"   as crypto
```

Chamadas com alias da stdlib **mantêm o prefixo**: `env.get("PORT")` → `env_get("PORT")` no Go gerado. Isso permite que várias stdlib coexistam sem colisão de nomes.

### husk/env

Leitura de variáveis de ambiente e loading de arquivos `.env`.

| Função                       | Descrição                                           |
|------------------------------|-----------------------------------------------------|
| `env.get(key)`               | Retorna o valor da variável ou `""` se não definida |
| `env.get_or(key, fallback)`  | Retorna o valor ou `fallback` se não definida       |
| `env.require(key)`           | Retorna o valor; pânico se a variável não existir   |
| `env.load(paths...)`         | Carrega `.env` de um ou mais arquivos               |

```husk
import "husk/env" as env

// Loading explícito (o main() já carrega .env automaticamente)
env.load(".env", "config/.env")

route GET /config {
    let porta = env.get_or("PORT", "8080")
    return json({ porta: porta })
}
```

> O `main()` gerado já carrega `.env` e `backend/.env` automaticamente na inicialização.
> Use `env.load()` quando precisar de arquivos adicionais ou controle explícito.

### husk/postgres

Conexão e queries para PostgreSQL via pgx.

A conexão **não é automática** — chame `db.connect()` explicitamente com a URL
que preferir (geralmente vinda de `env.require()`):

```husk
import "husk/postgres" as db
import "husk/env" as env

let url = env.require("DATABASE_URL")
let err = db.connect(url)
if err != nil {
    return status(500, json({ erro: "falha ao conectar: " + err.message }))
}
```

| Função                       | Retorno           | Descrição                         |
|------------------------------|-------------------|-----------------------------------|
| `db.connect(url)`            | `error`           | Conecta ao banco                  |
| `db.query(sql, args...)`     | `([]map, error)`  | Retorna todas as linhas           |
| `db.query_one(sql, args...)` | `(map, error)`    | Retorna a primeira linha          |
| `db.exec(sql, args...)`      | `(interface{}, error)` | Executa sem retornar linhas. Use com `?`: `let _ = db.exec(sql, args...)?` |

```husk
import "husk/postgres" as db

route GET /usuarios {
    let rows, err = db.query("SELECT id, nome FROM usuarios")
    if err != nil {
        return status(500, json({ erro: err.message }))
    }
    return json(rows)
}
```

### husk/crypto

Hashing e verificação de senhas com bcrypt. Suporte a HMAC-SHA256 para verificação de webhooks.

| Função                       | Retorno           | Descrição                   |
|------------------------------|-------------------|-----------------------------|
| `crypto.hash(senha)`         | `(string, error)` | Gera hash bcrypt            |
| `crypto.verify(senha, hash)` | `bool`            | Compara senha com hash      |
| `crypto.hmac_sha256(key, data)` | `string`       | Gera assinatura HMAC-SHA256 hex |
| `crypto.equal(a, b)`         | `bool`            | Comparação em tempo constante (seguro contra timing attacks) |

```husk
import "husk/crypto" as crypto

route POST /usuarios {
    let hash, err = crypto.hash("senha123")
    if err != nil {
        return status(500, json({ erro: err.message }))
    }
    return json({ hash: hash })
}
```

### husk/jwt

Geração e verificação de JSON Web Tokens com HMAC-SHA256.

| Função                       | Retorno           | Descrição                                                        |
|------------------------------|-------------------|------------------------------------------------------------------|
| `jwt.sign(payload, secret)`  | `(string, error)` | Cria um JWT assinado. Adiciona `exp` de 24h se não informado     |
| `jwt.verify(token, secret)`  | `(map, error)`    | Verifica a assinatura e retorna os claims como mapa              |

```husk
import "husk/jwt"  as jwt
import "husk/env"  as env

fn token_novo(user_id int) (string, error) {
    let secret = env.require("JWT_SECRET")
    return jwt.sign({ user_id: user_id }, secret)
}

route POST /login {
    let secret = env.require("JWT_SECRET")
    let token, err = jwt.sign({ user_id: 42, role: "admin" }, secret)
    if err != nil {
        return status(500, json({ erro: err.message }))
    }
    return json({ token: token })
}

route GET /perfil {
    let secret = env.require("JWT_SECRET")
    let raw = req.headers["Authorization"]
    let claims, err = jwt.verify(raw, secret)
    if err != nil {
        return status(401, json({ erro: "token inválido" }))
    }
    return json(claims)
}
```

### husk/log

Logging com níveis usando o pacote `log` do Go.

| Função               | Descrição                             |
|----------------------|---------------------------------------|
| `log.debug(msg)`     | Log nível DEBUG                       |
| `log.info(msg)`      | Log nível INFO                        |
| `log.warn(msg)`      | Log nível WARN                        |
| `log.error(msg)`     | Log nível ERROR                       |
| `log.fatal(msg)`     | Log nível FATAL e encerra o programa  |

```husk
import "husk/log" as log

route GET /ping {
    log.info("ping recebido")
    return text("pong")
}
```

A saída vai para stderr com timestamp e nível:

```
2026/06/12 10:15:30 [INFO] ping recebido
```

### husk/http

Requisições HTTP para APIs externas. Usa `net/http` do Go — sem dependências externas.

| Função                       | Retorno                | Descrição                          |
|------------------------------|------------------------|------------------------------------|
| `http.get(url, opts?)`       | `(*httpResponse, error)` | Requisição GET                   |
| `http.post(url, body, opts?)` | `(*httpResponse, error)` | Requisição POST com body JSON   |
| `http.put(url, body, opts?)`  | `(*httpResponse, error)` | Requisição PUT com body JSON    |
| `http.patch(url, body, opts?)` | `(*httpResponse, error)` | Requisição PATCH com body JSON  |
| `http.delete(url, opts?)`     | `(*httpResponse, error)` | Requisição DELETE               |

O retorno `httpResponse` tem os campos:
- `resp.status` — código HTTP (`int`)
- `resp.body` — corpo da resposta (`string`)
- `resp.headers` — cabeçalhos (`map[string]string`)

```husk
import "husk/http" as http

route GET /proxy {
    let resp, err = http.get("https://api.example.com/dados", {
        headers: { Authorization: "Bearer " + token },
        timeout: 10
    })
    if err != nil {
        return status(502, json({ erro: err.message }))
    }
    return json({ data: resp.body, status: resp.status })
}
```

**Suporte a multipart/form-data** para upload de ficheiros:

```husk
http.post("https://api.groq.com/v1/audio/transcriptions", {
    headers: { Authorization: "Bearer " + key },
    multipart: {
        model: "whisper-large-v3",
        file: {
            path: "/tmp/audio.mp3",
            filename: "audio.mp3"
        }
    }
})
```

Opções disponíveis:

| Opção      | Tipo                    | Descrição                          |
|------------|-------------------------|------------------------------------|
| `headers`  | `map[string]string`     | Cabeçalhos HTTP                    |
| `query`    | `map[string]string`     | Query string params                |
| `timeout`  | `int`                   | Timeout em segundos                |
| `multipart`| `map[string](string\|{path, filename})` | Multipart form-data |

---

## Importações circulares

Não são permitidas em módulos do projeto. O transpiler detecta ciclos e interrompe com erro.

---

## Dependências externas (vendor/)

O Husk suporta pacotes externos via git, gerenciados pelo comando `husk install`.

### Manifesto (`husk.json`)

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

### Fluxo

```sh
husk install            # clona para vendor/ + resolve transitivas
husk install --force    # reinstala mesmo se vendor/ já existir
husk get <git-url>      # adiciona ao husk.json e instala
```

O comando:
1. Lê `husk.json`
2. Clona cada dependência para `vendor/<nome>/` com `git clone --depth 1 --branch <ref>`
3. Se o `ref` falhar como branch, tenta como commit hash
4. Resolve dependências transitivas (cada package pode ter o seu `husk.json`)
5. Gera `.vendor.husk` com os imports necessários

### Estrutura gerada

```
vendor/
├── framework/
│   ├── main.husk
│   └── husk.json        (dependências transitivas)
└── outro/
    └── lib.husk

.vendor.husk              (auto-gerado, incluído em tempo de compilação)
```

O `.vendor.husk` é incluído automaticamente pelo transpiler — o utilizador não precisa de o importar manualmente.

### Entry points

O `husk install` procura o ponto de entrada de cada package nesta ordem:
1. `main.husk`
2. `mod.husk`
3. `lib.husk`
4. Primeiro `.husk` encontrado

### `.gitignore`

Projetos criados com `husk new` já incluem `vendor/` no `.gitignore`. Espera-se que as dependências sejam rastreadas pelo `husk.json` + `husk.lock` (futuro), não versionando o `vendor/`.
