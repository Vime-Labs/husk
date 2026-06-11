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

Leitura de variáveis de ambiente.

| Função                       | Descrição                                           |
|------------------------------|-----------------------------------------------------|
| `env.get(key)`               | Retorna o valor da variável ou `""` se não definida |
| `env.get_or(key, fallback)`  | Retorna o valor ou `fallback` se não definida       |
| `env.require(key)`           | Retorna o valor; pânico se a variável não existir   |

```husk
import "husk/env" as env

route GET /config {
    let porta = env.get_or("PORT", "8080")
    return json({ porta: porta })
}
```

### husk/postgres

Conexão e queries para PostgreSQL via pgx.

A conexão é estabelecida automaticamente a partir da variável `DATABASE_URL`. Não é necessário chamar `db.connect()` manualmente na maioria dos casos.

| Função                       | Retorno           | Descrição                         |
|------------------------------|-------------------|-----------------------------------|
| `db.connect(url)`            | `error`           | Conecta explicitamente            |
| `db.query(sql, args...)`     | `([]map, error)`  | Retorna todas as linhas           |
| `db.query_one(sql, args...)` | `(map, error)`    | Retorna a primeira linha          |
| `db.exec(sql, args...)`      | `error`           | Executa sem retornar linhas       |

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

Hashing e verificação de senhas com bcrypt.

| Função                       | Retorno           | Descrição                   |
|------------------------------|-------------------|-----------------------------|
| `crypto.hash(senha)`         | `(string, error)` | Gera hash bcrypt            |
| `crypto.verify(senha, hash)` | `bool`            | Compara senha com hash      |

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

---

## Importações circulares

Não são permitidas em módulos do projeto. O transpiler detecta ciclos e interrompe com erro.
