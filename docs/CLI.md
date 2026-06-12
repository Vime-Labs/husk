# CLI

A CLI do Husk é o único ponto de entrada para transpilar, executar, testar e gerenciar projetos.

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

Transpila o arquivo e inicia o servidor HTTP.

```sh
husk run main.husk
```

O servidor sobe na porta definida pela variável `PORT`, ou `:8080` se não definida.

### `husk dev <arquivo.husk>`

Modo desenvolvimento com hot reload. Monitora o arquivo a cada 500ms e reinicia o servidor automaticamente ao salvar.

```sh
husk dev main.husk
```

### `husk build <arquivo.husk>`

Gera um binário nativo otimizado no diretório atual.

```sh
husk build main.husk
# gera ./main
```

O binário resultante é autossuficiente — não precisa de Go instalado no servidor de destino.

### `husk test [arquivo | diretório]`

Descobre e executa arquivos `*_test.husk`. Cada função `test_*` no arquivo é executada como um caso de teste. Use `assert_eq(expected, actual)` dentro delas.

```sh
husk test                   # descobre *_test.husk no diretório atual
husk test tests/            # descobre em diretório específico
husk test usuario_test.husk # arquivo específico
```

Exemplo de `usuario_test.husk`:
```husk
fn test_soma() {
    assert_eq(4, 2 + 2)
}

fn test_string() {
    assert_eq("oi", string(42))
}
```

### `husk check <arquivo.husk>`

Verifica sintaxe, semântica e codegen sem gerar artefatos. Útil para CI ou editores.

```sh
husk check main.husk
```

### `husk fmt <arquivo.husk>`

Formata o código Husk in-place. Preserva comentários.

```sh
husk fmt main.husk
```

### `husk add <modulo>`

Adiciona um módulo da stdlib ao projeto. Módulos disponíveis: `env`, `postgres`, `crypto`, `jwt`, `log`, `http`.

```sh
husk add postgres
# adiciona: import "husk/postgres" as postgres
```

### `husk new <nome>`

Cria um novo projeto no diretório `<nome>/` com `main.husk`, `husk.json` e `.gitignore`.

```sh
husk new meu-projeto
cd meu-projeto
husk run main.husk
```

### `husk install`

Instala dependências externas declaradas em `husk.json` para o diretório `vendor/`.

```sh
husk install
husk install --force   # reinstala mesmo se vendor/ já existir
```

Cada dependência é clonada via git para `vendor/<nome>/`. Dependências transitivas (o package tem o seu próprio `husk.json`) são resolvidas recursivamente. Após a instalação, o ficheiro `.vendor.husk` é gerado automaticamente e incluído em tempo de compilação.

```json
{
  "name": "meu-app",
  "dependencies": {
    "framework": {
      "git": "https://github.com/vime/husk-framework",
      "ref": "v0.1.0"
    }
  }
}
```

---

## Fluxo interno

```
arquivo.husk
    │
    ▼
  Lexer              tokenização (comentários preservados como tokens)
    │
    ▼
  Parser             AST (comentários filtrados)
    │
    ▼
  Analisador         verificação de tipos, escopo, built-ins, for...in
    │
    ▼
  resolve_imports    inline de módulos locais (com detecção de ciclo)
    │
    ▼
  Codegen            Go (package main) + source maps // husk:file:line
    │
    ▼
  go mod tidy        baixa dependências Go
    │
    ▼
  go run .           ou  go build -o <nome> .
```

## Source maps

Erros do compilador Go são traduzidos de volta para as linhas do código `.husk` usando anotações `// husk:arquivo:linha` inseridas no Go gerado. Isso funciona automaticamente em `husk run` e `husk build` — erros de compilação mostram o arquivo `.husk` e a linha correta.

---

## Estrutura de projeto recomendada

```
meu-projeto/
├── husk.json             manifesto do projeto (nome, dependências)
├── main.husk             ponto de entrada, rotas e middlewares
├── usuarios.husk         módulo de domínio
├── usuarios_test.husk    testes
├── vendor/               dependências externas (gerado por husk install)
│   └── framework/
│       ├── main.husk
│       └── husk.json
├── .vendor.husk          auto-import gerado por husk install
└── .gitignore            exclui *.go, go.mod, go.sum, vendor/
```

O `.gitignore` gerado por `husk new` já exclui os artefatos Go e o diretório `vendor/`. Não versionar o código transpilado.

---

## Variáveis de ambiente relevantes

| Variável       | Uso                                              |
|----------------|--------------------------------------------------|
| `DATABASE_URL` | Conexão automática quando `husk/postgres` é usado |
| `PORT`         | Porta do servidor (padrão `:8080`)                |
