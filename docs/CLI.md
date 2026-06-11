# CLI

A CLI do Husk é o único ponto de entrada para transpilar, executar e compilar projetos.

## Instalação

```sh
cargo install --path crates/husk-cli
```

O binário `husk` é instalado em `~/.cargo/bin/husk`. Certifique-se de que esse diretório está no seu `PATH`.

**Requisitos:**
- Rust + Cargo (`curl https://sh.rustup.rs | sh`)
- Go 1.21+ (`https://go.dev/dl/`)

---

## Comandos

### `husk run <arquivo.husk>`

Transpila o arquivo e inicia o servidor HTTP em modo de desenvolvimento.

```sh
husk run main.husk
```

O servidor sobe na porta `8080` por padrão. Use a variável `PORT` para alterar (se o projeto usa `husk/env`).

O processo roda até ser interrompido com `Ctrl+C`.

### `husk build <arquivo.husk>`

Gera um binário nativo otimizado no diretório atual.

```sh
husk build main.husk
# gera ./main
```

O binário resultante é autossuficiente — não precisa de Go instalado no servidor de destino.

### `husk check <arquivo.husk>`

Verifica sintaxe, semântica e codegen sem gerar nenhum artefato. Útil para CI ou em editores.

```sh
husk check main.husk
```

Inclui análise de tipos, escopo e regras da linguagem. Termina com código 0 se não houver erros, 1 caso contrário.

### `husk new <nome>`

Cria um novo projeto no diretório `<nome>/` com um `main.husk` mínimo e um `.gitignore`.

```sh
husk new meu-projeto
cd meu-projeto
husk run main.husk
```

---

## Fluxo interno

```
arquivo.husk
    │
    ▼
  Lexer              tokenização
    │
    ▼
  Parser             AST
    │
    ▼
  Analisador         verificação de tipos e escopo ← novo
    │
    ▼
  resolve_imports    inline dos módulos locais + copia shims da stdlib
    │
    ▼
  Codegen            Go (package main)
    │
    ▼
  go mod tidy        baixa dependências Go
    │
    ▼
  go run .           ou  go build -o <nome> .
```

---

## Estrutura de projeto recomendada

```
meu-projeto/
├── main.husk          ponto de entrada, rotas e middlewares
├── usuarios.husk      módulo de domínio
├── produtos.husk      módulo de domínio
└── .gitignore         exclui *.go, go.mod, go.sum e o binário
```

O `.gitignore` gerado por `husk new` já exclui os artefatos Go. Não há necessidade de versionar o código transpilado.

---

## Variáveis de ambiente relevantes

| Variável       | Uso                                              |
|----------------|--------------------------------------------------|
| `DATABASE_URL` | Conexão automática quando `husk/postgres` é usado |
| `PORT`         | Configurável via `husk/env` no código da aplicação |
