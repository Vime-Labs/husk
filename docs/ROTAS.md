# Rotas

Rotas definem endpoints HTTP. O servidor usa `chi` por baixo.

## Sintaxe

```husk
route MÉTODO /caminho {
    // corpo
}
```

Rotas também podem receber middlewares e contexto tipado:

```husk
route MÉTODO /caminho [middleware1, middleware2] -> ctx {
    // corpo com ctx.field tipado
}
```

## Métodos suportados

`GET`, `POST`, `PUT`, `PATCH`, `DELETE`

## Porta

A porta do servidor é configurada pela variável de ambiente `PORT`:

```sh
PORT=3000 husk run main.husk
```

O padrão é `:8080`. O auto-prefixo `:` é adicionado se não estiver presente.

---

## Exemplos

### Rota simples

```husk
route GET /hello {
    return "Hello!"
}
```

### Parâmetro de caminho

Use `:nome` para capturar segmentos dinâmicos.

```husk
route GET /users/:id {
    return id
}
```

### Parâmetro tipado

Declare o tipo entre `< >` para receber o valor já convertido:

```husk
route GET /api/clientes/:id<int> {
    return json({ id: req.params.id })
}

route GET /medidas/:valor<float> {
    return json({ medido: req.params.valor })
}
```

Sem anotação, o parâmetro é `string`.

### Retorno em JSON

Toda expressão retornada em rota é serializada como JSON automaticamente:

```husk
route GET /ping {
    return { status: "ok" }
}
```

Gera `Content-Type: application/json` automaticamente.

### Retorno com status HTTP

```husk
route DELETE /item {
    return status(204)
}

route POST /item {
    return status(400, { erro: "campo ausente" })
}
```

### Retorno de texto simples

```husk
route GET /healthz {
    return text("ok")
}
```

## Funções de resposta

| Husk                           | Comportamento                          |
|--------------------------------|----------------------------------------|
| `return expr`                  | serializa como JSON                    |
| `return { ... }`               | serializa como JSON                    |
| `return text("...")`           | texto puro                             |
| `return status(N)`             | status HTTP sem corpo                  |
| `return status(N, expr)`       | status + body JSON                     |

## Variáveis implícitas

Dentro de uma rota, `req` está disponível automaticamente.

### Parâmetros de path

```husk
route GET /users/:id {
    let id = req.params.id
    return id
}
```

### Headers

```husk
route GET /secure {
    let token = req.headers["Authorization"]
    return token
}
```

### Query string

```husk
route GET /busca {
    let q = req.query["q"]
    return q
}
```

### Body (JSON)

Disponível em `POST`, `PUT`, `PATCH`.

```husk
route POST /login {
    let email = req.body["email"]
    let senha = req.body["senha"]
    return json({ ok: true })
}
```

## Middlewares por rota

```husk
route GET /perfil [autenticado] {
    return "dados do perfil"
}

route GET /admin [autenticado, admin] {
    return "painel admin"
}
```

## CORS

Configure CORS globalmente com o bloco `cors`:

```husk
cors {
    origins: ["*"]
    methods: ["GET", "POST"]
    headers: ["Authorization", "Content-Type"]
}
```

Campos disponíveis: `origins`, `methods`, `headers`. Gera middleware chi para todas as rotas.

## Verificação de role com `require_role`

```husk
route GET /admin [autenticado] -> ctx {
    require_role(ctx.role, "master")
    return "painel admin"
}
```

### Contexto tipado via `-> ctx`

```husk
route GET /api/clientes [autenticado] -> ctx {
    require_role(ctx.role, "master")
    return json({ user_id: ctx.user_id })
}
```

## `require_field` — validação de campos obrigatórios

```husk
route POST /usuarios {
    require_field("nome")
    require_field("email")
    return status(201, { ok: true })
}
```

## Notas

- Todas as rotas são registradas no mesmo router e servidas na mesma porta.
- A porta é lida de `PORT` (env var) com fallback para `:8080`.
