import * as vscode from 'vscode';
import { activate, deactivate } from '../extension';

// Mock vscode-languageclient module
const mockLanguageClient = {
  start: jest.fn().mockResolvedValue(undefined),
  stop: jest.fn().mockResolvedValue(undefined),
};

jest.mock('vscode-languageclient/node', () => ({
  LanguageClient: jest.fn(() => mockLanguageClient),
  TransportKind: {
    stdio: 0,
  },
}));

describe('CDM Extension', () => {
  let mockContext: vscode.ExtensionContext;
  let mockConfig: any;
  let mockOutputChannel: vscode.OutputChannel;
  let mockFileSystemWatcher: vscode.FileSystemWatcher;
  let registeredCommands: Map<string, Function>;

  beforeEach(() => {
    jest.clearAllMocks();
    registeredCommands = new Map();

    // Mock output channel
    mockOutputChannel = {
      name: 'CDM Language Server Trace',
      append: jest.fn(),
      appendLine: jest.fn(),
      clear: jest.fn(),
      show: jest.fn(),
      hide: jest.fn(),
      dispose: jest.fn(),
      replace: jest.fn(),
    };

    // Mock file system watcher
    mockFileSystemWatcher = {
      onDidChange: jest.fn(),
      onDidCreate: jest.fn(),
      onDidDelete: jest.fn(),
      dispose: jest.fn(),
      ignoreChangeEvents: false,
      ignoreCreateEvents: false,
      ignoreDeleteEvents: false,
    };

    // Mock configuration
    mockConfig = {
      get: jest.fn((key: string) => {
        const defaults: Record<string, any> = {
          'server.path': 'cdm-lsp',
          'trace.server': 'off',
          'validation.checkIds': true,
          'format.indentSize': 2,
        };
        return defaults[key];
      }),
    };

    // Mock workspace
    (vscode.workspace.getConfiguration as jest.Mock) = jest.fn(() => mockConfig);
    (vscode.workspace.createFileSystemWatcher as jest.Mock) = jest.fn(() => mockFileSystemWatcher);

    // Mock window
    (vscode.window.createOutputChannel as jest.Mock) = jest.fn(() => mockOutputChannel);
    (vscode.window.showInformationMessage as jest.Mock) = jest.fn();

    // Mock commands
    (vscode.commands.registerCommand as jest.Mock) = jest.fn((command: string, callback: Function) => {
      registeredCommands.set(command, callback);
      return { dispose: jest.fn() };
    });

    // Mock extension context
    mockContext = {
      subscriptions: [],
      extensionPath: '/test/path',
      globalState: {} as any,
      workspaceState: {} as any,
      extensionUri: {} as any,
      environmentVariableCollection: {} as any,
      asAbsolutePath: jest.fn(),
      storageUri: undefined,
      storagePath: undefined,
      globalStorageUri: {} as any,
      globalStoragePath: '/test/global/storage',
      logUri: {} as any,
      logPath: '/test/log',
      extensionMode: 3,
      extension: {} as any,
      secrets: {} as any,
      languageModelAccessInformation: {} as any,
    };
  });

  describe('activate', () => {
    it('should create and start language client with default configuration', async () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require('vscode-languageclient/node');

      activate(mockContext);

      // Verify configuration was read
      expect(vscode.workspace.getConfiguration).toHaveBeenCalledWith('cdm');
      expect(mockConfig.get).toHaveBeenCalledWith('server.path');
      expect(mockConfig.get).toHaveBeenCalledWith('trace.server');

      // Verify LanguageClient was created with correct parameters
      expect(LanguageClient).toHaveBeenCalledWith(
        'cdm',
        'CDM Language Server',
        expect.objectContaining({
          run: expect.objectContaining({
            command: 'cdm-lsp',
            args: [],
            transport: 0,
          }),
          debug: expect.objectContaining({
            command: 'cdm-lsp',
            args: [],
            transport: 0,
          }),
        }),
        expect.objectContaining({
          documentSelector: [
            { scheme: 'file', language: 'cdm' },
            { scheme: 'untitled', language: 'cdm' },
          ],
          initializationOptions: {
            checkIds: true,
            indentSize: 2,
          },
          outputChannelName: 'CDM Language Server',
        })
      );

      // Verify client was started
      expect(mockLanguageClient.start).toHaveBeenCalled();
    });

    it('should use custom server path from configuration', () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require('vscode-languageclient/node');
      mockConfig.get = jest.fn((key: string) => {
        if (key === 'server.path') {
          return '/custom/path/to/cdm-lsp';
        }
        if (key === 'trace.server') {
          return 'off';
        }
        if (key === 'validation.checkIds') {
          return true;
        }
        if (key === 'format.indentSize') {
          return 2;
        }
        return undefined;
      });

      activate(mockContext);

      expect(LanguageClient).toHaveBeenCalledWith(
        'cdm',
        'CDM Language Server',
        expect.objectContaining({
          run: expect.objectContaining({
            command: '/custom/path/to/cdm-lsp',
          }),
        }),
        expect.any(Object)
      );
    });

    it('should create trace output channel when trace is enabled', () => {
      mockConfig.get = jest.fn((key: string) => {
        if (key === 'trace.server') {
          return 'verbose';
        }
        if (key === 'server.path') {
          return 'cdm-lsp';
        }
        if (key === 'validation.checkIds') {
          return true;
        }
        if (key === 'format.indentSize') {
          return 2;
        }
        return undefined;
      });

      activate(mockContext);

      expect(vscode.window.createOutputChannel).toHaveBeenCalledWith('CDM Language Server Trace');
    });

    it('should not create trace output channel when trace is off', () => {
      activate(mockContext);

      expect(vscode.window.createOutputChannel).not.toHaveBeenCalled();
    });

    it('should register file system watcher for .cdm files', () => {
      activate(mockContext);

      expect(vscode.workspace.createFileSystemWatcher).toHaveBeenCalledWith('**/*.cdm');
    });

    it('should pass initialization options to client', () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require('vscode-languageclient/node');
      mockConfig.get = jest.fn((key: string) => {
        if (key === 'validation.checkIds') {
          return false;
        }
        if (key === 'format.indentSize') {
          return 4;
        }
        if (key === 'server.path') {
          return 'cdm-lsp';
        }
        if (key === 'trace.server') {
          return 'off';
        }
        return undefined;
      });

      activate(mockContext);

      expect(LanguageClient).toHaveBeenCalledWith(
        expect.any(String),
        expect.any(String),
        expect.any(Object),
        expect.objectContaining({
          initializationOptions: {
            checkIds: false,
            indentSize: 4,
          },
        })
      );
    });

    it('should register cdm.restartServer command', () => {
      activate(mockContext);

      expect(vscode.commands.registerCommand).toHaveBeenCalledWith(
        'cdm.restartServer',
        expect.any(Function)
      );
      expect(registeredCommands.has('cdm.restartServer')).toBe(true);
    });

    it('should add command to subscriptions', () => {
      activate(mockContext);

      expect(mockContext.subscriptions.length).toBeGreaterThan(0);
    });
  });

  describe('deactivate', () => {
    it('should stop the language client if it exists', async () => {
      activate(mockContext);

      const result = deactivate();

      expect(result).toBeDefined();
      await result;
      expect(mockLanguageClient.stop).toHaveBeenCalled();
    });
  });

  describe('restartServer command', () => {
    it('should stop and restart the language client', async () => {
      activate(mockContext);

      const restartCommand = registeredCommands.get('cdm.restartServer');
      expect(restartCommand).toBeDefined();

      await restartCommand!();

      expect(mockLanguageClient.stop).toHaveBeenCalled();
      expect(mockLanguageClient.start).toHaveBeenCalledTimes(2); // Once on activate, once on restart
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith('Restarting CDM Language Server...');
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith('CDM Language Server restarted');
    });

    it('should show user feedback during restart', async () => {
      activate(mockContext);

      const restartCommand = registeredCommands.get('cdm.restartServer');
      await restartCommand!();

      expect(vscode.window.showInformationMessage).toHaveBeenNthCalledWith(1, 'Restarting CDM Language Server...');
      expect(vscode.window.showInformationMessage).toHaveBeenNthCalledWith(2, 'CDM Language Server restarted');
    });
  });

  describe('document selector', () => {
    it('should register for file and untitled schemes', () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require('vscode-languageclient/node');

      activate(mockContext);

      // Find the most recent call (since activate may have been called in previous tests)
      const calls = (LanguageClient as jest.Mock).mock.calls;
      const clientOptions = calls[calls.length - 1][3];

      expect(clientOptions.documentSelector).toEqual([
        { scheme: 'file', language: 'cdm' },
        { scheme: 'untitled', language: 'cdm' },
      ]);
    });
  });
});
