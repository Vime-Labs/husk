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

### map

Representa um objeto com chaves string e valores dinâmicos. Gerado como `map[string]interface{}` no Go. Usado principalmente como tipo de retorno de funções que consultam o banco de dados.

```husk
fn auth_user(email string, senha string) (map, error) {
    return db.query_one("SELECT id, role FROM users WHERE email = $1", email)
}
```

### []tipo

Lista de elementos do mesmo tipo. Gerado como slice Go (`[]T`).

```husk
fn listar_usuarios() ([]map, error) {
    return db.query("SELECT id, nome FROM usuarios")
}
```

Combinações: `[]map`, `[]string`, `[]int`, `[]NomeDaStruct`.

### Struct

Structs agrupam campos nomeados:

```husk
struct Usuario {
    id    int
    nome  string
    email string
}
```

### Objeto literal (inline)

Disponível como argumento para `json()` ou `return`:

```husk
return { status: "ok", codigo: 200 }
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

## `nil`

Representa ausência de erro. Válido apenas como segundo valor em retornos `(Type, error)`:

```husk
return "resultado", nil
```

## Conversão de tipos

Não há conversão implícita entre tipos. Use funções built-in para conversão explícita:

| Função           | Retorno             | Go gerado                       |
|------------------|---------------------|---------------------------------|
| `parse_int(s)`   | `(int, error)`      | `strconv.Atoi(s)`               |
| `float(s)`       | `(float, error)`    | `strconv.ParseFloat(s, 64)`     |
| `string(val)`    | `string`            | `fmt.Sprintf("%v", val)`        |

```husk
let n = parse_int("42")?         // int
let f = float("3.14")?           // float
let s = string(42)               // string → "42"
let msg = "valor: " + string(n)  // concatenação com string()
```

`parse_int()` e `float()` retornam `(T, error)` — use com `?` para propagar erros.

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
