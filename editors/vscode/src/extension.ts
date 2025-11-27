import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("tsLspRust");

  // Get server path from configuration or use default
  let serverPath = config.get<string>("serverPath");

  if (!serverPath) {
    // Default: look for the server binary relative to the extension
    // In development, this would be in the target/debug or target/release directory
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (workspaceRoot) {
      // Try to find the server in the workspace's target directory
      serverPath = path.join(
        workspaceRoot,
        "..",
        "..",
        "target",
        "debug",
        "typescript-language-server"
      );
    }
  }

  if (!serverPath) {
    vscode.window.showErrorMessage(
      "TypeScript LSP (Rust): Server path not configured. Please set tsLspRust.serverPath in settings."
    );
    return;
  }

  // Server options - run the Rust binary via stdio
  const serverOptions: ServerOptions = {
    run: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
    debug: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
  };

  // Client options - which documents to handle
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "typescript" },
      { scheme: "file", language: "typescriptreact" },
    ],
    synchronize: {
      // Watch for TypeScript file changes
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{ts,tsx}"),
    },
    outputChannelName: "TypeScript LSP (Rust)",
  };

  // Create the language client
  client = new LanguageClient(
    "tsLspRust",
    "TypeScript LSP (Rust)",
    serverOptions,
    clientOptions
  );

  // Start the client (also starts the server)
  await client.start();

  vscode.window.showInformationMessage(
    "TypeScript LSP (Rust) server started!"
  );
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

