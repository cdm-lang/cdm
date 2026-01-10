import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';
import * as http from 'http';
import * as os from 'os';
import * as child_process from 'child_process';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let resolvedCliPath: string | null = null;

// Capabilities result from CLI
interface CapabilitiesResult {
  plugins: Array<{
    name: string;
    has_build: boolean;
    has_migrate: boolean;
  }>;
  can_build: boolean;
  can_migrate: boolean;
}

// Release manifest URL - uses cli-releases.json since cdm CLI now includes the LSP
const RELEASES_URL = 'https://raw.githubusercontent.com/cdm-lang/cdm/main/cli-releases.json';

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

  // Register commands
  outputChannel.appendLine('Registering commands...');
  context.subscriptions.push(
    vscode.commands.registerCommand('cdm.restartServer', async () => {
      await restartServer();
    }),
    vscode.commands.registerCommand('cdm.updateCli', async () => {
      await updateCli(context);
    }),
    vscode.commands.registerCommand('cdm.build', async () => {
      await runBuild();
    }),
    vscode.commands.registerCommand('cdm.migrate', async () => {
      await runMigrate();
    }),
    vscode.commands.registerCommand('cdm.downloadPlugin', async (...args: unknown[]) => {
      outputChannel.appendLine(`cdm.downloadPlugin called with args: ${JSON.stringify(args)}`);
      const pluginName = args[0] as string;
      outputChannel.appendLine(`Extracted pluginName: ${pluginName}`);
      await downloadPlugin(pluginName);
    }),
    vscode.commands.registerCommand('cdm.downloadAllPlugins', async () => {
      outputChannel.appendLine('cdm.downloadAllPlugins called');
      await downloadAllPlugins();
    })
  );
  outputChannel.appendLine('Commands registered: cdm.restartServer, cdm.updateCli, cdm.build, cdm.migrate, cdm.downloadPlugin, cdm.downloadAllPlugins');

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

  // Update capabilities context when active editor changes
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(async (editor) => {
      await updateCapabilitiesContext(editor);
    })
  );

  // Update capabilities context when document is saved (plugins might change)
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(async (document) => {
      if (document.languageId === 'cdm' && vscode.window.activeTextEditor?.document === document) {
        await updateCapabilitiesContext(vscode.window.activeTextEditor);
      }
    })
  );

  // Set initial capabilities context for current editor
  await updateCapabilitiesContext(vscode.window.activeTextEditor);

  // Try to start the language server
  try {
    resolvedCliPath = await resolveServerPath(context);
    if (resolvedCliPath) {
      await startLanguageServer(context, resolvedCliPath);
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
  const configuredPath = config.get<string>('cli.path');

  outputChannel.appendLine('--- Resolving CDM server path ---');
  outputChannel.appendLine(`Configured cdm.cli.path: "${configuredPath || '(not set)'}"`);

  // 1. Check if user has configured a custom path
  if (configuredPath && configuredPath !== 'cdm') {
    outputChannel.appendLine(`Checking custom path: ${configuredPath}`);
    if (await fileExists(configuredPath)) {
      outputChannel.appendLine(`✓ Using configured server path: ${configuredPath}`);
      return configuredPath;
    } else {
      throw new Error(`Configured server path does not exist: ${configuredPath}`);
    }
  }

  // 2. Check if cdm is in PATH
  outputChannel.appendLine('Searching for cdm in PATH...');
  const pathServer = await findInPath('cdm');
  if (pathServer) {
    outputChannel.appendLine(`✓ Found cdm in PATH: ${pathServer}`);
    return pathServer;
  }
  outputChannel.appendLine('✗ cdm not found in PATH');

  // 3. Check if we have a downloaded binary
  const downloadedPath = getDownloadedServerPath(context);
  outputChannel.appendLine(`Checking for downloaded binary: ${downloadedPath}`);
  if (await fileExists(downloadedPath)) {
    outputChannel.appendLine(`✓ Using downloaded server: ${downloadedPath}`);
    return downloadedPath;
  }
  outputChannel.appendLine('✗ No downloaded binary found');

  // 4. Download the latest release
  outputChannel.appendLine('Attempting to download CDM Language Server...');
  return await downloadLatestServer(context);
}

function getDownloadedServerPath(context: vscode.ExtensionContext): string {
  return path.join(context.globalStorageUri.fsPath, 'bin', getBinaryName());
}

function getBinaryName(): string {
  return os.platform() === 'win32' ? 'cdm.exe' : 'cdm';
}

async function downloadAndInstallBinary(
  context: vscode.ExtensionContext,
  platformInfo: PlatformInfo
): Promise<string> {
  const binDir = path.join(context.globalStorageUri.fsPath, 'bin');
  await fs.promises.mkdir(binDir, { recursive: true });

  const destPath = path.join(binDir, getBinaryName());
  await downloadFile(platformInfo.url, destPath);

  if (os.platform() !== 'win32') {
    await fs.promises.chmod(destPath, 0o755);
  }

  return destPath;
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
        throw new Error('No releases available. Please install cdm manually.');
      }

      const release = manifest.releases[manifest.latest];
      if (!release) {
        throw new Error(`Release ${manifest.latest} not found in manifest.`);
      }

      const platform = getPlatformIdentifier();
      const platformInfo = release.platforms[platform];

      if (!platformInfo) {
        throw new Error(
          `No binary available for ${platform}. Please install cdm manually.\n` +
          `See: https://github.com/cdm-lang/cdm`
        );
      }

      progress.report({ message: `Downloading v${manifest.latest}...` });

      const destPath = await downloadAndInstallBinary(context, platformInfo);

      // Save the version
      await saveCurrentCliVersion(context, manifest.latest);

      progress.report({ message: 'Done!' });
      outputChannel.appendLine(`Downloaded CDM CLI v${manifest.latest} to ${destPath}`);

      return destPath;
    }
  );

  return progress;
}

async function getCurrentCliVersion(context: vscode.ExtensionContext): Promise<string | null> {
  const versionFile = path.join(context.globalStorageUri.fsPath, 'version');
  try {
    const version = await fs.promises.readFile(versionFile, 'utf-8');
    return version.trim();
  } catch {
    return null;
  }
}

async function saveCurrentCliVersion(context: vscode.ExtensionContext, version: string): Promise<void> {
  const versionFile = path.join(context.globalStorageUri.fsPath, 'version');
  await fs.promises.mkdir(context.globalStorageUri.fsPath, { recursive: true });
  await fs.promises.writeFile(versionFile, version, 'utf-8');
}

async function updateCli(context: vscode.ExtensionContext): Promise<void> {
  try {
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'CDM CLI',
        cancellable: false
      },
      async (progress) => {
        progress.report({ message: 'Checking for updates...' });

        // Fetch the releases manifest
        const manifest = await fetchReleasesManifest();

        if (!manifest.latest) {
          vscode.window.showInformationMessage('No CDM CLI releases available.');
          return;
        }

        // Check current version
        const currentVersion = await getCurrentCliVersion(context);

        if (currentVersion === manifest.latest) {
          vscode.window.showInformationMessage(
            `CDM CLI is already up to date (v${currentVersion}).`
          );
          return;
        }

        const updateMessage = currentVersion
          ? `Update available: v${currentVersion} → v${manifest.latest}`
          : `Installing CDM CLI v${manifest.latest}`;

        progress.report({ message: updateMessage });

        // Stop the current server if running
        if (client) {
          await client.stop();
        }

        // Download the new version
        const release = manifest.releases[manifest.latest];
        const platform = getPlatformIdentifier();
        const platformInfo = release.platforms[platform];

        if (!platformInfo) {
          throw new Error(`No binary available for ${platform}`);
        }

        progress.report({ message: `Downloading v${manifest.latest}...` });

        const destPath = await downloadAndInstallBinary(context, platformInfo);

        // Save the version
        await saveCurrentCliVersion(context, manifest.latest);

        // Update the resolved path
        resolvedCliPath = destPath;

        // Restart the server
        progress.report({ message: 'Restarting server...' });
        await startLanguageServer(context, destPath);

        vscode.window.showInformationMessage(
          `CDM CLI updated to v${manifest.latest}`
        );
      }
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Failed to update CDM CLI: ${message}`);
  }
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

  outputChannel.appendLine(`PATH contains ${paths.length} directories`);
  outputChannel.appendLine(`PATH: ${pathEnv}`);

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

  outputChannel.appendLine('--- Starting Language Server ---');
  outputChannel.appendLine(`Server path: ${serverPath}`);
  outputChannel.appendLine(`Command: ${serverPath} lsp`);

  const serverExecutable: Executable = {
    command: serverPath,
    args: ['lsp']  // Run the LSP subcommand (uses stdio by default)
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

  try {
    await client.start();
    outputChannel.appendLine('✓ CDM Language Server started successfully');
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`✗ Failed to start Language Server: ${message}`);
    throw error;
  }
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  // Suppress "connection disposed" errors during shutdown - these are expected
  // when pending requests exist at deactivation time
  return client.stop().catch(() => {});
}

async function restartServer() {
  if (client) {
    vscode.window.showInformationMessage('Restarting CDM Language Server...');
    await client.stop();
    await client.start();
    vscode.window.showInformationMessage('CDM Language Server restarted');
  }
}

/**
 * Refresh plugin caches in the LSP without restarting the server
 */
async function refreshPlugins(): Promise<void> {
  if (!client) {
    outputChannel.appendLine('Cannot refresh plugins: LSP client not initialized');
    return;
  }

  try {
    outputChannel.appendLine('Sending cdm.refreshPlugins command to LSP...');
    await client.sendRequest('workspace/executeCommand', {
      command: 'cdm.refreshPlugins'
    });
    outputChannel.appendLine('Plugin cache refreshed');
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`Failed to refresh plugins: ${message}`);
    // Fall back to restart if the command fails
    outputChannel.appendLine('Falling back to server restart...');
    await restartServer();
  }
}

async function runBuild(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'cdm') {
    vscode.window.showErrorMessage('Please open a CDM file to build');
    return;
  }

  // Save the document first
  await editor.document.save();

  const filePath = editor.document.uri.fsPath;

  if (!resolvedCliPath) {
    vscode.window.showErrorMessage('CDM CLI not found. Please install it first.');
    return;
  }

  outputChannel.appendLine(`--- Running build ---`);
  outputChannel.appendLine(`File: ${filePath}`);
  outputChannel.appendLine(`Command: ${resolvedCliPath} build "${filePath}"`);
  outputChannel.show();

  try {
    const result = await runCliCommand(resolvedCliPath, ['build', filePath]);
    if (result.exitCode === 0) {
      vscode.window.showInformationMessage('Build completed successfully');
      outputChannel.appendLine('✓ Build completed successfully');
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
    } else {
      vscode.window.showErrorMessage('Build failed. See output for details.');
      outputChannel.appendLine('✗ Build failed');
      if (result.stderr) {
        outputChannel.appendLine(result.stderr);
      }
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Build failed: ${message}`);
    outputChannel.appendLine(`✗ Build error: ${message}`);
  }
}

async function runMigrate(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'cdm') {
    vscode.window.showErrorMessage('Please open a CDM file to generate migration');
    return;
  }

  // Prompt for migration name
  const migrationName = await vscode.window.showInputBox({
    prompt: 'Enter migration name',
    placeHolder: 'e.g., add_user_email_field',
    validateInput: (value) => {
      if (!value || value.trim().length === 0) {
        return 'Migration name is required';
      }
      if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(value)) {
        return 'Migration name must be alphanumeric with underscores (e.g., add_user_field)';
      }
      return null;
    }
  });

  if (!migrationName) {
    return; // User cancelled
  }

  // Save the document first
  await editor.document.save();

  const filePath = editor.document.uri.fsPath;

  if (!resolvedCliPath) {
    vscode.window.showErrorMessage('CDM CLI not found. Please install it first.');
    return;
  }

  outputChannel.appendLine(`--- Running migrate ---`);
  outputChannel.appendLine(`File: ${filePath}`);
  outputChannel.appendLine(`Migration name: ${migrationName}`);
  outputChannel.appendLine(`Command: ${resolvedCliPath} migrate "${filePath}" -n ${migrationName}`);
  outputChannel.show();

  try {
    const result = await runCliCommand(resolvedCliPath, ['migrate', filePath, '-n', migrationName]);
    if (result.exitCode === 0) {
      vscode.window.showInformationMessage(`Migration '${migrationName}' generated successfully`);
      outputChannel.appendLine('✓ Migration generated successfully');
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
    } else {
      vscode.window.showErrorMessage('Migration failed. See output for details.');
      outputChannel.appendLine('✗ Migration failed');
      if (result.stderr) {
        outputChannel.appendLine(result.stderr);
      }
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Migration failed: ${message}`);
    outputChannel.appendLine(`✗ Migration error: ${message}`);
  }
}

interface CliResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

function runCliCommand(cliPath: string, args: string[]): Promise<CliResult> {
  return new Promise((resolve, reject) => {
    const process = child_process.spawn(cliPath, args, {
      cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
    });

    let stdout = '';
    let stderr = '';

    process.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    process.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    process.on('error', (error) => {
      reject(error);
    });

    process.on('close', (code) => {
      resolve({
        exitCode: code ?? 1,
        stdout,
        stderr
      });
    });
  });
}

/**
 * Download a specific plugin using the CDM CLI
 */
async function downloadPlugin(pluginName: string): Promise<void> {
  if (!resolvedCliPath) {
    vscode.window.showErrorMessage('CDM CLI not found. Please install it first.');
    return;
  }

  // Validate plugin name
  if (!pluginName || typeof pluginName !== 'string' || pluginName.trim() === '') {
    outputChannel.appendLine(`ERROR: Invalid plugin name received: ${JSON.stringify(pluginName)}`);
    vscode.window.showErrorMessage('Invalid plugin name. Cannot download.');
    return;
  }

  outputChannel.appendLine(`--- Downloading plugin: ${pluginName} ---`);
  outputChannel.appendLine(`Plugin name type: ${typeof pluginName}, value: "${pluginName}"`);
  outputChannel.appendLine(`Command: ${resolvedCliPath} plugin cache ${pluginName}`);
  outputChannel.show();

  try {
    const result = await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: `Downloading plugin '${pluginName}'...`,
        cancellable: false
      },
      async () => {
        return await runCliCommand(resolvedCliPath!, ['plugin', 'cache', pluginName]);
      }
    );

    if (result.exitCode === 0) {
      vscode.window.showInformationMessage(`Plugin '${pluginName}' downloaded successfully`);
      outputChannel.appendLine(`✓ Plugin '${pluginName}' downloaded successfully`);
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
      // Refresh the LSP's plugin cache to pick up the new plugin
      await refreshPlugins();
    } else {
      vscode.window.showErrorMessage(`Failed to download plugin '${pluginName}'. See output for details.`);
      outputChannel.appendLine(`✗ Failed to download plugin '${pluginName}'`);
      if (result.stderr) {
        outputChannel.appendLine(result.stderr);
      }
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Failed to download plugin: ${message}`);
    outputChannel.appendLine(`✗ Download error: ${message}`);
  }
}

/**
 * Download all missing plugins using cdm plugin cache --all
 */
async function downloadAllPlugins(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'cdm') {
    vscode.window.showErrorMessage('Please open a CDM file first');
    return;
  }

  if (!resolvedCliPath) {
    vscode.window.showErrorMessage('CDM CLI not found. Please install it first.');
    return;
  }

  // Save the document first
  await editor.document.save();

  const filePath = editor.document.uri.fsPath;

  outputChannel.appendLine(`--- Downloading all plugins ---`);
  outputChannel.appendLine(`File: ${filePath}`);
  outputChannel.appendLine(`Command: ${resolvedCliPath} plugin cache --all`);
  outputChannel.show();

  try {
    const result = await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'Downloading plugins...',
        cancellable: false
      },
      async () => {
        return await runCliCommand(resolvedCliPath!, ['plugin', 'cache', '--all']);
      }
    );

    if (result.exitCode === 0) {
      vscode.window.showInformationMessage('All plugins cached successfully');
      outputChannel.appendLine('✓ All plugins cached');
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
      // Refresh the LSP's plugin cache to pick up the new plugins
      await refreshPlugins();
    } else {
      vscode.window.showErrorMessage('Failed to cache plugins. See output for details.');
      outputChannel.appendLine('✗ Failed to cache plugins');
      if (result.stderr) {
        outputChannel.appendLine(result.stderr);
      }
      if (result.stdout) {
        outputChannel.appendLine(result.stdout);
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    vscode.window.showErrorMessage(`Failed to cache plugins: ${message}`);
    outputChannel.appendLine(`✗ Cache error: ${message}`);
  }
}

/**
 * Update VS Code context keys based on plugin capabilities for the current file.
 * This controls visibility of build/migrate buttons in the editor title bar.
 */
async function updateCapabilitiesContext(editor: vscode.TextEditor | undefined): Promise<void> {
  // Default to hiding buttons if no CDM file is open
  if (!editor || editor.document.languageId !== 'cdm') {
    await vscode.commands.executeCommand('setContext', 'cdm.canBuild', false);
    await vscode.commands.executeCommand('setContext', 'cdm.canMigrate', false);
    return;
  }

  // If CLI is not available yet, hide buttons
  if (!resolvedCliPath) {
    await vscode.commands.executeCommand('setContext', 'cdm.canBuild', false);
    await vscode.commands.executeCommand('setContext', 'cdm.canMigrate', false);
    return;
  }

  const filePath = editor.document.uri.fsPath;

  try {
    const result = await runCliCommand(resolvedCliPath, ['capabilities', filePath]);

    if (result.exitCode === 0 && result.stdout) {
      const capabilities: CapabilitiesResult = JSON.parse(result.stdout);
      await vscode.commands.executeCommand('setContext', 'cdm.canBuild', capabilities.can_build);
      await vscode.commands.executeCommand('setContext', 'cdm.canMigrate', capabilities.can_migrate);
    } else {
      // On error, hide buttons
      await vscode.commands.executeCommand('setContext', 'cdm.canBuild', false);
      await vscode.commands.executeCommand('setContext', 'cdm.canMigrate', false);
    }
  } catch {
    // On error, hide buttons
    await vscode.commands.executeCommand('setContext', 'cdm.canBuild', false);
    await vscode.commands.executeCommand('setContext', 'cdm.canMigrate', false);
  }
}
