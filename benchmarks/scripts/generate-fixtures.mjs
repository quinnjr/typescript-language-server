#!/usr/bin/env node

/**
 * Generate TypeScript fixture files of various sizes for benchmarking
 */

import { writeFileSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, "../fixtures");

// Ensure fixtures directory exists
mkdirSync(fixturesDir, { recursive: true });

function generateInterface(name, fields) {
  return `interface ${name} {
${fields.map((f) => `  ${f.name}: ${f.type};`).join("\n")}
}`;
}

function generateClass(name, methods) {
  return `class ${name} {
  private data: Map<string, unknown> = new Map();

${methods
  .map(
    (m) => `  ${m.async ? "async " : ""}${m.name}(${m.params}): ${m.returnType} {
    ${m.body}
  }`
  )
  .join("\n\n")}
}`;
}

function generateFunction(name, params, returnType, body, isAsync = false) {
  return `${isAsync ? "async " : ""}function ${name}(${params}): ${returnType} {
  ${body}
}`;
}

function generateLargeFile(numInterfaces, numClasses, numFunctions) {
  const lines = [];

  // Imports
  lines.push(`// Auto-generated large TypeScript file for benchmarking`);
  lines.push(`// ${numInterfaces} interfaces, ${numClasses} classes, ${numFunctions} functions`);
  lines.push(``);
  lines.push(`import type { ReactNode } from 'react';`);
  lines.push(``);

  // Generate interfaces
  for (let i = 0; i < numInterfaces; i++) {
    lines.push(
      generateInterface(`Entity${i}`, [
        { name: "id", type: "string" },
        { name: "name", type: "string" },
        { name: `field${i}`, type: i % 2 === 0 ? "number" : "boolean" },
        { name: "metadata", type: `Record<string, unknown>` },
        { name: "createdAt", type: "Date" },
      ])
    );
    lines.push(``);
  }

  // Generate classes
  for (let i = 0; i < numClasses; i++) {
    lines.push(
      generateClass(`Service${i}`, [
        {
          name: "constructor",
          params: "private readonly config: Record<string, unknown>",
          returnType: "",
          body: "// Initialize",
          async: false,
        },
        {
          name: `process${i}`,
          params: `input: Entity${i % numInterfaces}`,
          returnType: `Promise<Entity${i % numInterfaces}>`,
          body: `const result = { ...input, field${i % numInterfaces}: ${i} };
    this.data.set(input.id, result);
    return result;`,
          async: true,
        },
        {
          name: `validate${i}`,
          params: `data: unknown`,
          returnType: `data is Entity${i % numInterfaces}`,
          body: `return typeof data === 'object' && data !== null && 'id' in data;`,
          async: false,
        },
        {
          name: "clear",
          params: "",
          returnType: "void",
          body: "this.data.clear();",
          async: false,
        },
      ])
    );
    lines.push(``);
  }

  // Generate functions
  for (let i = 0; i < numFunctions; i++) {
    const isAsync = i % 3 === 0;
    lines.push(
      generateFunction(
        `helper${i}`,
        `param: Entity${i % numInterfaces}`,
        isAsync ? `Promise<string>` : "string",
        `const value = param.name + '_${i}';
  ${isAsync ? "await new Promise(r => setTimeout(r, 0));" : ""}
  return value;`,
        isAsync
      )
    );
    lines.push(``);
  }

  // Generate some complex types
  lines.push(`// Complex types`);
  lines.push(`type AllEntities = ${Array.from({ length: Math.min(numInterfaces, 10) }, (_, i) => `Entity${i}`).join(" | ")};`);
  lines.push(``);
  lines.push(`type EntityMap = {`);
  for (let i = 0; i < Math.min(numInterfaces, 10); i++) {
    lines.push(`  entity${i}: Entity${i};`);
  }
  lines.push(`};`);
  lines.push(``);

  // Export
  lines.push(`export {`);
  for (let i = 0; i < numInterfaces; i++) {
    lines.push(`  Entity${i},`);
  }
  for (let i = 0; i < numClasses; i++) {
    lines.push(`  Service${i},`);
  }
  for (let i = 0; i < numFunctions; i++) {
    lines.push(`  helper${i},`);
  }
  lines.push(`  AllEntities,`);
  lines.push(`  EntityMap,`);
  lines.push(`};`);

  return lines.join("\n");
}

// Generate files of different sizes
const configs = [
  { name: "large", interfaces: 20, classes: 10, functions: 30 },
  { name: "xlarge", interfaces: 50, classes: 25, functions: 75 },
  { name: "huge", interfaces: 100, classes: 50, functions: 150 },
];

for (const config of configs) {
  const content = generateLargeFile(
    config.interfaces,
    config.classes,
    config.functions
  );
  const path = join(fixturesDir, `${config.name}.ts`);
  writeFileSync(path, content);
  console.log(
    `Generated ${config.name}.ts: ${content.split("\n").length} lines, ${content.length} bytes`
  );
}

console.log("\nFixture generation complete!");

