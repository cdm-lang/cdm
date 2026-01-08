import * as fs from "fs";
import * as path from "path";
import * as vsctm from "vscode-textmate";
import * as oniguruma from "vscode-oniguruma";

// Load the oniguruma WASM binary
const wasmPath = path.join(
  __dirname,
  "../../node_modules/vscode-oniguruma/release/onig.wasm"
);
const wasmBin = fs.readFileSync(wasmPath).buffer;

// Initialize the registry with oniguruma
let registry: vsctm.Registry;

beforeAll(async () => {
  await oniguruma.loadWASM(wasmBin);

  registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner(patterns: string[]) {
        return new oniguruma.OnigScanner(patterns);
      },
      createOnigString(s: string) {
        return new oniguruma.OnigString(s);
      },
    }),
    loadGrammar: async (scopeName: string) => {
      if (scopeName === "source.cdm") {
        const grammarPath = path.join(
          __dirname,
          "../../syntaxes/cdm.tmLanguage.json"
        );
        const grammarContent = fs.readFileSync(grammarPath, "utf-8");
        return vsctm.parseRawGrammar(grammarContent, grammarPath);
      }
      return null;
    },
  });
});

interface TokenInfo {
  text: string;
  scopes: string[];
}

async function tokenizeLine(line: string): Promise<TokenInfo[]> {
  const grammar = await registry.loadGrammar("source.cdm");
  if (!grammar) {
    throw new Error("Failed to load grammar");
  }

  const result = grammar.tokenizeLine(line, vsctm.INITIAL);
  const tokens: TokenInfo[] = [];

  for (const token of result.tokens) {
    tokens.push({
      text: line.substring(token.startIndex, token.endIndex),
      scopes: token.scopes,
    });
  }

  return tokens;
}

function findTokenWithText(tokens: TokenInfo[], text: string): TokenInfo | undefined {
  return tokens.find((t) => t.text === text);
}

function hasScope(token: TokenInfo | undefined, scope: string): boolean {
  return token?.scopes.some((s) => s.includes(scope)) ?? false;
}

describe("CDM Grammar - Import Statements", () => {
  describe("template imports with local paths", () => {
    it("should tokenize 'import pg from ../templates/sql-types/postgres.cdm'", async () => {
      const tokens = await tokenizeLine(
        "import pg from ../templates/sql-types/postgres.cdm"
      );

      // Find each token
      const importToken = findTokenWithText(tokens, "import");
      const pgToken = findTokenWithText(tokens, "pg");
      const fromToken = findTokenWithText(tokens, "from");
      const pathToken = findTokenWithText(
        tokens,
        "../templates/sql-types/postgres.cdm"
      );

      // Verify scopes
      expect(hasScope(importToken, "keyword.control.import")).toBe(true);
      expect(hasScope(pgToken, "variable.other.readwrite.alias")).toBe(true);
      expect(hasScope(fromToken, "keyword.control.import")).toBe(true);
      expect(hasScope(pathToken, "string.quoted.module")).toBe(true);
    });

    it("should tokenize 'import local from ./templates/shared'", async () => {
      const tokens = await tokenizeLine("import local from ./templates/shared");

      const importToken = findTokenWithText(tokens, "import");
      const localToken = findTokenWithText(tokens, "local");
      const fromToken = findTokenWithText(tokens, "from");
      const pathToken = findTokenWithText(tokens, "./templates/shared");

      expect(hasScope(importToken, "keyword.control.import")).toBe(true);
      expect(hasScope(localToken, "variable.other.readwrite.alias")).toBe(true);
      expect(hasScope(fromToken, "keyword.control.import")).toBe(true);
      expect(hasScope(pathToken, "string.quoted.module")).toBe(true);
    });
  });

  describe("template imports with registry names", () => {
    it("should tokenize 'import sql from sql/postgres-types'", async () => {
      const tokens = await tokenizeLine("import sql from sql/postgres-types");

      const importToken = findTokenWithText(tokens, "import");
      const sqlToken = findTokenWithText(tokens, "sql");
      const fromToken = findTokenWithText(tokens, "from");
      const nameToken = findTokenWithText(tokens, "sql/postgres-types");

      expect(hasScope(importToken, "keyword.control.import")).toBe(true);
      expect(hasScope(sqlToken, "variable.other.readwrite.alias")).toBe(true);
      expect(hasScope(fromToken, "keyword.control.import")).toBe(true);
      expect(hasScope(nameToken, "string.quoted.module")).toBe(true);
    });
  });

  describe("template imports with git source", () => {
    it("should tokenize 'import custom from git:https://github.com/org/repo.git'", async () => {
      const tokens = await tokenizeLine(
        "import custom from git:https://github.com/org/repo.git"
      );

      const importToken = findTokenWithText(tokens, "import");
      const customToken = findTokenWithText(tokens, "custom");
      const fromToken = findTokenWithText(tokens, "from");
      const gitToken = findTokenWithText(tokens, "git:");
      const urlToken = findTokenWithText(
        tokens,
        "https://github.com/org/repo.git"
      );

      expect(hasScope(importToken, "keyword.control.import")).toBe(true);
      expect(hasScope(customToken, "variable.other.readwrite.alias")).toBe(true);
      expect(hasScope(fromToken, "keyword.control.import")).toBe(true);
      expect(hasScope(gitToken, "keyword.control.import")).toBe(true);
      expect(hasScope(urlToken, "string.quoted.module")).toBe(true);
    });
  });
});

describe("CDM Grammar - Extends Directive", () => {
  it("should tokenize 'extends ./base.cdm'", async () => {
    const tokens = await tokenizeLine("extends ./base.cdm");

    const extendsToken = findTokenWithText(tokens, "extends");
    const pathToken = findTokenWithText(tokens, "./base.cdm");

    expect(hasScope(extendsToken, "keyword.control.import")).toBe(true);
    expect(hasScope(pathToken, "string.quoted.module")).toBe(true);
  });

  it("should tokenize 'extends cdm/auth'", async () => {
    const tokens = await tokenizeLine("extends cdm/auth");

    const extendsToken = findTokenWithText(tokens, "extends");
    const nameToken = findTokenWithText(tokens, "cdm/auth");

    expect(hasScope(extendsToken, "keyword.control.import")).toBe(true);
    expect(hasScope(nameToken, "string.quoted.module")).toBe(true);
  });
});

describe("CDM Grammar - Expected Color Mapping", () => {
  /**
   * These tests document the expected scope-to-color mapping:
   * - keyword.control.import -> pink/magenta (like TypeScript imports)
   * - variable.other.readwrite.alias -> light blue (variable names)
   * - string.quoted.module -> orange (string/path)
   *
   * The actual colors depend on the VS Code theme, but these scopes
   * should map to the same colors as TypeScript imports.
   */
  it("should use TypeScript-compatible scopes for import statements", async () => {
    const tokens = await tokenizeLine(
      "import pg from ../templates/sql-types/postgres.cdm"
    );

    // Print tokens for debugging
    // console.log("Tokens:", JSON.stringify(tokens, null, 2));

    const importToken = findTokenWithText(tokens, "import");
    const pgToken = findTokenWithText(tokens, "pg");
    const fromToken = findTokenWithText(tokens, "from");
    const pathToken = findTokenWithText(
      tokens,
      "../templates/sql-types/postgres.cdm"
    );

    // keyword.control.import is the standard scope for import keywords
    // This maps to pink/magenta in most themes (like TypeScript)
    expect(importToken?.scopes).toContain("keyword.control.import.cdm");
    expect(fromToken?.scopes).toContain("keyword.control.import.cdm");

    // variable.other.readwrite is used for variable names
    // This maps to light blue in most themes
    expect(pgToken?.scopes).toContain("variable.other.readwrite.alias.cdm");

    // string.quoted is used for strings
    // This maps to orange/brown in most themes
    expect(pathToken?.scopes).toContain("string.quoted.module.cdm");
  });
});
