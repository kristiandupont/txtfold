#!/usr/bin/env bun
/**
 * Generates typed output from output-schema.json:
 *   bindings/npm/src/types.ts   — TypeScript interfaces
 *   bindings/python/txtfold/_types.py  — Python TypedDicts
 *
 * Run from the repo root:
 *   bun tools/gen-types.ts
 */

import { readFileSync, writeFileSync } from "fs";
import { join } from "path";

const root = join(import.meta.dir, "..");
const schema = JSON.parse(readFileSync(join(root, "output-schema.json"), "utf8"));
const defs = schema.definitions as Record<string, any>;

// --- Shared helpers ---------------------------------------------------------

function refName(ref: string): string {
  return ref.replace("#/definitions/", "");
}

/** Convert snake_case to PascalCase. */
function toPascal(s: string): string {
  return s.split("_").map((w) => w[0].toUpperCase() + w.slice(1)).join("");
}

/** Last word of a PascalCase name, e.g. "AlgorithmResults" → "Results". */
function lastWord(name: string): string {
  const m = name.match(/[A-Z][a-z]*/g);
  return m ? m[m.length - 1] : name;
}

/** Resolve optional/nullable wrappers and return {inner, nullable}. */
function unwrap(node: any): { inner: any; nullable: boolean } {
  // anyOf: [{...}, {type: "null"}]
  if (node.anyOf) {
    const nonNull = node.anyOf.filter((n: any) => n.type !== "null");
    return { inner: nonNull.length === 1 ? nonNull[0] : { anyOf: nonNull }, nullable: nonNull.length < node.anyOf.length };
  }
  // "type": ["T", "null"]
  if (Array.isArray(node.type)) {
    const nonNull = node.type.filter((t: string) => t !== "null");
    return { inner: { ...node, type: nonNull.length === 1 ? nonNull[0] : nonNull }, nullable: node.type.includes("null") };
  }
  return { inner: node, nullable: false };
}

// --- TypeScript -------------------------------------------------------------

function tsType(node: any): string {
  if (!node) return "unknown";
  if (node.$ref) return refName(node.$ref);
  if (node.allOf?.length === 1) return tsType(node.allOf[0]);

  const { inner, nullable } = unwrap(node);
  if (inner !== node) {
    const base = tsType(inner);
    return nullable ? `${base} | null` : base;
  }

  switch (node.type) {
    case "string":  return node.enum?.length === 1 ? `"${node.enum[0]}"` : "string";
    case "boolean": return "boolean";
    case "number":  return "number";
    case "integer": return "number";
    case "array":
      return Array.isArray(node.items)
        ? `[${node.items.map(tsType).join(", ")}]`
        : `${tsType(node.items)}[]`;
    case "object":
      return node.additionalProperties
        ? `Record<string, ${tsType(node.additionalProperties)}>`
        : "Record<string, unknown>";
  }
  return "unknown";
}

function generateTS(): string {
  const out: string[] = [
    "// THIS FILE IS GENERATED — do not edit by hand.",
    "// Source: output-schema.json",
    "// Regenerate: bun tools/gen-types.ts",
    "",
  ];

  const doc = (text?: string) => { if (text) out.push(`/** ${text} */`); };

  function emitObject(name: string, def: any) {
    doc(def.description);
    out.push(`export interface ${name} {`);
    for (const [k, v] of Object.entries((def.properties ?? {}) as Record<string, any>)) {
      const opt = !def.required?.includes(k) ? "?" : "";
      out.push(`  ${k}${opt}: ${tsType(v)};`);
    }
    out.push("}");
    out.push("");
  }

  // Root type
  if (schema.title && schema.type === "object") emitObject(schema.title, schema);

  for (const [name, def] of Object.entries(defs)) {
    if (def.oneOf) {
      const suffix = lastWord(name);
      const variantNames: string[] = [];
      for (const variant of def.oneOf) {
        const tag: string = variant.properties?.type?.enum?.[0];
        const variantName = toPascal(tag) + suffix;
        variantNames.push(variantName);
        doc(variant.description);
        out.push(`export interface ${variantName} {`);
        for (const [k, v] of Object.entries(variant.properties as Record<string, any>)) {
          const opt = !variant.required?.includes(k) ? "?" : "";
          out.push(`  ${k}${opt}: ${tsType(v)};`);
        }
        out.push("}");
        out.push("");
      }
      doc(def.description);
      out.push(`export type ${name} =`);
      variantNames.forEach((v, i) => out.push(`  | ${v}${i === variantNames.length - 1 ? ";" : ""}`));
      out.push("");
    } else {
      emitObject(name, def);
    }
  }

  // ProcessOptions is binding-specific — not derived from the output schema.
  out.push(
    "/** Options for process() and processMarkdown(). */",
    "export interface ProcessOptions {",
    '  /** Algorithm to use. Default: "auto" (auto-detect). */',
    "  algorithm?: string;",
    "  /** Similarity threshold for clustering/schema algorithms (0.0–1.0). Default: 0.8. */",
    "  threshold?: number;",
    "  /** N-gram size for the ngram algorithm. Default: 2. */",
    "  ngramSize?: number;",
    "  /** Outlier threshold for ngram (0.0 = auto-detect). Default: 0.0. */",
    "  outlierThreshold?: number;",
    "  /** Maximum output lines. Most important groups shown first; output trimmed at limit. */",
    "  budgetLines?: number;",
    "}",
    "",
  );

  return out.join("\n");
}

// --- Python -----------------------------------------------------------------

function pyType(node: any): string {
  if (!node) return "Any";
  if (node.$ref) return refName(node.$ref);
  if (node.allOf?.length === 1) return pyType(node.allOf[0]);

  const { inner, nullable } = unwrap(node);
  if (inner !== node) {
    const base = pyType(inner);
    return nullable ? `${base} | None` : base;
  }

  switch (node.type) {
    case "string":  return node.enum?.length === 1 ? `Literal["${node.enum[0]}"]` : "str";
    case "boolean": return "bool";
    case "number":  return "float";
    case "integer": return "int";
    case "array":
      return Array.isArray(node.items)
        ? `tuple[${node.items.map(pyType).join(", ")}]`
        : `list[${pyType(node.items)}]`;
    case "object":
      return node.additionalProperties
        ? `dict[str, ${pyType(node.additionalProperties)}]`
        : "dict[str, Any]";
  }
  return "Any";
}

function generatePy(): string {
  const out: string[] = [
    "# THIS FILE IS GENERATED — do not edit by hand.",
    "# Source: output-schema.json",
    "# Regenerate: bun tools/gen-types.ts",
    "",
    "from __future__ import annotations",
    "",
    "from typing import Any, Literal, Union",
    "from typing import TypedDict",
    "",
    "",
  ];

  function emitObject(name: string, def: any) {
    const props = Object.entries((def.properties ?? {}) as Record<string, any>);
    const required = new Set<string>(def.required ?? []);
    const reqProps = props.filter(([k]) => required.has(k));
    const optProps = props.filter(([k]) => !required.has(k));

    if (def.description) out.push(`# ${def.description}`);

    if (optProps.length > 0 && reqProps.length > 0) {
      // Split into required base + optional subclass (Python 3.8+ pattern)
      out.push(`class _${name}Required(TypedDict):`);
      for (const [k, v] of reqProps) out.push(`    ${k}: ${pyType(v)}`);
      out.push("");
      out.push(`class ${name}(_${name}Required, total=False):`);
      for (const [k, v] of optProps) out.push(`    ${k}: ${pyType(v)}`);
    } else {
      const total = optProps.length > 0 ? ", total=False" : "";
      out.push(`class ${name}(TypedDict${total}):`);
      if (props.length === 0) {
        out.push("    pass");
      } else {
        for (const [k, v] of props) out.push(`    ${k}: ${pyType(v)}`);
      }
    }
    out.push("");
    out.push("");
  }

  // Root type
  if (schema.title && schema.type === "object") emitObject(schema.title, schema);

  for (const [name, def] of Object.entries(defs)) {
    if (def.oneOf) {
      const suffix = lastWord(name);
      const variantNames: string[] = [];
      for (const variant of def.oneOf) {
        const tag: string = variant.properties?.type?.enum?.[0];
        const variantName = toPascal(tag) + suffix;
        variantNames.push(variantName);
        if (variant.description) out.push(`# ${variant.description}`);
        out.push(`class ${variantName}(TypedDict):`);
        for (const [k, v] of Object.entries(variant.properties as Record<string, any>)) {
          out.push(`    ${k}: ${pyType(v)}`);
        }
        out.push("");
        out.push("");
      }
      if (def.description) out.push(`# ${def.description}`);
      out.push(`${name} = Union[${variantNames.join(", ")}]`);
      out.push("");
      out.push("");
    } else {
      emitObject(name, def);
    }
  }

  return out.join("\n");
}

// --- Write outputs ----------------------------------------------------------

const tsPath = join(root, "bindings/npm/src/types.ts");
writeFileSync(tsPath, generateTS());
console.log(`wrote ${tsPath}`);

const pyPath = join(root, "bindings/python/txtfold/_types.py");
writeFileSync(pyPath, generatePy());
console.log(`wrote ${pyPath}`);
