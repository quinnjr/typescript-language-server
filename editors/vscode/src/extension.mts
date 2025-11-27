import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  outputChannel = vscode.window.createOutputChannel("TypeScript/JavaScript LSP (Rust)");
  context.subscriptions.push(outputChannel);

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand("tsLspRust.restartServer", async () => {
      await restartServer(context);
    })
  );

  // Start the language server
  await startServer(context);
}

async function startServer(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration("tsLspRust");

  // Check if extension is enabled
  if (!config.get<boolean>("enable", true)) {
    outputChannel.appendLine("TypeScript/JavaScript LSP (Rust) is disabled.");
    return;
  }

  // Get server path from configuration or find it automatically
  const serverPath = await resolveServerPath(context, config);

  if (!serverPath) {
    const message = "TypeScript/JavaScript LSP (Rust): Could not find language server. Please set tsLspRust.serverPath in settings.";
    outputChannel.appendLine(message);
    vscode.window.showErrorMessage(message);
    return;
  }

  outputChannel.appendLine(`Using server at: ${serverPath}`);

  // Server options - run the Rust binary via stdio
  const serverOptions: ServerOptions = {
    run: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
    debug: {
      command: serverPath,
      transport: TransportKind.stdio,
      options: {
        env: {
          ...process.env,
          RUST_LOG: "debug",
          RUST_BACKTRACE: "1",
        },
      },
    },
  };

  // Client options - which documents to handle
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "typescript" },
      { scheme: "file", language: "typescriptreact" },
      { scheme: "file", language: "javascript" },
      { scheme: "file", language: "javascriptreact" },
    ],
    synchronize: {
      // Watch for TypeScript and JavaScript file changes
      fileEvents: vscode.workspace.createFileSystemWatcher(
        "**/*.{ts,tsx,js,jsx,mts,cts,mjs,cjs}"
      ),
    },
    outputChannel,
    traceOutputChannel: outputChannel,
  };

  // Create the language client
  client = new LanguageClient(
    "tsLspRust",
    "TypeScript/JavaScript LSP (Rust)",
    serverOptions,
    clientOptions
  );

  // Start the client (also starts the server)
  try {
    await client.start();
    outputChannel.appendLine("Language server started successfully.");
  } catch (error) {
    const message = `Failed to start language server: ${error}`;
    outputChannel.appendLine(message);
    vscode.window.showErrorMessage(message);
  }
}

async function stopServer(): Promise<void> {
  if (client) {
    try {
      await client.stop();
      outputChannel.appendLine("Language server stopped.");
    } catch (error) {
      outputChannel.appendLine(`Error stopping server: ${error}`);
    }
    client = undefined;
  }
}

async function restartServer(context: vscode.ExtensionContext): Promise<void> {
  outputChannel.appendLine("Restarting language server...");
  await stopServer();
  await startServer(context);
  vscode.window.showInformationMessage("TypeScript/JavaScript LSP (Rust) server restarted.");
}

async function resolveServerPath(
  context: vscode.ExtensionContext,
  config: vscode.WorkspaceConfiguration
): Promise<string | undefined> {
  // 1. Check user-configured path
  const configuredPath = config.get<string>("serverPath");
  if (configuredPath && fs.existsSync(configuredPath)) {
    return configuredPath;
  }

  // 2. Check for bundled server in extension
  const bundledPaths = [
    path.join(context.extensionPath, "server", "typescript-language-server"),
    path.join(context.extensionPath, "server", "typescript-language-server.exe"),
  ];

  for (const bundledPath of bundledPaths) {
    if (fs.existsSync(bundledPath)) {
      return bundledPath;
    }
  }

  // 3. Check workspace's target directory (development mode)
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (workspaceRoot) {
    const devPaths = [
      // Release build
      path.join(workspaceRoot, "..", "..", "target", "release", "typescript-language-server"),
      // Debug build
      path.join(workspaceRoot, "..", "..", "target", "debug", "typescript-language-server"),
      // Windows variants
      path.join(workspaceRoot, "..", "..", "target", "release", "typescript-language-server.exe"),
      path.join(workspaceRoot, "..", "..", "target", "debug", "typescript-language-server.exe"),
    ];

    for (const devPath of devPaths) {
      const resolved = path.resolve(devPath);
      if (fs.existsSync(resolved)) {
        return resolved;
      }
    }
  }

  // 4. Check if server is in PATH
  const serverName = process.platform === "win32" 
    ? "typescript-language-server.exe" 
    : "typescript-language-server";
  
  const pathDirs = (process.env.PATH || "").split(path.delimiter);
  for (const dir of pathDirs) {
    const fullPath = path.join(dir, serverName);
    if (fs.existsSync(fullPath)) {
      return fullPath;
    }
  }

  return undefined;
}

export async function deactivate(): Promise<void> {
  await stopServer();
}
