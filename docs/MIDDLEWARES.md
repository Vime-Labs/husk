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

## Passando dados para a rota — contexto tipado com `-> ctx`

### Definição do middleware com contexto

Adicione `-> ctx_var` após o nome do middleware para declarar que ele produz um contexto
tipado para as rotas:

```husk
middleware autenticado -> ctx {
    let token = req.headers["Authorization"]
    if token == "" {
        return status(401, json({ erro: "token ausente" }))
    }
    // claims vindos da validação do JWT
    let claims = jwt.verify(token, secret)

    ctx.role = claims["role"]       // string
    ctx.user_id = claims["sub"]     // string
    next()
}
```

O nome da variável (`ctx`) é escolhido por você — pode ser `auth`, `session`, etc.

### Rota consumindo o contexto

A rota declara `-> ctx` para receber o contexto tipado:

```husk
route GET /api/clientes [autenticado] -> ctx {
    require_role(ctx.role, "master")
    return json({ user_id: ctx.user_id })
}
```

Isso substitui o padrão antigo:

| Antes (string-based)        | Agora (type-safe)             |
|-----------------------------|-------------------------------|
| `set_ctx("role", valor)`   | `ctx.role = valor`            |
| `req.ctx["role"]`          | `ctx.role`                    |
| `r.Context().Value("role").(string)` | (gerado automaticamente) |

> As chaves são namespaced automaticamente (`__husk_ctx_role`), eliminando colisões
> entre middlewares diferentes.

### Múltiplos middlewares na cadeia

Um middleware posterior pode ler campos setados por um anterior:

```husk
middleware admin -> ctx {
    if ctx.role != "admin" {
        return status(403, json({ erro: "só admin" }))
    }
    next()
}

route POST /admin [autenticado, admin] -> ctx {
    // ctx.role e ctx.user_id disponíveis
    return json({ ok: true })
}
```

## Acesso ao `req`

Dentro de um middleware, `req` está disponível da mesma forma que em rotas:

```husk
middleware logger {
    // req.headers, req.query, req.params disponíveis
    next()
}
```

## Compatibilidade: `set_ctx` e `req.ctx` (estilo antigo)

O estilo antigo com `set_ctx("chave", valor)` e `req.ctx["chave"]` ainda funciona,
mas o novo padrão `-> ctx` é recomendado por ser type-safe.

## Regras

- Middlewares são definidos no arquivo raiz (não em módulos importados).
- O nome do middleware deve ser único no arquivo.
- `next()` sem argumentos passa o controle adiante.
- Chamar `return` antes de `next()` interrompe a requisição.
- Atribuições a `ctx.field` antes de `next()` são propagadas para a rota.
- O contexto (`-> ctx`) só está disponível se a rota também declarar `-> ctx`.
