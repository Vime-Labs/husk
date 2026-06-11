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

Gera `for _, item := range items { ... }` no Go.

## Try/Catch

Recupera de panics dentro de um bloco:

```husk
try {
    let user = db.query_one(sql)? 500
    return json(user)
} catch err {
    return status(500, json({ erro: err.message }))
}
```

Gera uma IIFE com `defer/recover` no Go. A variável `err` contém o erro como `string` convertido para `error`.

## Retry

Retenta um bloco de código N vezes com delay fixo em milissegundos entre tentativas:

```husk
retry 3 100 {
    let rows = db.query(sql)?
}
```

Gera um loop `for _i := 0; _i < N; _i++` com `time.Sleep(delay * time.Millisecond)` entre tentativas. Se o bloco executar sem panic, o loop é interrompido (`break`).

```husk
retry 5 500 {
    let user = db.query_one("SELECT * FROM users WHERE id = $1", id)? 500
    return json(user)
}
```

## Circuit Breaker

Ative o circuit breaker com `break` após `? [status] ["msg"]`:

```husk
let usuario = db.query_one("SELECT * FROM users WHERE id = $1", id)? 500 break
```

Gera wrapper que conta falhas consecutivas. Após 5 falhas, o circuito abre e retorna `503` sem executar a chamada. Após 30s de cooldown, permite um probe. Sucesso fecha o circuito.

```husk
route GET /users/:id {
    let user = db.query_one("SELECT * FROM users WHERE id = $1", id)? 404 "Não encontrado" break
    return json(user)
}
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
