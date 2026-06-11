# Análise Semântica

O analisador semântico é a terceira fase do pipeline do Husk (após lexer e parser, antes do codegen). Ele verifica tipos, escopos e regras da linguagem, garantindo que o programa seja válido antes de gerar código Go.

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

### Built-ins

- **`set_ctx(chave, valor)`** — disponível em middlewares e rotas para armazenar dados no contexto da requisição
- **`parse_int(s)`** — converte string para inteiro, retorna `(int, error)`. Use com `?`: `parse_int(req.params.id)?`
- **`require_role(role, mensagem?)`** — verifica se `req.ctx["role"]` é igual ao valor esperado. Se não, retorna `403` com JSON. Mensagem opcional (padrão: "Acesso restrito")

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
- Source maps (mapeamento de erros Go para linhas Husk) não estão implementados
