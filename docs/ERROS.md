# Tratamento de Erros

Husk não tem exceções. O padrão é retornar um par `(valor, error)` — o mesmo idioma do Go.

## Funções que podem falhar

Declare o tipo de retorno como `(tipo, error)`:

```husk
fn buscar(id int) (string, error) {
    return "encontrado", nil
}
```

## Criando erros customizados com `erro()`

Use `erro(mensagem)` para criar um valor de erro:

```husk
fn dividir(a int, b int) (int, error) {
    if b == 0 {
        return 0, erro("divisão por zero")
    }
    return a / b, nil
}
```

Gera `fmt.Errorf("divisão por zero")` no Go.

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

Cada `?` gera seu próprio bloco de tratamento de erro.

### Status code customizado

```husk
let usuario = usuarios.buscar(req.params.id)? 404
```

### Mensagem customizada

```husk
let usuario = usuarios.buscar(req.params.id)? 404 "Usuário não encontrado"
```

### Circuit breaker

Use `break` após `? [status] ["msg"]` para ativar o circuit breaker:

```husk
let usuario = usuarios.buscar(req.params.id)? 500 break
```

Isso gera um wrapper que:
- Mantém contagem de falhas consecutivas por `? break`
- Após 5 falhas consecutivas, abre o circuito — novas chamadas retornam `503` imediatamente sem executar a chamada real
- Após 30s de cooldown, permite um probe request (half-open)
- Em caso de sucesso, o circuito fecha e o contador de falhas zera

> O circuit breaker usa `sync.Mutex` e `time` no Go gerado. Ideal para chamadas a bancos de dados e APIs externas que podem ficar indisponíveis temporariamente.

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
