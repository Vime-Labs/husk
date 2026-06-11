# Rotas

Rotas definem endpoints HTTP. O servidor usa `chi` por baixo e é iniciado automaticamente na porta `8080`.

## Sintaxe

```husk
route MÉTODO /caminho {
    // corpo
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

O parâmetro `:id` é mapeado para `{id}` no chi e deve ser lido via `req.params` (disponível a partir do v0.4).

### Retorno em JSON

```husk
route GET /ping {
    return json({ status: "ok" })
}
```

Gera os headers `Content-Type: application/json` automaticamente e serializa o objeto.

### Retorno com status HTTP

```husk
route DELETE /item {
    return status(204)
}

route POST /item {
    return status(400, json({ erro: "campo ausente" }))
}
```

### Retorno de texto simples

```husk
route GET /healthz {
    return text("ok")
}
```

## Funções de resposta

| Husk                    | Comportamento                              |
|-------------------------|--------------------------------------------|
| `return expr`           | escreve o valor como texto (`fmt.Fprint`)  |
| `return json({...})`    | serializa como JSON com header correto     |
| `return text("...")`    | escreve texto puro                         |
| `return status(N)`      | define o status HTTP sem corpo             |
| `return status(N, ...)` | define o status HTTP e escreve o corpo     |

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

## Notas

- Não existe ordem de declaração — uma rota pode chamar uma função definida depois dela no arquivo.
- Todas as rotas do arquivo são registradas no mesmo router e servidas na mesma porta.
- A porta padrão é `8080`. Configuração de porta virá em versão futura.
