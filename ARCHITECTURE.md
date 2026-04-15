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

Metadata is colocated with each component's implementation. The registry aggregates it for discovery. Internal consumers (CLI, WASM) link against core directly; external consumers use the generated artifacts: `schema.json` (build-time artifact, consumed by the web UI) and `output-schema.json` (checked in, used by language bindings and docs).

```
core/src/*.rs  (metadata colocated with implementation)
    ↓
registry.rs    (ALL_ALGORITHMS, ALL_FORMATTERS, ALL_INPUT_FORMATS)
    ├─→ CLI          (clap builder reads registry at startup)
    ├─→ WASM         (reads registry at startup)
    └─→ dump-schema  (dev tool)
                     ├─→ schema.json        (build artifact → Web UI)
                     └─→ output-schema.json (checked in)
                                                 ↓
                                          gen-types.ts  (bun tool)
                                             ├─→ bindings/npm/src/types.ts
                                             └─→ bindings/python/txtfold/_types.py
```

`dump-schema` is a pure dev tool with no user-facing surface. It avoids a circular dependency: schema determines CLI args, so the CLI can't generate the schema itself.

## Processing Pipeline

```
Input (file or stdin)
  ↓
Input Format (explicit: json | line | block)
  — inferred from file extension for files (.json → json, else → line)
  — required via --format flag for stdin
  ↓
  ├─ --discover (bypasses analysis entirely)
  │    ↓
  │   discover.rs → DiscoverOutput (paths, types, cardinality, samples)
  │    ↓
  │   Formatter → Markdown table / JSON
  │
  ├─ --cost-preview (runs full analysis, then cost pass)
  │    ↓
  │   (normal analysis pipeline — see below)
  │    ↓
  │   cost_preview.rs → CostPreviewOutput (per-field token estimates, suggestion)
  │    ↓
  │   Formatter → Markdown table / JSON
  │
  └─ (normal analysis)
       ├─ line  → Parser → Text Entries (one per line)
       │            ↓
       │         Algorithm (auto) → template | clustering | ngram
       │
       ├─ block → Parser → Text Entries (multi-line; --entry-pattern or timestamp heuristic)
       │            ↓
       │         Algorithm (auto) → template | clustering | ngram
       │
       └─ json  → Parser → JSON Values (array or map — internal heuristic)
                    ↓
                 Algorithm (auto) → schema | subtree
       ↓
      Structured Output (JSON) — algorithm-specific result type
       ↓
      Formatter → Markdown / JSON output
```

### Configuration Hierarchy

Two levels of selection, each overridable:

1. **Algorithm** (`auto` → template for line/block, schema for json)
2. **Parameters** (threshold, ngram size, entry pattern, etc.)

Input format is always explicit — either declared via `--format` or inferred from the file extension. There is no content-based auto-detection.

## Discover

`discover.rs` implements a fast structural scan that runs on the full document before any analysis. It produces a `DiscoverOutput`: a list of `FieldSummary` entries, one per unique field path (JSON) or token slot position (line/block).

For **JSON**, it walks the entire document tree. Array indices are normalized to `[*]` so that all elements of an array share a single representative path (e.g. `$.diagnostics[*].category`). For each leaf path it records: value types seen, cardinality (distinct values, capped at 10 000), up to 5 samples, and `present_in_pct` (occurrences of this path ÷ total elements in the nearest enclosing array).

For **line/block**, it tokenizes the first line of each entry using the existing `Tokenizer` and treats each non-whitespace token position as a slot. Reports the token type (timestamp, number, ip\_address, identifier, literal, …), cardinality, and samples per slot.

`DiscoverOutput` is part of `output-schema.json` alongside `AnalysisOutput` and `CostPreviewOutput`.

## Cost Preview

`cost_preview.rs` runs the full analysis pipeline and then walks the resulting `AnalysisOutput` to compute a field-level token breakdown. Token count is estimated as `chars / 4` (ceiling division), which is a good approximation for English-language and code content.

For each result variant the relevant sample data is:

- **SchemaGrouped / PathGrouped** — `sample_values` maps from field name to sampled strings; costs are aggregated by field name across all groups.
- **Grouped** (text templates / clustering) — pattern text goes to a `pattern` bucket; sample entry content goes to `content`; per-variable values (e.g. `var_0`) are counted separately.
- **OutlierFocused** (n-gram) — baseline `common_features` strings and outlier content are counted as separate buckets.

After aggregation, fields are sorted by token count. Any field consuming >20% of the total is flagged as a noise candidate and included in a `del(...)` suggestion string showing the estimated remaining tokens.

`CostPreviewOutput` is part of `output-schema.json` alongside `AnalysisOutput` and `DiscoverOutput`.

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

`schema.json` documents the *input* side: algorithms, parameters, formatters, and their metadata. It is generated by `dump-schema` at build time and is **not** checked in — the web UI depends on it as a build artifact (`web/schema.json`). `output-schema.json` documents the *output* side: a JSON Schema derived from `AnalysisOutput` via `schemars`, covering all four result variants (`grouped`, `outlier_focused`, `schema_grouped`, `path_grouped`) and their nested types. It **is** checked in and serves as the stable contract for language bindings' typed deserialization.

## Language Bindings

Both bindings expose a six-function API: `process()` → `AnalysisOutput`, `processMarkdown()` → string, `discover()` → `DiscoverOutput`, `discoverMarkdown()` → string, `costPreview()` → `CostPreviewOutput`, `costPreviewMarkdown()` → string. All three output types and their supporting sub-types are generated from `output-schema.json` by `tools/gen-types.ts`. The schema is a combined multi-root document (`roots: [AnalysisOutput, DiscoverOutput, CostPreviewOutput]`) with a shared `definitions` block; `dump-schema` regenerates it from the Rust types via `schemars`.

**Python (`bindings/python/`)** — PyO3 native extension built with maturin. `process()` returns a `dict` typed as `AnalysisOutput` (a `TypedDict`). Published as `txtfold` on PyPI. Per-platform wheels built by a CI matrix.

**npm (`bindings/npm/`)** — WASM core compiled with `wasm-pack --target nodejs`, wrapped in a TypeScript module. `process()` returns a typed `AnalysisOutput` object. Published as `txtfold` on npm. Single platform-neutral package (WASM is portable). Build: `bun run build` (runs wasm-pack → gen-types → tsc).

## Workspace Structure

```
core/                    Core library (algorithms, parsers, formatters, registry)
cli/                     Command-line interface (built from registry at startup)
tools/dump-schema/       Dev tool: serializes registry → schema.json + output-schema.json
tools/sample-generator/  Dev tool: generates synthetic logs and JSON for testing
tools/gen-types.ts       Dev tool: generates typed output wrappers from output-schema.json
web/                     Web UI (Crank + TypeScript + Tailwind, WASM backend)
bindings/python/         Python binding (PyO3 + maturin, published to PyPI)
bindings/npm/            npm binding (WASM + TypeScript, published to npm)
schema.json              Generated registry snapshot (build artifact, not checked in)
output-schema.json       Checked-in JSON Schema for AnalysisOutput (for language bindings)
```

- **Language**: Rust (performance + determinism + WASM support)
- **License**: MIT OR Apache-2.0
