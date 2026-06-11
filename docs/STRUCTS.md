# Structs

Structs agrupam campos nomeados sob um tipo. Disponível a partir do v0.3.

## Definição

```husk
struct NomeDoTipo {
    campo1 tipo1
    campo2 tipo2
}
```

Os campos são separados por quebra de linha. Não há vírgula ou ponto-e-vírgula.

```husk
struct Usuario {
    id    int
    nome  string
    email string
    ativo bool
}
```

No Go gerado, cada campo recebe uma tag `json` com o nome em minúsculas:

```go
type Usuario struct {
    Id    int    `json:"id"`
    Nome  string `json:"nome"`
    Email string `json:"email"`
    Ativo bool   `json:"ativo"`
}
```

## Instanciação

```husk
let u = Usuario{ id: 1, nome: "João", email: "joao@exemplo.com", ativo: true }
```

Todos os campos devem ser fornecidos na instanciação. Campos omitidos ficam com o zero value do Go (`0`, `""`, `false`).

## Acesso a campos

```husk
let nome = u.nome
```

No Go gerado, o campo é capitalizado automaticamente: `u.Nome`.

## Structs em rotas

```husk
import "./usuarios" as usuarios

route GET /usuarios/:id {
    let u = usuarios.buscar(req.params.id)
    return json(u)
}
```

`json()` serializa a struct para JSON usando as tags geradas automaticamente.

## Structs como parâmetro e retorno de função

```husk
struct Produto {
    id    int
    preco float
}

fn desconto(p Produto, pct float) Produto {
    return Produto{ id: p.id, preco: p.preco }
}
```

## Tipos de campo suportados

| Tipo Husk | Tipo Go      |
|-----------|--------------|
| `int`     | `int`        |
| `float`   | `float64`    |
| `string`  | `string`     |
| `bool`    | `bool`       |
| `NomeTipo`| `NomeTipo`   |

Tipos compostos como slices e maps não são suportados como campos de struct em v0.5. Use `json()` com objeto literal para respostas que precisam de listas.
