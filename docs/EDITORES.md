# Suporte a Editores

## VS Code

O Husk possui uma extensão oficial para VS Code em `editors/vscode/`.

### Funcionalidades

- Syntax highlighting (TextMate grammar)
- Diagnósticos via LSP (erros de sintaxe, tipo, escopo)
- Comandos `Husk: Check` e `Husk: Run` no palette (Ctrl+Shift+P)

### Pré-requisitos

O `husk-lsp` precisa estar no PATH:

```bash
cargo install --path crates/husk-lsp --force
```

Isso instala o binário em `~/.cargo/bin/husk-lsp`.

### Instalação da extensão

```bash
cd editors/vscode
npm install
npx @vscode/vsce package
code --install-extension husk-*.vsix
```

### Configuração

| Propriedade       | Padrão      | Descrição                     |
|-------------------|-------------|-------------------------------|
| `husk.lsp.path`   | `husk-lsp`  | Caminho para o binário LSP    |

## LSP (genérico)

O servidor LSP (`husk-lsp`) funciona com qualquer editor compatível via stdio:

```bash
husk-lsp
```

- **Neovim**: `vim.lsp.start({ cmd = { "husk-lsp" }, ... })`
- **Emacs**: via `eglot` ou `lsp-mode`
- **Sublime Text**: via pacote LSP

## GitHub Linguist

Para que o GitHub reconheça `.husk` como linguagem, é necessário submeter um
[PR ao projeto Linguist](https://github.com/github-linguist/linguist).

### Pré-requisitos

1. **Grammar**: O repositório [husk-grammar](https://github.com/Vime-Labs/husk-grammar)
   contém a TextMate grammar (`source.husk`) com licença MIT.
2. **Samples**: Amostras de código real em `samples/` na raiz do projecto.
3. **Adopção**: Pelo menos 200 ficheiros `.husk` no GitHub (excluindo forks).

### Passos para submeter

1. Adicionar entrada no `languages.yml` do Linguist:
   ```yaml
   Husk:
     type: programming
     color: "#8B5CF6"
     extensions: [".husk"]
     tm_scope: source.husk
     ace_mode: text
     language_id: (gerado por script/update-ids)
   ```
2. Executar `script/add-grammar https://github.com/Vime-Labs/husk-grammar`
   para registar a grammar.
3. Adicionar os samples de `samples/` ao diretório `samples/Husk/` do Linguist.
4. Abrir PR seguindo o template oficial.
5. Associar uma cor (ex: `#8B5CF6` — roxo, alinhado com a identidade visual).
6. Anexar resultado de busca no GitHub que comprove adopção:
   `https://github.com/search?type=code&q=NOT+is%3Afork+path%3A*.husk`
