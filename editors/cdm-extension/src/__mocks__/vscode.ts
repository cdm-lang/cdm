export const window = {
  showInformationMessage: jest.fn(),
  createOutputChannel: jest.fn(() => ({
    appendLine: jest.fn(),
    show: jest.fn(),
  })),
};

export const workspace = {
  getConfiguration: jest.fn(() => ({
    get: jest.fn((key: string) => {
      if (key === 'server.path') return 'cdm-lsp';
      if (key === 'trace.server') return 'off';
      return undefined;
    }),
  })),
  onDidChangeConfiguration: jest.fn(),
};

export const commands = {
  registerCommand: jest.fn(),
};

export const Uri = {
  file: jest.fn((path: string) => ({ fsPath: path })),
};
