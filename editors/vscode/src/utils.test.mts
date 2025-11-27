import { describe, it, expect } from "vitest";
import {
  isTypeScriptFile,
  isJavaScriptFile,
  isSupportedFile,
  getLanguageId,
  isValidPath,
  getDefaultServerPaths,
  DOCUMENT_SELECTORS,
  WATCH_PATTERN,
  CONFIG_NAMESPACE,
  CLIENT_ID,
  CLIENT_NAME,
  OUTPUT_CHANNEL_NAME,
  SERVER_BINARY_NAME,
} from "./utils.mjs";

describe("Constants", () => {
  it("should have correct document selectors", () => {
    expect(DOCUMENT_SELECTORS).toHaveLength(4);
    expect(DOCUMENT_SELECTORS[0]).toEqual({
      scheme: "file",
      language: "typescript",
    });
    expect(DOCUMENT_SELECTORS[1]).toEqual({
      scheme: "file",
      language: "typescriptreact",
    });
    expect(DOCUMENT_SELECTORS[2]).toEqual({
      scheme: "file",
      language: "javascript",
    });
    expect(DOCUMENT_SELECTORS[3]).toEqual({
      scheme: "file",
      language: "javascriptreact",
    });
  });

  it("should have correct watch pattern", () => {
    expect(WATCH_PATTERN).toBe("**/*.{ts,tsx,js,jsx,mts,cts,mjs,cjs}");
  });

  it("should have correct configuration namespace", () => {
    expect(CONFIG_NAMESPACE).toBe("tsLspRust");
  });

  it("should have correct client ID", () => {
    expect(CLIENT_ID).toBe("tsLspRust");
  });

  it("should have correct client name", () => {
    expect(CLIENT_NAME).toBe("TypeScript/JavaScript LSP (Rust)");
  });

  it("should have correct output channel name", () => {
    expect(OUTPUT_CHANNEL_NAME).toBe("TypeScript/JavaScript LSP (Rust)");
  });

  it("should have correct server binary name", () => {
    if (process.platform === "win32") {
      expect(SERVER_BINARY_NAME).toBe("typescript-language-server.exe");
    } else {
      expect(SERVER_BINARY_NAME).toBe("typescript-language-server");
    }
  });
});

describe("isTypeScriptFile", () => {
  it("should return true for .ts files", () => {
    expect(isTypeScriptFile("file.ts")).toBe(true);
    expect(isTypeScriptFile("path/to/file.ts")).toBe(true);
    expect(isTypeScriptFile("FILE.TS")).toBe(true);
  });

  it("should return true for .tsx files", () => {
    expect(isTypeScriptFile("file.tsx")).toBe(true);
    expect(isTypeScriptFile("component.tsx")).toBe(true);
  });

  it("should return true for .mts files", () => {
    expect(isTypeScriptFile("module.mts")).toBe(true);
  });

  it("should return true for .cts files", () => {
    expect(isTypeScriptFile("commonjs.cts")).toBe(true);
  });

  it("should return false for JavaScript files", () => {
    expect(isTypeScriptFile("file.js")).toBe(false);
    expect(isTypeScriptFile("file.jsx")).toBe(false);
    expect(isTypeScriptFile("file.mjs")).toBe(false);
    expect(isTypeScriptFile("file.cjs")).toBe(false);
  });

  it("should return false for other files", () => {
    expect(isTypeScriptFile("file.txt")).toBe(false);
    expect(isTypeScriptFile("file.json")).toBe(false);
    expect(isTypeScriptFile("file.html")).toBe(false);
    expect(isTypeScriptFile("file")).toBe(false);
  });
});

describe("isJavaScriptFile", () => {
  it("should return true for .js files", () => {
    expect(isJavaScriptFile("file.js")).toBe(true);
    expect(isJavaScriptFile("path/to/file.js")).toBe(true);
    expect(isJavaScriptFile("FILE.JS")).toBe(true);
  });

  it("should return true for .jsx files", () => {
    expect(isJavaScriptFile("file.jsx")).toBe(true);
    expect(isJavaScriptFile("component.jsx")).toBe(true);
  });

  it("should return true for .mjs files", () => {
    expect(isJavaScriptFile("module.mjs")).toBe(true);
  });

  it("should return true for .cjs files", () => {
    expect(isJavaScriptFile("commonjs.cjs")).toBe(true);
  });

  it("should return false for TypeScript files", () => {
    expect(isJavaScriptFile("file.ts")).toBe(false);
    expect(isJavaScriptFile("file.tsx")).toBe(false);
    expect(isJavaScriptFile("file.mts")).toBe(false);
    expect(isJavaScriptFile("file.cts")).toBe(false);
  });

  it("should return false for other files", () => {
    expect(isJavaScriptFile("file.txt")).toBe(false);
    expect(isJavaScriptFile("file.json")).toBe(false);
    expect(isJavaScriptFile("file")).toBe(false);
  });
});

describe("isSupportedFile", () => {
  it("should return true for TypeScript files", () => {
    expect(isSupportedFile("file.ts")).toBe(true);
    expect(isSupportedFile("file.tsx")).toBe(true);
    expect(isSupportedFile("file.mts")).toBe(true);
    expect(isSupportedFile("file.cts")).toBe(true);
  });

  it("should return true for JavaScript files", () => {
    expect(isSupportedFile("file.js")).toBe(true);
    expect(isSupportedFile("file.jsx")).toBe(true);
    expect(isSupportedFile("file.mjs")).toBe(true);
    expect(isSupportedFile("file.cjs")).toBe(true);
  });

  it("should return false for unsupported files", () => {
    expect(isSupportedFile("file.txt")).toBe(false);
    expect(isSupportedFile("file.json")).toBe(false);
    expect(isSupportedFile("file.html")).toBe(false);
    expect(isSupportedFile("file.css")).toBe(false);
    expect(isSupportedFile("file.py")).toBe(false);
    expect(isSupportedFile("file")).toBe(false);
  });
});

describe("getLanguageId", () => {
  it("should return typescript for .ts files", () => {
    expect(getLanguageId("file.ts")).toBe("typescript");
    expect(getLanguageId("file.mts")).toBe("typescript");
    expect(getLanguageId("file.cts")).toBe("typescript");
  });

  it("should return typescriptreact for .tsx files", () => {
    expect(getLanguageId("file.tsx")).toBe("typescriptreact");
    expect(getLanguageId("component.tsx")).toBe("typescriptreact");
  });

  it("should return javascript for .js files", () => {
    expect(getLanguageId("file.js")).toBe("javascript");
    expect(getLanguageId("file.mjs")).toBe("javascript");
    expect(getLanguageId("file.cjs")).toBe("javascript");
  });

  it("should return javascriptreact for .jsx files", () => {
    expect(getLanguageId("file.jsx")).toBe("javascriptreact");
    expect(getLanguageId("component.jsx")).toBe("javascriptreact");
  });

  it("should return undefined for unsupported files", () => {
    expect(getLanguageId("file.txt")).toBeUndefined();
    expect(getLanguageId("file.json")).toBeUndefined();
    expect(getLanguageId("file")).toBeUndefined();
  });

  it("should be case insensitive", () => {
    expect(getLanguageId("FILE.TS")).toBe("typescript");
    expect(getLanguageId("FILE.TSX")).toBe("typescriptreact");
    expect(getLanguageId("FILE.JS")).toBe("javascript");
    expect(getLanguageId("FILE.JSX")).toBe("javascriptreact");
  });
});

describe("isValidPath", () => {
  it("should return true for non-empty strings", () => {
    expect(isValidPath("/path/to/server")).toBe(true);
    expect(isValidPath("./relative/path")).toBe(true);
    expect(isValidPath("server")).toBe(true);
  });

  it("should return false for empty strings", () => {
    expect(isValidPath("")).toBe(false);
  });

  it("should return false for undefined", () => {
    expect(isValidPath(undefined)).toBe(false);
  });

  it("should return false for null", () => {
    expect(isValidPath(null)).toBe(false);
  });
});

describe("getDefaultServerPaths", () => {
  it("should return an array of paths", () => {
    const paths = getDefaultServerPaths("/workspace/project");
    expect(Array.isArray(paths)).toBe(true);
    expect(paths.length).toBeGreaterThan(0);
  });

  it("should include debug paths", () => {
    const paths = getDefaultServerPaths("/workspace/project");
    const hasDebugPath = paths.some((p) => p.includes("target/debug"));
    expect(hasDebugPath).toBe(true);
  });

  it("should include release paths", () => {
    const paths = getDefaultServerPaths("/workspace/project");
    const hasReleasePath = paths.some((p) => p.includes("target/release"));
    expect(hasReleasePath).toBe(true);
  });

  it("should include the server binary name", () => {
    const paths = getDefaultServerPaths("/workspace/project");
    const allHaveBinaryName = paths.every((p) =>
      p.includes("typescript-language-server")
    );
    expect(allHaveBinaryName).toBe(true);
  });

  it("should use workspace root in paths", () => {
    const workspaceRoot = "/my/custom/workspace";
    const paths = getDefaultServerPaths(workspaceRoot);
    const hasWorkspaceRoot = paths.some((p) => p.includes(workspaceRoot));
    expect(hasWorkspaceRoot).toBe(true);
  });
});

