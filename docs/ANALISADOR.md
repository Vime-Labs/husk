# Análise Semântica

O analisador semântico é a terceira fase do pipeline do Husk (após lexer e parser, antes do codegen). Ele verifica tipos, escopos e regras da linguagem.

## O que é verificado

### Escopo

- **Variável não declarada** — uso de variável que não foi definida com `let` ou como parâmetro
- **Função não definida** — chamada de função inexistente
- **Struct não definido** — uso de struct não declarado com `struct`
- **Duplicatas top-level** — duas funções, structs ou middlewares com o mesmo nome

### Tipos

- **`let` com anotação** — o tipo do valor atribuído é compatível com a anotação explícita
- **Argumentos de função** — tipos dos argumentos correspondem aos parâmetros
- **Retorno de função** — valores retornados são compatíveis com o tipo de retorno declarado
- **Operadores binários**:
  - `&&` e `||` — operandos devem ser `bool`
  - `<`, `>`, `<=`, `>=` — operandos devem ser numéricos
  - `+`, `-`, `*`, `/`, `%` — operandos devem ser numéricos (exceto `+` para strings: concatenação)
  - `==`, `!=` — operandos devem ser do mesmo tipo
- **Operadores unários**:
  - `!` — operando deve ser `bool`
  - `-` — operando deve ser numérico

### Estruturas de controle

- **`if`** — condição deve ser do tipo `bool`
- **`for...in`** — coleção deve ser `[]T` ou `map`

### Contexto

- **`next()`** — só pode ser usado dentro de um bloco `middleware`
- **`req.*`** — só pode ser usado dentro de uma `route`
  - `req.params.campo` → `string`
  - `req.headers["chave"]` → `string`
  - `req.query["chave"]` → `string`
  - `req.body["chave"]` → `string`

### Structs

- **Campos existentes** — inicialização de struct só pode conter campos que existem na definição
- **Campos obrigatórios** — todos os campos do struct devem ser preenchidos na inicialização
- **Tipos dos campos** — cada campo deve receber um valor de tipo compatível

### Middlewares

- **Referência em rota** — middlewares listados em `route GET /path [mw]` devem estar definidos

### Erros

- **`err.message`** — só pode ser acessado em valores do tipo `error` (ou `Unknown`)
- **Multi-retorno** — `let a, b = fn()` deve ter exatamente 2 variáveis

### For...in

- **Coleção** — o iterável deve ser `[]T` ou `map`

## Built-ins registrados

| Função                  | Assinatura                |
|-------------------------|---------------------------|
| `json(val)`             | `(any)`                   |
| `text(s)`               | `(any)`                   |
| `status(code, body?)`   | `(int, any?)`             |
| `set_ctx(key, val)`     | `(string, any)`           |
| `parse_int(s)`          | `(string) → (int, error)` |
| `float(s)`              | `(string) → (float, error)` |
| `string(val)`           | `(any) → string`          |
| `erro(msg)`             | `(string) → error`        |
| `assert_eq(expected, actual)` | `(any, any)`        |
| `require_role(actual, expected, msg?)` | `(string, string, string?)` |
| `require_field(field, msg?)` | `(string, string?)`    |
| `len(s)` / `contains(...)` / etc. | string built-ins  |
| `abs(n)` / `sqrt(n)` / etc. | math built-ins        |

## Source maps

O analisador não gera source maps diretamente, mas o codegen insere anotações `// husk:arquivo:linha` antes de cada `FnDef`, `RouteDef` e `MiddlewareDef`. A CLI (`husk run`, `husk build`) usa essas anotações para traduzir erros do compilador Go de volta para as linhas do código `.husk` original.

## Integração com CLI

O analisador é executado automaticamente em todos os comandos da CLI:

```
husk check arquivo.husk   # análise + sintaxe
husk run   arquivo.husk   # análise + sintaxe + geração Go + execução
husk build arquivo.husk   # análise + sintaxe + geração Go + compilação
```

Erros semânticos são exibidos no formato:

```
arquivo.husk:linha:col erro semântico: mensagem
```

## Limitações

- Tipos de retorno de funções via módulo (`alias.metodo()`) não são verificados — assume-se `Unknown`
- Não há verificação de fluxo (ex: nem todos os caminhos retornam um valor)
