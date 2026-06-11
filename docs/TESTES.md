# Testes

O Husk tem suporte nativo a testes com `husk test` e a built-in `assert_eq()`.

## Escrevendo testes

Arquivos com sufixo `_test.husk` são automaticamente descobertos por `husk test`. Cada função com prefixo `test_` é um caso de teste:

```husk
// math_test.husk
fn test_soma() {
    assert_eq(4, 2 + 2)
}

fn test_subtracao() {
    assert_eq(1, 3 - 2)
}
```

## Executando

```sh
husk test                   # descobre *_test.husk no diretório atual
husk test tests/            # diretório específico
husk test usuario_test.husk # arquivo específico
```

Saída:
```
      testes 2 arquivo(s)
               usuario_test.husk
  executando testes...
  PASS  test_soma
  PASS  test_string
         ✓ todos os testes passaram (0.5s)
```

## `assert_eq(expected, actual)`

Compara dois valores. Se forem diferentes, o teste falha com uma mensagem:

```husk
assert_eq(4, 2 + 2)        # passa
assert_eq("oi", "oi")       # passa
assert_eq(42, "42")         # falha: tipos diferentes
```

Gera no Go:
```go
func() { if expected != actual { panic(fmt.Sprintf("assert_eq falhou: esperado %v, recebido %v", expected, actual)) } }()
```

## Combinando com outras built-ins

```husk
fn test_conversoes() {
    assert_eq(42, parse_int("42")?)
    let f = float("3.14")?
    assert_eq(3.14, f)
    assert_eq("42", string(42))
}
```

## Organização

```
meu-projeto/
├── main.husk
├── usuarios.husk
├── usuarios_test.husk    # testes do módulo usuarios
├── math_test.husk        # testes de funções matemáticas
└── ...
```

Não é necessário importar nada — todas as funções `test_*` são descobertas automaticamente.

## Limitações (v1)

- Testes não têm suporte a setup/teardown
- `assert_eq` usa comparação Go (`!=`), que funciona para tipos primitivos mas não para maps diretamente
- Testes que dependem de HTTP (`req.*`) ou banco de dados não são suportados nativamente
