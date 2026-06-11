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

## Padrão recomendado em rotas

```husk
route GET /usuarios/:id {
    let usuario, err = usuarios.buscar(req.params.id)
    if err != nil {
        return status(404, json({ erro: err.message }))
    }
    return json(usuario)
}
```
