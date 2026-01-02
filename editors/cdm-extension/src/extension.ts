import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';
import * as http from 'http';
import * as os from 'os';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;

// Release manifest URL
const RELEASES_URL = 'https://raw.githubusercontent.com/cdm-lang/cdm/main/lsp-releases.json';

interface PlatformInfo {
  url: string;
  checksum: string;
}

interface ReleaseInfo {
  release_date: string;
  platforms: Record<string, PlatformInfo>;
}

interface ReleasesManifest {
  version: number;
  updated_at: string;
  latest: string | null;
  releases: Record<string, ReleaseInfo>;
}

export async function activate(context: vscode.ExtensionContext) {
  outputChannel = vscode.window.createOutputChannel('CDM Language Server');
  outputChannel.appendLine('CDM extension is now active');

  // Register commands first
  context.subscriptions.push(
    vscode.commands.registerCommand('cdm.restartServer', async () => {
      await restartServer();
    })
  );

  // Register on-save handler for auto-assigning entity IDs
  context.subscriptions.push(
    vscode.workspace.onWillSaveTextDocument(async (event) => {
      const config = vscode.workspace.getConfiguration('cdm');
      const assignIdsOnSave = config.get<boolean>('format.assignIdsOnSave');

      if (!assignIdsOnSave) {
        return;
      }

      const document = event.document;
      if (document.languageId !== 'cdm') {
        return;
      }

      event.waitUntil(
        vscode.commands.executeCommand('editor.action.formatDocument')
      );
    })
  );

  // Try to start the language server
  try {
    const serverPath = await resolveServerPath(context);
    if (serverPath) {
      await startLanguageServer(context, serverPath);
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`Failed to start language server: ${message}`);
    vscode.window.showErrorMessage(
      `CDM Language Server failed to start: ${message}`,
      'View Output'
    ).then(selection => {
      if (selection === 'View Output') {
        outputChannel.show();
      }
    });
  }
}

async function resolveServerPath(context: vscode.ExtensionContext): Promise<string | null> {
  const config = vscode.workspace.getConfiguration('cdm');
  const configuredPath = config.get<string>('server.path');

  // 1. Check if user has configured a custom path
  if (configuredPath && configuredPath !== 'cdm-lsp') {
    if (await fileExists(configuredPath)) {
      outputChannel.appendLine(`Using configured server path: ${configuredPath}`);
      return configuredPath;
    } else {
      throw new Error(`Configured server path does not exist: ${configuredPath}`);
    }
  }

  // 2. Check if cdm-lsp is in PATH
  const pathServer = await findInPath('cdm-lsp');
  if (pathServer) {
    outputChannel.appendLine(`Found cdm-lsp in PATH: ${pathServer}`);
    return pathServer;
  }

  // 3. Check if we have a downloaded binary
  const downloadedPath = getDownloadedServerPath(context);
  if (await fileExists(downloadedPath)) {
    outputChannel.appendLine(`Using downloaded server: ${downloadedPath}`);
    return downloadedPath;
  }

  // 4. Download the latest release
  outputChannel.appendLine('CDM Language Server not found. Downloading...');
  return await downloadLatestServer(context);
}

function getDownloadedServerPath(context: vscode.ExtensionContext): string {
  const platform = getPlatformIdentifier();
  const binaryName = platform.includes('windows') ? 'cdm-lsp.exe' : 'cdm-lsp';
  return path.join(context.globalStorageUri.fsPath, 'bin', binaryName);
}

function getPlatformIdentifier(): string {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === 'darwin') {
    return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  } else if (platform === 'linux') {
    return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  } else if (platform === 'win32') {
    return 'x86_64-pc-windows-msvc.exe';
  }

  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

async function downloadLatestServer(context: vscode.ExtensionContext): Promise<string> {
  const progress = await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: 'CDM Language Server',
      cancellable: false
    },
    async (progress) => {
      progress.report({ message: 'Fetching release information...' });

      // Fetch the releases manifest
      const manifest = await fetchReleasesManifest();

      if (!manifest.latest) {
        throw new Error('No releases available. Please install cdm-lsp manually.');
      }

      const release = manifest.releases[manifest.latest];
      if (!release) {
        throw new Error(`Release ${manifest.latest} not found in manifest.`);
      }

      const platform = getPlatformIdentifier();
      const platformInfo = release.platforms[platform];

      if (!platformInfo) {
        throw new Error(
          `No binary available for ${platform}. Please install cdm-lsp manually.\n` +
          `See: https://github.com/cdm-lang/cdm/tree/main/crates/cdm-lsp`
        );
      }

      progress.report({ message: `Downloading v${manifest.latest}...` });

      // Create the bin directory
      const binDir = path.join(context.globalStorageUri.fsPath, 'bin');
      await fs.promises.mkdir(binDir, { recursive: true });

      // Download the binary
      const binaryName = platform.includes('windows') ? 'cdm-lsp.exe' : 'cdm-lsp';
      const destPath = path.join(binDir, binaryName);

      await downloadFile(platformInfo.url, destPath);

      // Make executable on Unix
      if (os.platform() !== 'win32') {
        await fs.promises.chmod(destPath, 0o755);
      }

      progress.report({ message: 'Done!' });
      outputChannel.appendLine(`Downloaded CDM Language Server v${manifest.latest} to ${destPath}`);

      return destPath;
    }
  );

  return progress;
}

async function fetchReleasesManifest(): Promise<ReleasesManifest> {
  return new Promise((resolve, reject) => {
    https.get(RELEASES_URL, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        // Handle redirect
        const redirectUrl = res.headers.location;
        if (redirectUrl) {
          https.get(redirectUrl, (redirectRes) => {
            handleResponse(redirectRes, resolve, reject);
          }).on('error', reject);
          return;
        }
      }
      handleResponse(res, resolve, reject);
    }).on('error', reject);
  });
}

function handleResponse(
  res: http.IncomingMessage,
  resolve: (value: ReleasesManifest) => void,
  reject: (reason: Error) => void
) {
  if (res.statusCode !== 200) {
    reject(new Error(`Failed to fetch releases: HTTP ${res.statusCode}`));
    return;
  }

  let data = '';
  res.on('data', (chunk: Buffer | string) => { data += chunk; });
  res.on('end', () => {
    try {
      resolve(JSON.parse(data));
    } catch (e) {
      reject(new Error('Failed to parse releases manifest'));
    }
  });
}

async function downloadFile(url: string, destPath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);

    const request = (urlString: string) => {
      https.get(urlString, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          // Handle redirect
          const redirectUrl = res.headers.location;
          if (redirectUrl) {
            request(redirectUrl);
            return;
          }
        }

        if (res.statusCode !== 200) {
          file.close();
          fs.unlinkSync(destPath);
          reject(new Error(`Failed to download: HTTP ${res.statusCode}`));
          return;
        }

        res.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      }).on('error', (err) => {
        file.close();
        fs.unlinkSync(destPath);
        reject(err);
      });
    };

    request(url);
  });
}

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.promises.access(filePath, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

async function findInPath(binaryName: string): Promise<string | null> {
  const pathEnv = process.env.PATH || '';
  const pathSeparator = os.platform() === 'win32' ? ';' : ':';
  const paths = pathEnv.split(pathSeparator);

  const extensions = os.platform() === 'win32' ? ['', '.exe', '.cmd', '.bat'] : [''];

  for (const dir of paths) {
    for (const ext of extensions) {
      const fullPath = path.join(dir, binaryName + ext);
      if (await fileExists(fullPath)) {
        return fullPath;
      }
    }
  }

  return null;
}

async function startLanguageServer(context: vscode.ExtensionContext, serverPath: string) {
  const config = vscode.workspace.getConfiguration('cdm');
  const traceLevel = config.get<string>('trace.server') || 'off';

  const serverExecutable: Executable = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio
  };

  const serverOptions: ServerOptions = {
    run: serverExecutable,
    debug: serverExecutable
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'cdm' },
      { scheme: 'untitled', language: 'cdm' }
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.cdm')
    },
    initializationOptions: {
      checkIds: config.get('validation.checkIds'),
      indentSize: config.get('format.indentSize'),
      assignIdsOnSave: config.get('format.assignIdsOnSave')
    },
    outputChannel: outputChannel,
    traceOutputChannel: traceLevel !== 'off' ? vscode.window.createOutputChannel('CDM Language Server Trace') : undefined
  };

  client = new LanguageClient(
    'cdm',
    'CDM Language Server',
    serverOptions,
    clientOptions
  );

  await client.start();
  outputChannel.appendLine('CDM Language Server started');
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
