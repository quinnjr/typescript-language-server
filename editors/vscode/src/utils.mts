/**
 * Utility functions for the TypeScript/JavaScript LSP extension
 */

/**
 * Document selectors for TypeScript and JavaScript files
 */
export const DOCUMENT_SELECTORS = [
  { scheme: "file", language: "typescript" },
  { scheme: "file", language: "typescriptreact" },
  { scheme: "file", language: "javascript" },
  { scheme: "file", language: "javascriptreact" },
] as const;

/**
 * File extensions to watch for changes
 */
export const WATCH_PATTERN = "**/*.{ts,tsx,js,jsx,mts,cts,mjs,cjs}";

/**
 * Configuration namespace
 */
export const CONFIG_NAMESPACE = "tsLspRust";

/**
 * Language client ID
 */
export const CLIENT_ID = "tsLspRust";

/**
 * Language client name
 */
export const CLIENT_NAME = "TypeScript/JavaScript LSP (Rust)";

/**
 * Output channel name
 */
export const OUTPUT_CHANNEL_NAME = "TypeScript/JavaScript LSP (Rust)";

/**
 * Check if a file extension is a TypeScript file
 */
export function isTypeScriptFile(filename: string): boolean {
  return /\.(ts|tsx|mts|cts)$/i.test(filename);
}

/**
 * Check if a file extension is a JavaScript file
 */
export function isJavaScriptFile(filename: string): boolean {
  return /\.(js|jsx|mjs|cjs)$/i.test(filename);
}

/**
 * Check if a file extension is a supported language file
 */
export function isSupportedFile(filename: string): boolean {
  return isTypeScriptFile(filename) || isJavaScriptFile(filename);
}

/**
 * Get the language ID for a file based on its extension
 */
export function getLanguageId(filename: string): string | undefined {
  if (/\.tsx$/i.test(filename)) return "typescriptreact";
  if (/\.(ts|mts|cts)$/i.test(filename)) return "typescript";
  if (/\.jsx$/i.test(filename)) return "javascriptreact";
  if (/\.(js|mjs|cjs)$/i.test(filename)) return "javascript";
  return undefined;
}

/**
 * Validate server path exists and is executable
 */
export function isValidPath(path: string | undefined | null): path is string {
  return typeof path === "string" && path.length > 0;
}

/**
 * Default server binary name
 */
export const SERVER_BINARY_NAME =
  process.platform === "win32"
    ? "typescript-language-server.exe"
    : "typescript-language-server";

/**
 * Get the default server paths to search
 */
export function getDefaultServerPaths(workspaceRoot: string): string[] {
  const paths: string[] = [];

  // Debug build path
  paths.push(
    `${workspaceRoot}/../../target/debug/${SERVER_BINARY_NAME}`,
    `${workspaceRoot}/../target/debug/${SERVER_BINARY_NAME}`,
    `${workspaceRoot}/target/debug/${SERVER_BINARY_NAME}`
  );

  // Release build path
  paths.push(
    `${workspaceRoot}/../../target/release/${SERVER_BINARY_NAME}`,
    `${workspaceRoot}/../target/release/${SERVER_BINARY_NAME}`,
    `${workspaceRoot}/target/release/${SERVER_BINARY_NAME}`
  );

  return paths;
}

