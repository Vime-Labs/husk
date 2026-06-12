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
