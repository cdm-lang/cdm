import * as vscode from "vscode";
import { activate, deactivate } from "../extension";

// Mock vscode-languageclient module
const mockLanguageClient = {
  start: jest.fn().mockResolvedValue(undefined),
  stop: jest.fn().mockResolvedValue(undefined),
};

jest.mock("vscode-languageclient/node", () => ({
  LanguageClient: jest.fn(() => mockLanguageClient),
}));

// Mock fs module
jest.mock("fs", () => ({
  ...jest.requireActual("fs"),
  promises: {
    access: jest.fn().mockRejectedValue(new Error("Not found")),
    mkdir: jest.fn().mockResolvedValue(undefined),
    readFile: jest.fn().mockRejectedValue(new Error("Not found")),
    writeFile: jest.fn().mockResolvedValue(undefined),
  },
  createWriteStream: jest.fn(() => ({
    on: jest.fn(),
    close: jest.fn(),
  })),
}));

// Mock https module
jest.mock("https", () => ({
  get: jest.fn(),
}));

// Mock os module
jest.mock("os", () => ({
  platform: jest.fn().mockReturnValue("darwin"),
  arch: jest.fn().mockReturnValue("arm64"),
}));

describe("CDM Extension", () => {
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
      name: "CDM Language Server",
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

    // Mock configuration - default returns 'cdm' for cli.path (will use PATH lookup)
    mockConfig = {
      get: jest.fn((key: string) => {
        const defaults: Record<string, any> = {
          "cli.path": "cdm",
          "trace.server": "off",
          "validation.checkIds": true,
          "format.indentSize": 2,
          "format.assignIdsOnSave": false,
        };
        return defaults[key];
      }),
    };

    // Mock workspace
    (vscode.workspace.getConfiguration as jest.Mock) = jest.fn(
      () => mockConfig
    );
    (vscode.workspace.createFileSystemWatcher as jest.Mock) = jest.fn(
      () => mockFileSystemWatcher
    );
    (vscode.workspace.onWillSaveTextDocument as jest.Mock) = jest.fn(
      () => ({ dispose: jest.fn() })
    );

    // Mock window
    (vscode.window.createOutputChannel as jest.Mock) = jest.fn(
      () => mockOutputChannel
    );
    (vscode.window.showInformationMessage as jest.Mock) = jest.fn();
    (vscode.window.showErrorMessage as jest.Mock) = jest.fn().mockResolvedValue(undefined);
    (vscode.window.withProgress as jest.Mock) = jest.fn();

    // Mock commands
    (vscode.commands.registerCommand as jest.Mock) = jest.fn(
      (command: string, callback: Function) => {
        registeredCommands.set(command, callback);
        return { dispose: jest.fn() };
      }
    );

    // Mock extension context
    mockContext = {
      subscriptions: [],
      extensionPath: "/test/path",
      globalState: {} as any,
      workspaceState: {} as any,
      extensionUri: {} as any,
      environmentVariableCollection: {} as any,
      asAbsolutePath: jest.fn(),
      storageUri: undefined,
      storagePath: undefined,
      globalStorageUri: { fsPath: "/test/global/storage" } as any,
      globalStoragePath: "/test/global/storage",
      logUri: {} as any,
      logPath: "/test/log",
      extensionMode: 3,
      extension: {} as any,
      secrets: {} as any,
      languageModelAccessInformation: {} as any,
    };
  });

  describe("activate", () => {
    it("should create output channel on activation", async () => {
      await activate(mockContext);

      expect(vscode.window.createOutputChannel).toHaveBeenCalledWith(
        "CDM Language Server"
      );
    });

    it("should register cdm.restartServer command", async () => {
      await activate(mockContext);

      expect(vscode.commands.registerCommand).toHaveBeenCalledWith(
        "cdm.restartServer",
        expect.any(Function)
      );
      expect(registeredCommands.has("cdm.restartServer")).toBe(true);
    });

    it("should register cdm.updateCli command", async () => {
      await activate(mockContext);

      expect(vscode.commands.registerCommand).toHaveBeenCalledWith(
        "cdm.updateCli",
        expect.any(Function)
      );
      expect(registeredCommands.has("cdm.updateCli")).toBe(true);
    });

    it("should register cdm.build command", async () => {
      await activate(mockContext);

      expect(vscode.commands.registerCommand).toHaveBeenCalledWith(
        "cdm.build",
        expect.any(Function)
      );
      expect(registeredCommands.has("cdm.build")).toBe(true);
    });

    it("should register cdm.migrate command", async () => {
      await activate(mockContext);

      expect(vscode.commands.registerCommand).toHaveBeenCalledWith(
        "cdm.migrate",
        expect.any(Function)
      );
      expect(registeredCommands.has("cdm.migrate")).toBe(true);
    });

    it("should register onWillSaveTextDocument handler", async () => {
      await activate(mockContext);

      expect(vscode.workspace.onWillSaveTextDocument).toHaveBeenCalled();
    });

    it("should read cli.path from configuration", async () => {
      await activate(mockContext);

      expect(vscode.workspace.getConfiguration).toHaveBeenCalledWith("cdm");
      expect(mockConfig.get).toHaveBeenCalledWith("cli.path");
    });

    it("should add subscriptions to context", async () => {
      await activate(mockContext);

      expect(mockContext.subscriptions.length).toBeGreaterThan(0);
    });

    it("should use custom cli path from configuration when file exists", async () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require("vscode-languageclient/node");
      const fs = require("fs");

      // Mock custom path exists
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/custom/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      expect(LanguageClient).toHaveBeenCalledWith(
        "cdm",
        "CDM Language Server",
        expect.objectContaining({
          run: expect.objectContaining({
            command: "/custom/path/to/cdm",
            args: ["lsp"],
          }),
        }),
        expect.any(Object)
      );
    });

    it("should pass lsp argument to server command", async () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require("vscode-languageclient/node");
      const fs = require("fs");

      // Mock custom path exists
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      expect(LanguageClient).toHaveBeenCalledWith(
        expect.any(String),
        expect.any(String),
        expect.objectContaining({
          run: expect.objectContaining({
            args: ["lsp"],
          }),
          debug: expect.objectContaining({
            args: ["lsp"],
          }),
        }),
        expect.any(Object)
      );
    });

    it("should create trace output channel when trace is enabled", async () => {
      const fs = require("fs");
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "trace.server") return "verbose";
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      // Should create both the main output channel and trace channel
      expect(vscode.window.createOutputChannel).toHaveBeenCalledWith(
        "CDM Language Server"
      );
      expect(vscode.window.createOutputChannel).toHaveBeenCalledWith(
        "CDM Language Server Trace"
      );
    });

    it("should pass initialization options to client", async () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require("vscode-languageclient/node");
      const fs = require("fs");

      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "validation.checkIds") return false;
        if (key === "format.indentSize") return 4;
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "format.assignIdsOnSave") return true;
        return undefined;
      });

      await activate(mockContext);

      expect(LanguageClient).toHaveBeenCalledWith(
        expect.any(String),
        expect.any(String),
        expect.any(Object),
        expect.objectContaining({
          initializationOptions: {
            checkIds: false,
            indentSize: 4,
            assignIdsOnSave: true,
          },
        })
      );
    });

    it("should register file system watcher for .cdm files", async () => {
      const fs = require("fs");
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      expect(vscode.workspace.createFileSystemWatcher).toHaveBeenCalledWith(
        "**/*.cdm"
      );
    });
  });

  describe("deactivate", () => {
    it("should stop the language client if it exists", async () => {
      const fs = require("fs");
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      const result = deactivate();

      expect(result).toBeDefined();
      await result;
      expect(mockLanguageClient.stop).toHaveBeenCalled();
    });

    it("should suppress errors during shutdown", async () => {
      const fs = require("fs");
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      // Mock stop to reject (simulates "connection disposed" error)
      mockLanguageClient.stop.mockRejectedValueOnce(
        new Error("Pending response rejected since connection got disposed")
      );

      await activate(mockContext);

      // Should not throw even if stop() rejects
      const result = deactivate();
      await expect(result).resolves.toBeUndefined();
    });
  });

  describe("restartServer command", () => {
    it("should stop and restart the language client", async () => {
      const fs = require("fs");
      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      const restartCommand = registeredCommands.get("cdm.restartServer");
      expect(restartCommand).toBeDefined();

      await restartCommand!();

      expect(mockLanguageClient.stop).toHaveBeenCalled();
      expect(mockLanguageClient.start).toHaveBeenCalledTimes(2); // Once on activate, once on restart
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
        "Restarting CDM Language Server..."
      );
      expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
        "CDM Language Server restarted"
      );
    });
  });

  describe("document selector", () => {
    it("should register for file and untitled schemes", async () => {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { LanguageClient } = require("vscode-languageclient/node");
      const fs = require("fs");

      fs.promises.access.mockResolvedValueOnce(undefined);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "cli.path") return "/path/to/cdm";
        if (key === "trace.server") return "off";
        if (key === "validation.checkIds") return true;
        if (key === "format.indentSize") return 2;
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      await activate(mockContext);

      const calls = (LanguageClient as jest.Mock).mock.calls;
      const clientOptions = calls[calls.length - 1][3];

      expect(clientOptions.documentSelector).toEqual([
        { scheme: "file", language: "cdm" },
        { scheme: "untitled", language: "cdm" },
      ]);
    });
  });

  describe("onWillSaveTextDocument handler", () => {
    let onWillSaveHandler: Function;
    let mockTextDocumentWillSaveEvent: any;

    beforeEach(() => {
      // Capture the handler function
      (vscode.workspace.onWillSaveTextDocument as jest.Mock) = jest.fn(
        (handler: Function) => {
          onWillSaveHandler = handler;
          return { dispose: jest.fn() };
        }
      );

      // Mock executeCommand
      (vscode.commands.executeCommand as jest.Mock) = jest.fn().mockResolvedValue(undefined);
    });

    it("should register onWillSaveTextDocument handler", async () => {
      await activate(mockContext);

      expect(vscode.workspace.onWillSaveTextDocument).toHaveBeenCalled();
      expect(onWillSaveHandler).toBeDefined();
    });

    it("should do nothing when assignIdsOnSave is false", async () => {
      await activate(mockContext);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "format.assignIdsOnSave") return false;
        return undefined;
      });

      mockTextDocumentWillSaveEvent = {
        document: {
          languageId: "cdm",
        },
        waitUntil: jest.fn(),
      };

      await onWillSaveHandler(mockTextDocumentWillSaveEvent);

      expect(mockTextDocumentWillSaveEvent.waitUntil).not.toHaveBeenCalled();
    });

    it("should do nothing when document is not CDM", async () => {
      await activate(mockContext);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "format.assignIdsOnSave") return true;
        return undefined;
      });

      mockTextDocumentWillSaveEvent = {
        document: {
          languageId: "javascript",
        },
        waitUntil: jest.fn(),
      };

      await onWillSaveHandler(mockTextDocumentWillSaveEvent);

      expect(mockTextDocumentWillSaveEvent.waitUntil).not.toHaveBeenCalled();
    });

    it("should format document when assignIdsOnSave is true and document is CDM", async () => {
      await activate(mockContext);

      mockConfig.get = jest.fn((key: string) => {
        if (key === "format.assignIdsOnSave") return true;
        return undefined;
      });

      mockTextDocumentWillSaveEvent = {
        document: {
          languageId: "cdm",
        },
        waitUntil: jest.fn(),
      };

      await onWillSaveHandler(mockTextDocumentWillSaveEvent);

      expect(mockTextDocumentWillSaveEvent.waitUntil).toHaveBeenCalledWith(
        expect.any(Promise)
      );
      expect(vscode.commands.executeCommand).toHaveBeenCalledWith(
        "editor.action.formatDocument"
      );
    });
  });

  describe("deactivate when client is undefined", () => {
    it("should return undefined when client has not been initialized", () => {
      // Reset the module to clear the client variable
      jest.resetModules();

      // Re-import to get fresh module state
      const { deactivate: freshDeactivate } = require("../extension");

      const result = freshDeactivate();

      expect(result).toBeUndefined();
    });
  });
});
