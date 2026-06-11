# Funções

Funções são definidas com a palavra-chave `fn`. Podem receber parâmetros tipados e declarar um tipo de retorno.

## Sintaxe

```husk
fn nome(param1 tipo1, param2 tipo2) tipo_retorno {
    return valor
}
```

## Exemplos

### Sem parâmetros e sem retorno anotado

O tipo de retorno é inferido pelo transpiler a partir do primeiro `return` encontrado.

```husk
fn greeting() {
    return "Hello, World!"
}
```

Go gerado:
```go
func greeting() string {
    return "Hello, World!"
}
```

### Com parâmetros e retorno explícito

```husk
fn soma(a int, b int) int {
    return a + b
}
```

Go gerado:
```go
func soma(a int, b int) int {
    return a + b
}
```

### Chamando uma função

```husk
route GET /hello {
    return greeting()
}
```

Funções podem ser chamadas dentro de rotas, de outras funções, e em expressões.

## Retorno múltiplo (valor, error)

Funções que podem falhar retornam dois valores: o resultado e um `error`.

```husk
fn buscar(id int) (string, error) {
    return "encontrado", nil
}
```

Go gerado:
```go
func buscar(id int) (string, error) {
    return "encontrado", nil
}
```

Para capturar os dois valores, use `let` com múltiplos nomes:

```husk
let usuario, err = buscar(req.params.id)
if err != nil {
    return status(404, json({ erro: err.message }))
}
return json(usuario)
```

`err.message` é traduzido para `err.Error()` no Go gerado.

## Tipos de retorno suportados

| Husk              | Go                  |
|-------------------|---------------------|
| `int`             | `int`               |
| `float`           | `float64`           |
| `string`          | `string`            |
| `bool`            | `bool`              |
| `NomeDoStruct`    | `NomeDoStruct`      |
| `(Type, error)`   | `(Type, error)`     |
| (omitido)         | inferido pelo transpiler |

## Regras

- Toda função com `return` deve retornar um valor compatível com o tipo declarado (ou inferido).
- Funções sem `return` não precisam de tipo de retorno.
- Sobrecarga de função não é suportada — cada nome deve ser único no módulo.
- Recursão é permitida.
- `nil` é válido como segundo valor em retornos `(Type, error)`.
