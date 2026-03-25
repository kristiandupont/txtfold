# txtfold: Architecture

Identifies patterns and outliers in large log files and structured data. Converts large files into human/LLM-friendly summaries. No ML, no fuzzy logic — same input always produces same output.

## Core Principles

- **Deterministic**: same input always produces same output
- **Composable**: multiple algorithms, each suited to different data patterns
- **Transparent**: output explains decisions (auto-detected thresholds, score ranges)
- **Fast enough**: can run multiple algorithms and still be practical

## Three-Layer Design

```
1. Core Engine (Rust) → Structured Data (JSON)
2. Formatters         → Human/LLM output (Markdown, HTML, etc.)
3. Consumers          → CLI, WASM, Python/JS bindings
```

## Source of Truth Hierarchy

Metadata is colocated with each component's implementation. The registry aggregates it for discovery. Internal consumers (CLI, WASM) link against core directly; external consumers (TS/Python bindings, web UI, docs) depend on `schema.json`, which is generated from the registry and checked in.

```
core/src/*.rs  (metadata colocated with implementation)
    ↓
registry.rs    (ALL_ALGORITHMS, ALL_FORMATTERS, ALL_INPUT_FORMATS)
    ├─→ CLI          (clap builder reads registry at startup)
    ├─→ WASM         (reads registry at startup)
    └─→ dump-schema  (dev tool → schema.json)
                          ↓
               ┌──────────┴──────────┐
               ↓                     ↓
         TS/Python bindings    Docs / Web UI
```

`dump-schema` is a pure dev tool with no user-facing surface. It avoids a circular dependency: schema determines CLI args, so the CLI can't generate the schema itself.

## Processing Pipeline

```
Input (file or stdin)
  ↓
Input Format Detection (auto) → Text | JSON Array | JSON Map
  ↓
  ├─ Text → Entry Mode Detection (auto) → Single-line | Multi-line
  │            ↓
  │         Parser → Text Entries
  │            ↓
  │         Algorithm (auto) → template | clustering | ngram
  │
  └─ JSON → Parser → JSON Values
               ↓
            Algorithm (auto) → schema
  ↓
Structured Output (JSON) — algorithm-specific result type
  ↓
Formatter → Markdown / JSON output
```

### Configuration Hierarchy

Three levels of auto-detection, each overridable:

1. **Input Format** (`auto` → text / json-array / json-map)
2. **Algorithm** (`auto` → template / clustering / ngram / schema, based on format)
3. **Parameters** (threshold, entry mode, etc.)

Beginners get "just works" behavior; power users can override at any level.

## Algorithms

**Template Extraction** — tokenizes entries, extracts patterns with variable slots (`[<TIMESTAMP>] INFO User <ID> logged in`). Best for structured logs with clear token patterns. Typical reduction: 30–40%.

**Edit Distance Clustering** — groups similar entries using Levenshtein distance. Best for entries differing only in IDs/numbers/service names. Configurable threshold (default 0.8). Typical reduction: 70–80%.

**N-gram Outlier Detection** (word-based) — identifies rare word combinations. Best for finding unusual entries in uniform logs. Auto-threshold flags bottom ~5% by default; reports score distribution and threshold used. Typical reduction: 2–5% (highlights outliers only).

**JSON Schema Clustering** — groups JSON objects by structural similarity (field names + types). Configurable threshold (default 0.8 = 80% field match). Singletons flagged as outliers; sample values shown per field.

### Why Word-Based N-grams?

Character-based n-grams found fragments like `'ler'`, `'er.'` — useless for logs. Word-based n-grams find meaningful patterns like `'process_data payload'`, `'NullPointerException at'`.

### Why Auto-Threshold?

Threshold values (0.001 vs 0.02) are arbitrary and data-dependent. Auto-detection (bottom 5%) is intuitive and works across datasets. Manual override still available for power users.

### Why Algorithm-Specific Output Types?

Template/clustering produce groups+outliers. N-gram produces baseline+outliers. Forcing both into a single structure felt unnatural. The enum-based approach (`Grouped`, `OutlierFocused`, `SchemaGrouped`) lets each algorithm express results optimally.

## Entry Handling

- Multi-line support for stack traces and structured logs
- Timestamp-based boundary detection, fallback to single-line mode
- Preserves line numbers and entry metadata

## Metadata System

Each algorithm, formatter, and input format declares a `const` metadata struct alongside its implementation. The registry aggregates these for discovery. Metadata covers: name, aliases, description, best_for, parameters (type, default, range, special values), accepted input types, MIME type.

This means adding a new algorithm automatically propagates to CLI help text, valid-value validation, `schema.json`, and any generated bindings — no manual sync.

### Why Colocated Metadata?

Considered a central config file vs. each component declaring its own. Chose colocated because:
- No sync overhead: metadata lives with the implementation
- Variable needs: Template has 0 params, N-gram has 2, Schema has 1 with special values
- Type-safe at compile time via `const`
- Registry provides discovery without owning the data

Trade-off: registry needs updating when adding components, but this is intentional.

## Output Schema

`schema.json` currently documents the *input* side: algorithms, parameters, formatters, and their metadata. The *output* side — the JSON structure each algorithm produces — should follow the same pattern: derived from Rust types (via `schemars`) and attached to each algorithm's entry in `schema.json`. This gives language bindings a machine-readable contract for typed deserialization.

## Workspace Structure

```
core/                    Core library (algorithms, parsers, formatters, registry)
cli/                     Command-line interface (built from registry at startup)
tools/dump-schema/       Dev tool: serializes registry → schema.json
tools/sample-generator/  Dev tool: generates synthetic logs and JSON for testing
web/                     Web UI (Crank + TypeScript + Tailwind, WASM backend)
schema.json              Checked-in registry snapshot for external consumers
```

- **Language**: Rust (performance + determinism + WASM support)
- **License**: MIT OR Apache-2.0
