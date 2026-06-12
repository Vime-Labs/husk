# Model / ORM

Models estendem schemas com métodos de banco de dados (CRUD).

## Sintaxe

```husk
model Nome {
    table: "nome_da_tabela"
    fields: {
        campo: tipo [validador...]
    }
}
```

O campo `id int64` é gerado automaticamente (chave primária auto-increment).

## Exemplo

```husk
model Usuario {
    table: "usuarios"
    fields: {
        nome: string required max(100)
        email: string required email()
        idade: int min(18)
    }
}
```

## CRUD gerado

### `ModelFind(db, id)` — busca por ID

```go
u, err := UsuarioFind(db, 1)
```

Retorna `(*Model, error)`. `nil` se não encontrado.

### `ModelWhere(db, field, value)` — busca por campo

```go
usuarios, err := UsuarioWhere(db, "email", "a@b.com")
```

Retorna `([]Model, error)`. Lista vazia se nenhum.

### `Insert(db)` — insere registro

```go
u := Usuario{Nome: "João", Email: "joao@email.com", Idade: 30}
err := u.Insert(db)
```

### `Update(db)` — atualiza registro

```go
u.Nome = "João Silva"
err := u.Update(db)
```

### `Delete(db)` — deleta registro

```go
err := u.Delete(db)
```

## Validação

Todo model também gera um método `Validate()` idêntico ao de schemas. Use `validate(req.body, Nome)` em rotas para decodificar + validar antes de inserir/atualizar.

## Exemplo com rota

```husk
route POST /usuarios {
    let data = validate(req.body, Usuario)
    if errs := data.Validate(); errs != nil {
        return status(422) json(errs)
    }
    let u = Usuario{Nome: data.Nome, Email: data.Email, Idade: data.Idade}
    u.Insert(db)
    return json(u)
}
```
