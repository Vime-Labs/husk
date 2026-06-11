# Expressões e Operadores

## Operadores aritméticos

| Operador | Descrição       |
|----------|-----------------|
| `+`      | adição          |
| `-`      | subtração       |
| `*`      | multiplicação   |
| `/`      | divisão         |
| `%`      | módulo (resto)  |

```husk
fn area(largura int, altura int) int {
    return largura * altura
}
```

## Operadores de comparação

| Operador | Descrição         |
|----------|-------------------|
| `==`     | igual             |
| `!=`     | diferente         |
| `<`      | menor             |
| `>`      | maior             |
| `<=`     | menor ou igual    |
| `>=`     | maior ou igual    |

## Operadores lógicos

| Operador | Descrição |
|----------|-----------|
| `&&`     | e (and)   |
| `\|\|`   | ou (or)   |
| `!`      | não (not) |

## Precedência (maior para menor)

1. `!`, `-` (unário)
2. `*`, `/`, `%`
3. `+`, `-`
4. `<`, `>`, `<=`, `>=`
5. `==`, `!=`
6. `&&`
7. `\|\|`

Use parênteses para forçar outra ordem:

```husk
let resultado = (a + b) * c
```

## Acesso a campos

```husk
let nome = usuario.nome
let cidade = usuario.endereco.cidade
```

## Acesso por índice

```husk
let primeiro = lista[0]
let valor = mapa["chave"]
```

## Chamadas de função

```husk
let resultado = soma(10, 20)
greeting()
```

### Spread `...`

Um map pode ser desestruturado como argumentos de função com `...`:

```husk
let cliente = clientes.criar(body...)? 500 "Erro"
```

Mapeia as chaves do map para os nomes dos parâmetros da função.

## Laço `for...in`

Itera sobre os elementos de uma lista ou array:

```husk
for item in items {
    return item
}
```

Gera `for _, item := range items { ... }` no Go. Útil para percorrer resultados de queries:

```husk
route GET /lista {
    let rows, err = db.query("SELECT id, nome FROM usuarios")
    if err != nil {
        return status(500, json({ erro: err.message }))
    }
    for row in rows {
        return json(row)
    }
}
```

# Condicional

```husk
if condicao {
    // bloco then
}

if condicao {
    // then
} else {
    // else
}
```

Exemplo com erro:

```husk
let val, err = buscar(id)
if err != nil {
    return status(404)
}
return json(val)
```

## Literais

```husk
42          // int
3.14        // float
"texto"     // string
true        // bool
false       // bool
```

## Objeto literal

```husk
{ chave: "valor", numero: 42 }
```
