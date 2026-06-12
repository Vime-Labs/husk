const path = require("path");
const { commands, window, workspace } = require("vscode");
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;

function activate() {
  const binaryPath =
    workspace.getConfiguration("husk").get("lsp.path") ?? "husk-lsp";

  const serverOptions = {
    command: binaryPath,
    args: [],
    transport: TransportKind.stdio,
  };

  const clientOptions = {
    documentSelector: [{ language: "husk" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.husk"),
    },
  };

  client = new LanguageClient("husk", "Husk Language Server", serverOptions, clientOptions);
  client.registerProposedFeatures();
  client.start();

  commands.registerCommand("husk.check", () => {
    const editor = window.activeTextEditor;
    if (!editor) return;
    const uri = editor.document.uri.fsPath;
    const terminal = window.createTerminal("Husk Check");
    terminal.sendText(`husk check "${uri}"`);
    terminal.show();
  });

  commands.registerCommand("husk.run", () => {
    const editor = window.activeTextEditor;
    if (!editor) return;
    const uri = editor.document.uri.fsPath;
    const terminal = window.createTerminal("Husk Run");
    terminal.sendText(`husk run "${uri}"`);
    terminal.show();
  });
}

function deactivate() {
  return client?.stop();
}

module.exports = { activate, deactivate };
