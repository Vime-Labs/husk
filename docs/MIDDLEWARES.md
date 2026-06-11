# Middlewares

Middlewares interceptam requisições antes de chegarem ao handler da rota. São usados para autenticação, logging, rate limiting, etc.

## Definição

```husk
middleware nome {
    // lógica executada antes do handler
    next()  // passa para o próximo middleware ou handler
}
```

`next()` deve ser chamado para que a requisição continue. Se não for chamado, a requisição é interrompida.

## Exemplos

### Logger simples

```husk
middleware logger {
    next()
}
```

### Autenticação

```husk
middleware autenticado {
    let token = req.headers["Authorization"]
    if token == "" {
        return status(401, json({ erro: "token ausente" }))
    }
    next()
}
```

Se `return` for chamado antes de `next()`, a requisição para ali e o handler não é executado.

## Aplicando a rotas

Middlewares são declarados entre colchetes após o path da rota:

```husk
route GET /perfil [autenticado] {
    return "dados do perfil"
}
```

Múltiplos middlewares são executados na ordem declarada:

```husk
route POST /admin [autenticado, admin] {
    return "ok"
}
```

Go gerado:
```go
r.With(autenticado, admin).Post("/admin", func(...) {
    // handler
})
```

## Passando dados para a rota — `set_ctx` e `req.ctx`

Um middleware pode escrever valores no contexto da requisição com `set_ctx("chave", valor)`. As rotas (e middlewares seguintes) leem esses valores com `req.ctx["chave"]`.

```husk
import "./auth" as auth
import "husk/env" as env

middleware autenticado {
    let token = req.headers["Authorization"]
    if token == "" {
        return status(401, json({ erro: "token ausente" }))
    }
    let claims, err = auth.claims_do_token(token)
    if err != nil {
        return status(401, json({ erro: "token inválido" }))
    }
    set_ctx("role", claims["role"])
    set_ctx("user_id", claims["sub"])
    next()
}
```

Na rota, os valores ficam disponíveis via `req.ctx`:

```husk
route GET /api/auth/users [autenticado] {
    let role = req.ctx["role"]
    if role != "master" {
        return status(403, json({ erro: "acesso negado" }))
    }
    let rows, err = auth.listar_usuarios()
    if err != nil {
        return status(500, json({ erro: err.message }))
    }
    return json({ data: rows })
}
```

Isso elimina a necessidade de re-verificar o token em cada rota protegida.

## Acesso ao `req`

Dentro de um middleware, `req` está disponível da mesma forma que em rotas:

```husk
middleware logger {
    // req.headers, req.query, req.params, req.ctx disponíveis
    next()
}
```

## Regras

- Middlewares são definidos no arquivo raiz (não em módulos importados).
- O nome do middleware deve ser único no arquivo.
- `next()` sem argumentos passa o controle adiante.
- Chamar `return` antes de `next()` interrompe a requisição.
- `set_ctx` só faz sentido antes de `next()` — valores escritos depois não chegam ao handler.
