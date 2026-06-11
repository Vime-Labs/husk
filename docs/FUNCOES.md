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

### Com parâmetros e retorno explícito

```husk
fn soma(a int, b int) int {
    return a + b
}
```

### Chamando uma função

```husk
route GET /hello {
    return greeting()
}
```

## Retorno múltiplo (valor, error)

Funções que podem falhar retornam dois valores: o resultado e um `error`.

```husk
fn buscar(id int) (string, error) {
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

## Tipos de retorno suportados

| Husk              | Go                  |
|-------------------|---------------------|
| `int`             | `int`               |
| `float`           | `float64`           |
| `string`          | `string`            |
| `bool`            | `bool`              |
| `NomeDoStruct`    | `NomeDoStruct`      |
| `(Type, error)`   | `(Type, error)`     |
| `map`             | `map[string]interface{}` |
| `[]tipo`          | `[]Tipo`            |
| (omitido)         | inferido pelo transpiler |

## Built-ins de conversão

| Função        | Retorno             | Go gerado                         |
|---------------|---------------------|-----------------------------------|
| `parse_int(s)` | `(int, error)`      | `strconv.Atoi(s)`                |
| `float(s)`    | `(float, error)`     | `strconv.ParseFloat(s, 64)`      |
| `string(val)` | `string`             | `fmt.Sprintf("%v", val)`         |
| `erro(msg)`   | `error`              | `fmt.Errorf(msg)`                |

```husk
let n = parse_int("42")?           // 42
let f = float("3.14")?             // 3.14
let s = string(42)                 // "42"
return "", erro("deu ruim")        // cria erro
```

## Built-ins de string

| Função                  | Retorno     | Go gerado                         |
|-------------------------|-------------|-----------------------------------|
| `len(s)`                | `int`       | `len(s)`                          |
| `contains(s, sub)`      | `bool`      | `strings.Contains(s, sub)`        |
| `starts_with(s, prefix)`| `bool`      | `strings.HasPrefix(s, prefix)`    |
| `replace(s, old, new)`  | `string`    | `strings.Replace(s, old, new, -1)`|
| `split(s, sep)`         | `[]string`  | `strings.Split(s, sep)`           |
| `trim(s)`               | `string`    | `strings.TrimSpace(s)`            |
| `upper(s)`              | `string`    | `strings.ToUpper(s)`              |
| `lower(s)`              | `string`    | `strings.ToLower(s)`              |

## Built-ins de math

| Função       | Retorno  | Go gerado    |
|--------------|----------|--------------|
| `abs(n)`     | `float`  | `math.Abs(n)`|
| `sqrt(n)`    | `float`  | `math.Sqrt(n)`|
| `min(a, b)`  | `int`    | `min(a, b)`  |
| `max(a, b)`  | `int`    | `max(a, b)`  |

## Regras

- Toda função com `return` deve retornar um valor compatível com o tipo declarado (ou inferido).
- Funções sem `return` não precisam de tipo de retorno.
- Sobrecarga não é suportada — cada nome deve ser único no módulo.
- Recursão é permitida.
- `nil` é válido como segundo valor em retornos `(Type, error)`.
