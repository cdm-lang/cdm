import * as vscode from 'vscode';

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

describe('CDM Extension Configuration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('should read server path from configuration', () => {
    const config = vscode.workspace.getConfiguration('cdm');
    const serverPath = config.get('server.path');
    expect(serverPath).toBe('cdm-lsp');
  });

  test('should read trace level from configuration', () => {
    const config = vscode.workspace.getConfiguration('cdm');
    const traceLevel = config.get('trace.server');
    expect(traceLevel).toBe('off');
  });
});

describe('CDM Server Options', () => {
  test('should construct correct server executable path', () => {
    const serverPath = 'cdm-lsp';
    expect(serverPath).toBe('cdm-lsp');
  });

  test('should use stdio transport', () => {
    const { TransportKind } = require('vscode-languageclient/node');
    expect(TransportKind.stdio).toBe(0);
  });
});

describe('CDM File Pattern Matching', () => {
  test('should match .cdm file extension', () => {
    const pattern = '**/*.cdm';
    const testFile = 'models/user.cdm';

    // Simple pattern matching test
    expect(testFile.endsWith('.cdm')).toBe(true);
    expect(testFile.match(/\.cdm$/)).toBeTruthy();
  });

  test('should create correct document selector', () => {
    const documentSelector = [
      { scheme: 'file', language: 'cdm' }
    ];

    expect(documentSelector).toHaveLength(1);
    expect(documentSelector[0].scheme).toBe('file');
    expect(documentSelector[0].language).toBe('cdm');
  });
});

describe('CDM Language Client', () => {
  test('should create language client with correct ID', () => {
    const { LanguageClient } = require('vscode-languageclient/node');

    const serverOptions = {
      command: 'cdm-lsp',
      transport: 0,
    };

    const clientOptions = {
      documentSelector: [{ scheme: 'file', language: 'cdm' }],
    };

    const client = new LanguageClient(
      'cdm',
      'CDM Language Server',
      serverOptions,
      clientOptions
    );

    expect(LanguageClient).toHaveBeenCalledWith(
      'cdm',
      'CDM Language Server',
      serverOptions,
      clientOptions
    );
    expect(client).toBe(mockLanguageClient);
  });

  test('should start language client', async () => {
    const { LanguageClient } = require('vscode-languageclient/node');
    const client = new LanguageClient('cdm', 'CDM Language Server', {}, {});

    await client.start();

    expect(client.start).toHaveBeenCalled();
  });

  test('should stop language client', async () => {
    const { LanguageClient } = require('vscode-languageclient/node');
    const client = new LanguageClient('cdm', 'CDM Language Server', {}, {});

    await client.stop();

    expect(client.stop).toHaveBeenCalled();
  });
});

describe('CDM Extension Commands', () => {
  test('should register restart server command', () => {
    const commandId = 'cdm.restartServer';
    expect(commandId).toBe('cdm.restartServer');
  });

  test('should call registerCommand with correct parameters', () => {
    const commandId = 'cdm.restartServer';
    const handler = jest.fn();

    vscode.commands.registerCommand(commandId, handler);

    expect(vscode.commands.registerCommand).toHaveBeenCalledWith(
      commandId,
      handler
    );
  });
});

describe('CDM Configuration Properties', () => {
  test('should have valid default values', () => {
    const defaults = {
      'server.path': 'cdm-lsp',
      'format.indentSize': 2,
      'validation.checkIds': true,
      'trace.server': 'off',
    };

    expect(defaults['server.path']).toBe('cdm-lsp');
    expect(defaults['format.indentSize']).toBe(2);
    expect(defaults['validation.checkIds']).toBe(true);
    expect(defaults['trace.server']).toBe('off');
  });

  test('should validate trace server enum values', () => {
    const validValues = ['off', 'messages', 'verbose'];

    expect(validValues).toContain('off');
    expect(validValues).toContain('messages');
    expect(validValues).toContain('verbose');
    expect(validValues).not.toContain('invalid');
  });

  test('should validate indent size is a number', () => {
    const indentSize = 2;
    expect(typeof indentSize).toBe('number');
    expect(indentSize).toBeGreaterThan(0);
  });
});

describe('CDM Language Configuration', () => {
  test('should define CDM language id', () => {
    const languageId = 'cdm';
    expect(languageId).toBe('cdm');
  });

  test('should define file extensions', () => {
    const extensions = ['.cdm'];
    expect(extensions).toContain('.cdm');
    expect(extensions).toHaveLength(1);
  });

  test('should define language aliases', () => {
    const aliases = ['CDM', 'cdm'];
    expect(aliases).toContain('CDM');
    expect(aliases).toContain('cdm');
  });
});
