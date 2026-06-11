# Schema e Validação

Schemas definem a estrutura e validação de dados de entrada (JSON body).

## Sintaxe

```husk
schema Nome {
    campo: tipo [validador...]
}
```

## Tipos suportados

`int`, `float`, `string`, `bool`

## Validadores

| Validador     | Descrição                          | Exemplo              |
|---------------|------------------------------------|----------------------|
| `required`    | Campo obrigatório (não-zero)       | `nome: string required` |
| `email()`     | Valida formato de email            | `email: string email()`  |
| `unique()`    | Marca para unicidade no banco      | `cpf: string unique()`   |
| `min(N)`      | Valor/length mínimo                | `idade: int min(18)`     |
| `max(N)`      | Valor/length máximo                | `nome: string max(100)`  |

## Exemplo

```husk
schema Usuario {
    nome: string required max(100)
    email: string required email()
    idade: int min(18) max(120)
}
```

Gera no Go:

```go
type Usuario struct {
    Nome  string `json:"nome"`
    Email string `json:"email"`
    Idade int    `json:"idade"`
}

func (s *Usuario) Validate() map[string]string {
    errs := make(map[string]string)
    if s.Nome == "" {
        errs["nome"] = "campo obrigatório"
    }
    if len(s.Nome) > 100 {
        errs["nome"] = "máximo 100 caracteres"
    }
    if s.Email == "" {
        errs["email"] = "campo obrigatório"
    }
    if s.Email != "" && !__emailRegex.MatchString(s.Email) {
        errs["email"] = "email inválido"
    }
    if s.Idade < 18 {
        errs["idade"] = "mínimo 18"
    }
    if s.Idade > 120 {
        errs["idade"] = "máximo 120"
    }
    return errs
}
```

## `validate()` — validação em tempo real

Use `validate(req.body, Schema)` dentro de uma rota para decodificar e validar automaticamente:

```husk
route POST /usuarios {
    let data = validate(req.body, Usuario)
    // data tem os campos tipados: data.nome, data.email, data.idade
    return json(data)
}
```

Isso gera:
1. Decodificação do JSON body para a struct (`400` se JSON inválido)
2. Chamada do `Validate()` (`422` se validação falhar)
3. Retorno dos erros de validação como JSON

Exemplo de resposta `422`:
```json
{
    "erros": {
        "nome": "campo obrigatório",
        "email": "email inválido"
    }
}
```

## Notas

- O validador `required` verifica valor zero do tipo (string vazia, int 0, etc.)
- O validador `email()` usa regex e só valida se o campo não estiver vazio
- O validador `unique()` não gera validação inline — serve como marcador para camada de banco de dados
- Validadores `min(N)`/`max(N)` em strings verificam `len()`, em números verificam valor
- Schemas não geram tabelas no banco automaticamente — são apenas definições de struct + validação
