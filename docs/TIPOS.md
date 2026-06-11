# Tipos

Husk tem tipagem estática. Os tipos aparecem em parâmetros de função, retornos explícitos e declarações `let` com anotação.

## Tipos primitivos

| Husk     | Go        | Exemplos de literal       |
|----------|-----------|---------------------------|
| `int`    | `int`     | `0`, `42`, `-7`           |
| `float`  | `float64` | `3.14`, `0.5`             |
| `string` | `string`  | `"texto"`, `""`           |
| `bool`   | `bool`    | `true`, `false`           |

## Tipos compostos

### Struct

Structs agrupam campos nomeados (disponível a partir do v0.3):

```husk
struct Usuario {
    id    int
    nome  string
    email string
}
```

### Objeto literal (inline)

Disponível como argumento para `json()` em rotas:

```husk
return json({ status: "ok", codigo: 200 })
```

Gera `map[string]interface{}` no Go.

## Declaração de variáveis

### Com inferência

```husk
let nome = "João"   // inferido como string
let idade = 30      // inferido como int
```

### Com anotação explícita

```husk
let nome string = "João"
let idade int = 30
```

## Tipo `error`

Usado exclusivamente como segundo valor em retornos de função:

```husk
fn buscar(id int) (string, error) {
    return "ok", nil
}
```

Dentro de um `if`, acesse a mensagem de erro com `.message`:

```husk
let val, err = buscar(id)
if err != nil {
    return status(500, json({ erro: err.message }))
}
```

## `nil`

Representa ausência de erro. Válido apenas como segundo valor em retornos `(Type, error)`:

```husk
return "resultado", nil
```

## Conversão de tipos

Não há conversão implícita. Conversão explícita virá em versão futura.

## Inferência de retorno em funções

Quando o tipo de retorno não é anotado, o transpiler infere pelo primeiro `return`:

| Expressão retornada | Tipo inferido |
|---------------------|---------------|
| literal string      | `string`      |
| literal int         | `int`         |
| literal float       | `float64`     |
| literal bool        | `bool`        |
| chamada de função   | `interface{}` |
| outro               | `interface{}` |

Para casos complexos, declare o tipo explicitamente.
