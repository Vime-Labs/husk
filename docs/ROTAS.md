# Rotas

Rotas definem endpoints HTTP. O servidor usa `chi` por baixo e é iniciado automaticamente na porta `8080`.

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

## Exemplos

### Rota simples

```husk
route GET /hello {
    return "Hello!"
}
```

### Parâmetro de caminho

Use `:nome` para capturar segmentos dinâmicos. O valor fica disponível como variável no corpo.

```husk
route GET /users/:id {
    return id
}
```

O parâmetro `:id` é mapeado para `{id}` no chi e pode ser lido via `req.params.id` ou usado diretamente como variável.

### Parâmetro tipado

Declare o tipo após `:` entre `< >` para receber o valor já convertido:

```husk
route GET /api/clientes/:id<int> {
    // req.params.id já é int — sem parse_int()
    return json({ id: req.params.id })
}

route GET /medidas/:valor<float> {
    // req.params.valor já é float64
    return json({ medido: req.params.valor })
}
```

Sem a anotação de tipo, o parâmetro é `string` (comportamento padrão).

### Retorno em JSON

Toda expressão retornada em rota é serializada como JSON automaticamente:

```husk
route GET /ping {
    return { status: "ok" }
}
```

Gera os headers `Content-Type: application/json` automaticamente.

Objetos literais, structs, variáveis — tudo vira JSON sem precisar de `json()`.

### Retorno com status HTTP

```husk
route DELETE /item {
    return status(204)
}

route POST /item {
    return status(400, { erro: "campo ausente" })
}
```

O body do `status()` também é JSON automático — não precisa de `json()`.

### Retorno de texto simples

Use `text()` para resposta em texto puro (única exceção ao JSON automático):

```husk
route GET /healthz {
    return text("ok")
}
```

## Funções de resposta

| Husk                    | Comportamento                              |
|-------------------------|--------------------------------------------|
| `return expr`           | serializa qualquer expressão como JSON     |
| `return { ... }`        | serializa como JSON (atalho para expr)     |
| `return text("...")`    | escreve texto puro (única exceção)         |
| `return status(N)`      | define o status HTTP sem corpo             |
| `return status(N, expr)` | define status e serializa body como JSON  |

## Variáveis implícitas

Dentro de um bloco de rota, `req` está disponível automaticamente.

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

Disponível em rotas `POST`, `PUT` e `PATCH`. O body é decodificado automaticamente como JSON quando qualquer campo é acessado.

```husk
route POST /login {
    let email = req.body["email"]
    let senha = req.body["senha"]
    return json({ ok: true })
}
```

O Go gerado decodifica o body uma vez no início do handler:

```go
var _huskBody map[string]interface{}
json.NewDecoder(r.Body).Decode(&_huskBody)
```

Você também pode atribuir `req.body` a uma variável para reutilizar:

```husk
route POST /cadastro {
    let body = req.body
    if body["nome"] == "" {
        return status(400, { erro: "Nome obrigatório" })
    }
    let nome = body["nome"]
    let email = body["email"]
    return status(201, { id: 1 })
}
```

## Middlewares por rota

Aplique um ou mais middlewares declarando-os entre colchetes após o path:

```husk
route GET /perfil [autenticado] {
    return "dados do perfil"
}

route GET /admin [autenticado, admin] {
    return "painel admin"
}
```

Ver [MIDDLEWARES.md](MIDDLEWARES.md) para como definir middlewares.

### Verificação de role com `require_role`

Para rotas que exigem uma role específica, use `require_role()` no corpo da rota.
O primeiro argumento é o valor real (geralmente vindo do contexto), o segundo é o valor esperado:

```husk
route GET /admin [autenticado] -> ctx {
    require_role(ctx.role, "master")
    // só chega aqui se ctx.role == "master"
    return "painel admin"
}
```

Gera:
```go
if ctx_role != "master" {
    w.WriteHeader(403)
    json.NewEncoder(w).Encode(map[string]interface{}{"erro": "Acesso restrito"})
    return
}
```

Mensagem customizada (3º argumento):

```husk
require_role(ctx.role, "admin", "Só administradores")
```

### Contexto tipado via `-> ctx`

Quando uma rota recebe `-> ctx`, ela ganha acesso a um objeto `ctx` com campos tipados
setados pelos middlewares da cadeia. Isso substitui o padrão antigo
`set_ctx("chave", valor)` / `req.ctx["chave"]` por algo type-safe:

```husk
route GET /api/clientes [autenticado] -> ctx {
    require_role(ctx.role, "master")
    return json({ user_id: ctx.user_id })
}
```

> O `ctx` só existe se houver um middleware na cadeia que o produza
> (via `-> ctx` na definição do middleware).

### `require_field` — validação de campos obrigatórios

Valida que um campo do body não está vazio antes de prosseguir:

```husk
route POST /usuarios {
    require_field("nome")
    require_field("email")
    let body = req.body
    // só chega aqui se nome e email não estão vazios
    return status(201, { ok: true })
}
```

Equivalente a:

```husk
if body["nome"] == "" || body["nome"] == nil {
    return status(400, { erro: "Campo obrigatório: nome" })
}
```

Mensagem customizada:

```husk
require_field("cpf", "CPF é obrigatório para pessoa física")
```

> `require_field` acessa `req.body` internamente, então o body JSON é decodificado automaticamente.

## Notas

- Não existe ordem de declaração — uma rota pode chamar uma função definida depois dela no arquivo.
- Todas as rotas do arquivo são registradas no mesmo router e servidas na mesma porta.
- A porta padrão é `8080`. Configuração de porta virá em versão futura.
