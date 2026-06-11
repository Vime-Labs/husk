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

## Acesso ao `req`

Dentro de um middleware, `req` está disponível da mesma forma que em rotas:

```husk
middleware logger {
    // req.headers, req.query, req.params disponíveis
    next()
}
```

## Regras

- Middlewares são definidos no arquivo raiz (não em módulos importados).
- O nome do middleware deve ser único no arquivo.
- `next()` sem argumentos passa o controle adiante.
- Chamar `return` antes de `next()` interrompe a requisição.
