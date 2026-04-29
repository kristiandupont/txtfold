#!/usr/bin/env bun
// Extracts doc sections from source files and writes them to generated TypeScript
// modules for use in the web UI.

import { readFileSync, writeFileSync, mkdirSync } from "fs";

function extractBetween(source: string, start: string, end: string, label: string): string {
  const startIdx = source.indexOf(start);
  const endIdx = source.indexOf(end);
  if (startIdx === -1 || endIdx === -1) {
    throw new Error(`Markers not found in ${label}`);
  }
  return source.slice(startIdx + start.length, endIdx).trim();
}

mkdirSync("web/src/generated", { recursive: true });

// consumer-docs: extracted from README.md
const readme = readFileSync("README.md", "utf-8");
const consumerDocs = extractBetween(readme, "<!-- docs:consumer-start -->", "<!-- docs:consumer-end -->", "README.md");
writeFileSync(
  "web/src/generated/consumer-docs.ts",
  `// Auto-generated from README.md — do not edit directly\nexport const consumerDocs = ${JSON.stringify(consumerDocs)};\n`
);
console.log("Generated web/src/generated/consumer-docs.ts");

// llmsContent: full contents of llms.txt
const llmsContent = readFileSync("llms.txt", "utf-8").trim();
writeFileSync(
  "web/src/generated/llms-content.ts",
  `// Auto-generated from llms.txt — do not edit directly\nexport const llmsContent = ${JSON.stringify(llmsContent)};\n`
);
console.log("Generated web/src/generated/llms-content.ts");
