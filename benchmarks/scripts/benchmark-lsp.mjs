#!/usr/bin/env node

/**
 * LSP Benchmark Script
 *
 * Compares performance of Rust TypeScript LSP vs official TypeScript language server
 *
 * Usage:
 *   node benchmark-lsp.mjs                  # Run all benchmarks
 *   node benchmark-lsp.mjs --server rust    # Only benchmark Rust server
 *   node benchmark-lsp.mjs --server tsserver # Only benchmark tsserver
 */

import { spawn } from "child_process";
import { createInterface } from "readline";
import { readFileSync, readdirSync, statSync } from "fs";
import { join, resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { performance } from "perf_hooks";

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = resolve(__dirname, "../fixtures");
const projectRoot = resolve(__dirname, "../..");

// Configuration
const WARMUP_ITERATIONS = 3;
const BENCHMARK_ITERATIONS = 10;

// Parse arguments
const args = process.argv.slice(2);
const serverArg = args.find((a) => a.startsWith("--server="))?.split("=")[1];
const serverFlag = args.indexOf("--server");
const selectedServer =
  serverArg || (serverFlag >= 0 ? args[serverFlag + 1] : null);

/**
 * LSP Client for benchmarking
 */
class LspBenchmarkClient {
  constructor(name, command, args = []) {
    this.name = name;
    this.command = command;
    this.args = args;
    this.process = null;
    this.requestId = 0;
    this.pendingRequests = new Map();
    this.buffer = "";
  }

  async start() {
    return new Promise((resolve, reject) => {
      this.process = spawn(this.command, this.args, {
        stdio: ["pipe", "pipe", "pipe"],
      });

      this.process.stdout.on("data", (data) => {
        this.handleData(data.toString());
      });

      this.process.stderr.on("data", (data) => {
        // Ignore stderr for benchmarks
      });

      this.process.on("error", reject);
      this.process.on("exit", (code) => {
        if (code !== 0 && code !== null) {
          console.error(`${this.name} exited with code ${code}`);
        }
      });

      // Give the server time to start
      setTimeout(resolve, 500);
    });
  }

  handleData(data) {
    this.buffer += data;

    while (true) {
      const headerEnd = this.buffer.indexOf("\r\n\r\n");
      if (headerEnd === -1) break;

      const header = this.buffer.slice(0, headerEnd);
      const contentLengthMatch = header.match(/Content-Length: (\d+)/i);
      if (!contentLengthMatch) break;

      const contentLength = parseInt(contentLengthMatch[1], 10);
      const messageStart = headerEnd + 4;
      const messageEnd = messageStart + contentLength;

      if (this.buffer.length < messageEnd) break;

      const message = this.buffer.slice(messageStart, messageEnd);
      this.buffer = this.buffer.slice(messageEnd);

      try {
        const json = JSON.parse(message);
        if (json.id !== undefined && this.pendingRequests.has(json.id)) {
          const { resolve } = this.pendingRequests.get(json.id);
          this.pendingRequests.delete(json.id);
          resolve(json);
        }
      } catch (e) {
        // Ignore parse errors
      }
    }
  }

  send(message) {
    const json = JSON.stringify(message);
    const header = `Content-Length: ${Buffer.byteLength(json)}\r\n\r\n`;
    this.process.stdin.write(header + json);
  }

  request(method, params) {
    return new Promise((resolve, reject) => {
      const id = ++this.requestId;
      this.pendingRequests.set(id, { resolve, reject });

      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request ${method} timed out`));
      }, 30000);

      this.pendingRequests.set(id, {
        resolve: (result) => {
          clearTimeout(timeout);
          resolve(result);
        },
        reject,
      });

      this.send({ jsonrpc: "2.0", id, method, params });
    });
  }

  notify(method, params) {
    this.send({ jsonrpc: "2.0", method, params });
  }

  async initialize(rootUri) {
    const result = await this.request("initialize", {
      processId: process.pid,
      rootUri,
      capabilities: {
        textDocument: {
          hover: { contentFormat: ["markdown", "plaintext"] },
          completion: { completionItem: { snippetSupport: true } },
          definition: { linkSupport: true },
          references: {},
          documentSymbol: { hierarchicalDocumentSymbolSupport: true },
          semanticTokens: {
            requests: { full: true },
            tokenTypes: [
              "namespace",
              "type",
              "class",
              "enum",
              "interface",
              "struct",
              "typeParameter",
              "parameter",
              "variable",
              "property",
              "enumMember",
              "event",
              "function",
              "method",
              "macro",
              "keyword",
              "modifier",
              "comment",
              "string",
              "number",
              "regexp",
              "operator",
            ],
            tokenModifiers: [],
          },
        },
      },
    });

    this.notify("initialized", {});
    return result;
  }

  async openDocument(uri, content, languageId = "typescript") {
    this.notify("textDocument/didOpen", {
      textDocument: { uri, languageId, version: 1, text: content },
    });
    // Give server time to process
    await new Promise((r) => setTimeout(r, 100));
  }

  async closeDocument(uri) {
    this.notify("textDocument/didClose", {
      textDocument: { uri },
    });
  }

  async hover(uri, line, character) {
    return this.request("textDocument/hover", {
      textDocument: { uri },
      position: { line, character },
    });
  }

  async definition(uri, line, character) {
    return this.request("textDocument/definition", {
      textDocument: { uri },
      position: { line, character },
    });
  }

  async references(uri, line, character) {
    return this.request("textDocument/references", {
      textDocument: { uri },
      position: { line, character },
      context: { includeDeclaration: true },
    });
  }

  async documentSymbols(uri) {
    return this.request("textDocument/documentSymbol", {
      textDocument: { uri },
    });
  }

  async semanticTokens(uri) {
    return this.request("textDocument/semanticTokens/full", {
      textDocument: { uri },
    });
  }

  async shutdown() {
    try {
      await this.request("shutdown", null);
      this.notify("exit", null);
    } catch (e) {
      // Ignore shutdown errors
    }
    this.process?.kill();
  }
}

/**
 * Run a benchmark and collect timing statistics
 */
async function runBenchmark(name, fn, iterations = BENCHMARK_ITERATIONS) {
  // Warmup
  for (let i = 0; i < WARMUP_ITERATIONS; i++) {
    await fn();
  }

  // Benchmark
  const times = [];
  for (let i = 0; i < iterations; i++) {
    const start = performance.now();
    await fn();
    times.push(performance.now() - start);
  }

  // Calculate statistics
  times.sort((a, b) => a - b);
  const sum = times.reduce((a, b) => a + b, 0);
  const mean = sum / times.length;
  const median = times[Math.floor(times.length / 2)];
  const min = times[0];
  const max = times[times.length - 1];
  const stdDev = Math.sqrt(
    times.reduce((acc, t) => acc + (t - mean) ** 2, 0) / times.length
  );

  return { name, mean, median, min, max, stdDev, samples: times.length };
}

/**
 * Format benchmark results as a table
 */
function formatResults(results) {
  console.log("\n" + "=".repeat(80));
  console.log(
    `${"Benchmark".padEnd(35)} ${"Mean".padStart(10)} ${"Median".padStart(10)} ${"Min".padStart(10)} ${"Max".padStart(10)}`
  );
  console.log("-".repeat(80));

  for (const r of results) {
    console.log(
      `${r.name.padEnd(35)} ${r.mean.toFixed(2).padStart(8)}ms ${r.median.toFixed(2).padStart(8)}ms ${r.min.toFixed(2).padStart(8)}ms ${r.max.toFixed(2).padStart(8)}ms`
    );
  }
  console.log("=".repeat(80) + "\n");
}

/**
 * Get all fixture files
 */
function getFixtures() {
  const fixtures = [];
  for (const file of readdirSync(fixturesDir)) {
    if (file.endsWith(".ts") || file.endsWith(".tsx")) {
      const path = join(fixturesDir, file);
      const content = readFileSync(path, "utf-8");
      const stats = statSync(path);
      fixtures.push({
        name: file,
        path,
        uri: `file://${path}`,
        content,
        size: stats.size,
        lines: content.split("\n").length,
      });
    }
  }
  return fixtures.sort((a, b) => a.size - b.size);
}

/**
 * Run benchmarks for a single server
 */
async function benchmarkServer(client, fixtures) {
  const results = [];

  // Initialize
  const initResult = await runBenchmark(
    `[${client.name}] Initialize`,
    async () => {
      // Re-initialize is not standard, so we measure initial response
    },
    1
  );

  for (const fixture of fixtures) {
    const label = `${fixture.name} (${fixture.lines} lines)`;

    // Document open
    results.push(
      await runBenchmark(`[${client.name}] Open: ${label}`, async () => {
        await client.openDocument(fixture.uri, fixture.content);
        await client.closeDocument(fixture.uri);
      })
    );

    // Open document for remaining tests
    await client.openDocument(fixture.uri, fixture.content);

    // Document symbols
    results.push(
      await runBenchmark(`[${client.name}] Symbols: ${label}`, async () => {
        await client.documentSymbols(fixture.uri);
      })
    );

    // Semantic tokens
    results.push(
      await runBenchmark(`[${client.name}] Tokens: ${label}`, async () => {
        await client.semanticTokens(fixture.uri);
      })
    );

    // Hover (middle of file)
    const hoverLine = Math.floor(fixture.lines / 2);
    results.push(
      await runBenchmark(`[${client.name}] Hover: ${label}`, async () => {
        await client.hover(fixture.uri, hoverLine, 10);
      })
    );

    await client.closeDocument(fixture.uri);
  }

  return results;
}

/**
 * Main benchmark runner
 */
async function main() {
  console.log("LSP Benchmark Suite");
  console.log("==================\n");

  const fixtures = getFixtures();
  if (fixtures.length === 0) {
    console.error("No fixture files found in", fixturesDir);
    process.exit(1);
  }

  console.log("Fixtures:");
  for (const f of fixtures) {
    console.log(`  - ${f.name}: ${f.lines} lines, ${f.size} bytes`);
  }
  console.log();

  const allResults = [];

  // Benchmark Rust server
  if (!selectedServer || selectedServer === "rust") {
    console.log("Starting Rust TypeScript Language Server...");
    const rustServerPath = join(
      projectRoot,
      "target/release/typescript-language-server"
    );

    try {
      const rustClient = new LspBenchmarkClient("Rust LSP", rustServerPath);
      await rustClient.start();
      await rustClient.initialize(`file://${projectRoot}`);

      const rustResults = await benchmarkServer(rustClient, fixtures);
      allResults.push(...rustResults);

      await rustClient.shutdown();
    } catch (e) {
      console.error("Failed to benchmark Rust server:", e.message);
      console.log("Make sure to build with: cargo build --release");
    }
  }

  // Benchmark tsserver
  if (!selectedServer || selectedServer === "tsserver") {
    console.log("\nStarting TypeScript Language Server (tsserver)...");

    try {
      const tsClient = new LspBenchmarkClient("tsserver", "npx", [
        "typescript-language-server",
        "--stdio",
      ]);
      await tsClient.start();
      await tsClient.initialize(`file://${projectRoot}`);

      const tsResults = await benchmarkServer(tsClient, fixtures);
      allResults.push(...tsResults);

      await tsClient.shutdown();
    } catch (e) {
      console.error("Failed to benchmark tsserver:", e.message);
      console.log("Make sure typescript-language-server is installed");
    }
  }

  // Print results
  if (allResults.length > 0) {
    formatResults(allResults);

    // Print comparison if both servers were benchmarked
    if (!selectedServer) {
      console.log("\nComparison Summary:");
      console.log("-".repeat(60));

      const rustResults = allResults.filter((r) =>
        r.name.startsWith("[Rust LSP]")
      );
      const tsResults = allResults.filter((r) =>
        r.name.startsWith("[tsserver]")
      );

      for (const rustResult of rustResults) {
        const benchName = rustResult.name.replace("[Rust LSP] ", "");
        const tsResult = tsResults.find(
          (r) => r.name.replace("[tsserver] ", "") === benchName
        );

        if (tsResult) {
          const speedup = tsResult.mean / rustResult.mean;
          const indicator = speedup > 1 ? "🚀" : "🐢";
          console.log(
            `${benchName.padEnd(40)} ${indicator} ${speedup.toFixed(2)}x ${speedup > 1 ? "faster" : "slower"}`
          );
        }
      }
    }
  }
}

main().catch(console.error);

