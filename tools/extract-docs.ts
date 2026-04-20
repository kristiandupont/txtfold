#!/usr/bin/env bun
// Extracts the consumer-facing section from README.md and writes it to
// web/src/generated/consumer-docs.ts for use in the web UI.

import { readFileSync, writeFileSync, mkdirSync } from "fs";

const readme = readFileSync("README.md", "utf-8");

const START = "<!-- docs:consumer-start -->";
const END = "<!-- docs:consumer-end -->";

const startIdx = readme.indexOf(START);
const endIdx = readme.indexOf(END);

if (startIdx === -1 || endIdx === -1) {
  throw new Error("Consumer doc markers not found in README.md");
}

const content = readme.slice(startIdx + START.length, endIdx).trim();

mkdirSync("web/src/generated", { recursive: true });
writeFileSync(
  "web/src/generated/consumer-docs.ts",
  `// Auto-generated from README.md — do not edit directly\nexport const consumerDocs = ${JSON.stringify(content)};\n`
);

console.log("Generated web/src/generated/consumer-docs.ts");
