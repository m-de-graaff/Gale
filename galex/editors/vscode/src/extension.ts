import * as path from 'path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;

export function activate(context: vscode.ExtensionContext) {
  outputChannel = vscode.window.createOutputChannel('GaleX Language Server');
  context.subscriptions.push(outputChannel);

  // Status bar
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    -100
  );
  statusBarItem.name = 'GaleX';
  statusBarItem.command = 'gale.showOutput';
  context.subscriptions.push(statusBarItem);

  // Commands
  context.subscriptions.push(
    vscode.commands.registerCommand('gale.restartServer', () =>
      restartServer(context)
    )
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('gale.showOutput', () =>
      outputChannel.show()
    )
  );

  startServer(context);
}

export function deactivate(): Thenable<void> | undefined {
  statusBarItem?.dispose();
  return stopServer();
}

async function startServer(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration('gale');
  const serverPath = resolveServerPath(config, context);

  if (!serverPath) {
    setStatus('error', 'gale-lsp not found');
    const action = await vscode.window.showErrorMessage(
      'GaleX: Could not find the `gale-lsp` binary. Install it or set `gale.lspPath` in settings.',
      'Open Settings',
      'Dismiss'
    );
    if (action === 'Open Settings') {
      vscode.commands.executeCommand(
        'workbench.action.openSettings',
        'gale.lspPath'
      );
    }
    return;
  }

  setStatus('starting', 'Starting...');
  outputChannel.appendLine(`Starting GaleX language server: ${serverPath}`);

  const traceLevel = config.get<string>('trace.server', 'off');

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

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'gale' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.gx'),
    },
    outputChannel,
    traceOutputChannel: outputChannel,
    initializationOptions: {
      trace: traceLevel,
    },
  };

  client = new LanguageClient(
    'gale',
    'GaleX Language Server',
    serverOptions,
    clientOptions
  );

  try {
    await client.start();
    setStatus('ready', 'GaleX');
    outputChannel.appendLine('GaleX language server started successfully');
  } catch (err) {
    setStatus('error', 'Failed to start');
    outputChannel.appendLine(`Failed to start server: ${err}`);
    vscode.window.showErrorMessage(
      `GaleX: Language server failed to start. Check the output channel for details.`
    );
  }
}

async function stopServer(): Promise<void> {
  if (client) {
    try {
      await client.stop();
    } catch {
      // Ignore stop errors
    }
    client = undefined;
  }
}

async function restartServer(context: vscode.ExtensionContext) {
  setStatus('starting', 'Restarting...');
  outputChannel.appendLine('Restarting GaleX language server...');
  await stopServer();
  await startServer(context);
}

function resolveServerPath(
  config: vscode.WorkspaceConfiguration,
  context: vscode.ExtensionContext
): string | undefined {
  const configured = config.get<string>('lspPath', '');

  // 1. Explicit user-configured path
  if (configured && configured !== 'gale-lsp') {
    return configured;
  }

  // 2. Bundled with extension
  const ext = process.platform === 'win32' ? '.exe' : '';
  const bundled = path.join(context.extensionPath, 'server', `gale-lsp${ext}`);
  try {
    const fs = require('fs');
    if (fs.existsSync(bundled)) {
      return bundled;
    }
  } catch {
    // Ignore
  }

  // 3. On PATH (default)
  return 'gale-lsp';
}

type ServerStatus = 'starting' | 'ready' | 'error';

function setStatus(status: ServerStatus, text: string) {
  switch (status) {
    case 'starting':
      statusBarItem.text = `$(loading~spin) ${text}`;
      statusBarItem.backgroundColor = undefined;
      statusBarItem.tooltip = 'GaleX Language Server is starting...';
      break;
    case 'ready':
      statusBarItem.text = `$(check) ${text}`;
      statusBarItem.backgroundColor = undefined;
      statusBarItem.tooltip =
        'GaleX Language Server is running. Click to show output.';
      break;
    case 'error':
      statusBarItem.text = `$(error) ${text}`;
      statusBarItem.backgroundColor = new vscode.ThemeColor(
        'statusBarItem.errorBackground'
      );
      statusBarItem.tooltip =
        'GaleX Language Server encountered an error. Click to show output.';
      break;
  }
  statusBarItem.show();
}
