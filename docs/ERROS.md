# Tratamento de Erros

Husk não tem exceções. O padrão é retornar um par `(valor, error)` — o mesmo idioma do Go.

## Funções que podem falhar

Declare o tipo de retorno como `(tipo, error)`:

```husk
fn buscar(id int) (string, error) {
    return "encontrado", nil
}
```

Quando há erro, retorne `nil` no lugar do valor:

```husk
fn dividir(a int, b int) (int, error) {
    if b == 0 {
        return 0, erro("divisão por zero")
    }
    return a / b, nil
}
```

> **Nota:** a função `erro()` para criar erros customizados está planejada para v1.0. Por ora, errors são propagados de chamadas Go internas.

## Capturando erros

Use `let` com múltiplos nomes:

```husk
let resultado, err = buscar(id)
```

Sempre verifique o erro antes de usar o valor:

```husk
let usuario, err = buscar(req.params.id)
if err != nil {
    return status(404, json({ erro: err.message }))
}
return json(usuario)
```

## `err.message`

Acessa a mensagem de texto do erro. Traduzido para `.Error()` no Go gerado.

```husk
err.message  →  err.Error()
```

## `nil`

Indica ausência de erro. Use sempre como segundo valor em retornos bem-sucedidos:

```husk
return valor, nil
```

## Operador `?` (Try)

Para reduzir boilerplate, use `?` após uma chamada de função que retorna `(valor, error)`.

### Uso básico

```husk
let usuario = usuarios.buscar(req.params.id)?
```

Isso gera automaticamente:

```go
__try1_val, __try1_err := usuarios_buscar(req.params.id)
if __try1_err != nil {
    w.WriteHeader(500)
    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(map[string]interface{}{"erro": __try1_err.Error()})
    return
}
usuario := __try1_val
```

Equivalente a:

```husk
let usuario, err = usuarios.buscar(req.params.id)
if err != nil {
    return status(500, json({ erro: err.message }))
}
```

### Uso em expressões aninhadas

O `?` funciona dentro de argumentos de função, permitindo encadear chamadas:

```husk
let cliente = clientes.buscar(parse_int(req.params.id)?)? 404 "Cliente não encontrado"
```

Cada `?` gera seu próprio bloco de tratamento de erro. O código gerado executa as operações na ordem correta:

```go
__try1_val, __try1_err := strconv.Atoi(chi.URLParam(r, "id"))
if __try1_err != nil {
    w.WriteHeader(500)
    ...
    return
}
__try2_val, __try2_err := buscar(__try1_val)
if __try2_err != nil {
    w.WriteHeader(404)
    ...
    return
}
cliente := __try2_val
```

### Status code customizado

```husk
let usuario = usuarios.buscar(req.params.id)? 404
```

### Mensagem customizada

```husk
let usuario = usuarios.buscar(req.params.id)? 404 "Usuário não encontrado"
```

### Exemplo completo

Antes:
```husk
let usuario, err = usuarios.buscar(req.params.id)
if err != nil {
    return status(404, json({ erro: err.message }))
}
return json(usuario)
```

Depois:
```husk
let usuario = usuarios.buscar(req.params.id)? 404
return json(usuario)
```

> O operador `?` só funciona dentro de `route` e `middleware` (contextos com acesso a `w`/`r` HTTP).
> Status code padrão é `500`.
