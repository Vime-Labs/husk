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

## Condicional

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

`if`/`else` pode aparecer em funções e em rotas. Dentro de rotas, `return` dentro de um bloco `if` funciona normalmente.

## Literais

```husk
42          // int
3.14        // float
"texto"     // string
true        // bool
false       // bool
```

## Objeto literal

Usado principalmente como argumento de `json()`:

```husk
{ chave: "valor", numero: 42 }
```
