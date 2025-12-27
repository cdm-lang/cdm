import * as path from 'path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
  console.log('CDM extension is now active');

  // Get configuration
  const config = vscode.workspace.getConfiguration('cdm');
  const serverPath = config.get<string>('server.path') || 'cdm-lsp';
  const traceLevel = config.get<string>('trace.server') || 'off';

  // Server executable configuration
  const serverExecutable: Executable = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio
  };

  // Server options
  const serverOptions: ServerOptions = {
    run: serverExecutable,
    debug: serverExecutable
  };

  // Client options
  const clientOptions: LanguageClientOptions = {
    // Register the server for CDM documents
    documentSelector: [
      { scheme: 'file', language: 'cdm' },
      { scheme: 'untitled', language: 'cdm' }
    ],
    synchronize: {
      // Notify the server about file changes to .cdm files
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.cdm')
    },
    initializationOptions: {
      checkIds: config.get('validation.checkIds'),
      indentSize: config.get('format.indentSize')
    },
    outputChannelName: 'CDM Language Server',
    traceOutputChannel: traceLevel !== 'off' ? vscode.window.createOutputChannel('CDM Language Server Trace') : undefined
  };

  // Create and start the language client
  client = new LanguageClient(
    'cdm',
    'CDM Language Server',
    serverOptions,
    clientOptions
  );

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('cdm.restartServer', async () => {
      await restartServer();
    })
  );

  // Start the client (this will also launch the server)
  void client.start();

  console.log('CDM Language Server started');
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

async function restartServer() {
  if (client) {
    vscode.window.showInformationMessage('Restarting CDM Language Server...');
    await client.stop();
    await client.start();
    vscode.window.showInformationMessage('CDM Language Server restarted');
  }
}
