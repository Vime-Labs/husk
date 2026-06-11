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

Combinações possíveis: `[]map`, `[]string`, `[]int`, `[]NomeDaStruct`.

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

Não há conversão implícita entre tipos. Use `parse_int()` para converter string para inteiro:

```husk
fn buscar(id int) (map, error)

route GET /users/:id {
    let id = parse_int(req.params.id)? 400 "ID inválido"
    let user = buscar(id)? 404 "Usuário não encontrado"
    return json(user)
}
```

`parse_int(s)` gera `strconv.Atoi(s)` no Go, retornando `(int, error)`. Por isso deve ser usado com `?`.

> Conversão explícita para outros tipos (`float`, `string`) virá em versões futuras.

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
