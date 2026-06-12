import * as path from "path";
import { commands, window, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(): void {
  const binaryPath =
    workspace.getConfiguration("husk").get<string>("lsp.path") ?? "husk-lsp";

  const serverOptions: ServerOptions = {
    command: binaryPath,
    args: ["lsp"],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "husk" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.husk"),
    },
  };

  client = new LanguageClient("husk", "Husk Language Server", serverOptions, clientOptions);
  client.registerProposedFeatures();
  client.start();

  commands.registerCommand("husk.check", async () => {
    const editor = window.activeTextEditor;
    if (!editor) return;
    const uri = editor.document.uri.fsPath;
    const terminal = window.createTerminal("Husk Check");
    terminal.sendText(`husk check "${uri}"`);
    terminal.show();
  });

  commands.registerCommand("husk.run", async () => {
    const editor = window.activeTextEditor;
    if (!editor) return;
    const uri = editor.document.uri.fsPath;
    const terminal = window.createTerminal("Husk Run");
    terminal.sendText(`husk run "${uri}"`);
    terminal.show();
  });
}

export function deactivate(): Promise<void> | undefined {
  return client?.stop();
}
