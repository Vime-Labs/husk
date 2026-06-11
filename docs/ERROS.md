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
usuario, __try_err := usuarios_buscar(req.params.id)
if __try_err != nil {
    w.WriteHeader(500)
    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(map[string]interface{}{"erro": __try_err.Error()})
    return
}
```

Equivalente a:

```husk
let usuario, err = usuarios.buscar(req.params.id)
if err != nil {
    return status(500, json({ erro: err.message }))
}
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
