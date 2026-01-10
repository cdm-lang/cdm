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
      if (key === 'server.path') return 'cdm';
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

export const languages = {
  registerCodeActionsProvider: jest.fn(),
};

export const CodeActionKind = {
  QuickFix: { value: 'quickfix' },
};

export class CodeAction {
  title: string;
  kind: typeof CodeActionKind.QuickFix;
  command?: { command: string; title: string; arguments?: unknown[] };
  isPreferred?: boolean;

  constructor(title: string, kind: typeof CodeActionKind.QuickFix) {
    this.title = title;
    this.kind = kind;
  }
}
