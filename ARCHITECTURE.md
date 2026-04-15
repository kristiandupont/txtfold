# txtfold: Architecture

Identifies patterns and outliers in large log files and structured data. Converts large files into human/LLM-friendly summaries. No ML, no fuzzy logic — same input always produces same output.

## Core Principles

- **Deterministic**: same input always produces same output
- **Composable**: pipeline expressions select algorithms and pre-process data
- **Transparent**: output explains decisions (algorithm used, reduction ratio, thresholds)
- **Fast enough**: can run multiple algorithms and still be practical

## Three-Layer Design

```
1. Core Engine (Rust) → Structured Data (JSON)
2. Formatters         → Human/LLM output (Markdown, HTML, etc.)
3. Consumers          → CLI, WASM, Python/JS bindings
```

## Source of Truth Hierarchy

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

`dump-schema` is a pure dev tool. It avoids a circular dependency: schema determines CLI args, so the CLI can't generate the schema itself.

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
  │
  ├─ --cost-preview (runs full pipeline, then cost pass)
  │    ↓
  │   (normal pipeline below)
  │    ↓
  │   cost_preview.rs → CostPreviewOutput (per-field token estimates, suggestion)
  │
  └─ (normal analysis)
       ↓
      Pipeline Expression (optional positional arg)
        ├─ Pre-processing stages: PathSelect, Del
        ├─ Algorithm verb (terminal): summarize | similar(t) | patterns |
        │                             outliers | schemas | subtree | group_by(.f)
        └─ Post-processing: top(N), label(.f)
       ↓
      Algorithm runs on transformed input
       ↓
      Post-processing applied (top, label)
       ↓
      Structured Output (JSON) — algorithm-specific result type
       ↓
      Formatter → Markdown / JSON output
```

### Algorithm Selection

The terminal verb in the pipeline expression selects the algorithm. `--algorithm` does not exist; the pipeline is the sole mechanism.

| Terminal verb | Algorithm | Notes |
|---|---|---|
| `summarize` (default) | fixed per format: json→subtree, line/block→template | no content sniffing |
| `similar(t)` | edit-distance clustering | threshold `t` ∈ [0.0, 1.0] |
| `patterns` | template extraction | |
| `outliers` | n-gram outlier detection | |
| `schemas` | schema clustering | JSON only |
| `subtree` | subtree algorithm | JSON only |
| `group_by(.f)` | value-based frequency table | |

If no pipeline is given, `summarize` is the implicit default.

### Configuration

Two levels of selection:

1. **Algorithm** — terminal verb (default: `summarize`)
2. **Parameters** — `--ngram-size`, `--outlier-threshold`, `--depth`, `--entry-pattern`

`similar(t)` is the only algorithm that takes a parameter inline (the threshold).

## Pipeline Expressions

`pipeline.rs` implements a hand-rolled recursive descent parser and executor.

**Grammar**:
```
pipeline      = stage ("|" stage)*
stage         = path_expr | verb
path_expr     = "." ident ( "[" ("*" | integer | "") "]" )* ("." ident)*
verb          = del_verb | group_by_verb | label_verb | top_verb | algorithm_verb
del_verb      = "del" "(" field_list ")"
group_by_verb = "group_by" "(" field_expr ")"
label_verb    = "label" "(" field_expr ")"
top_verb      = "top" "(" integer ")"
algorithm_verb = "summarize" | "similar" "(" float ")" | "patterns"
              | "outliers" | "schemas" | "subtree"
```

**Stage taxonomy**:
- **Pre-processing** (`PathSelect`, `Del`) — transform input before the algorithm sees it. JSON-only.
- **Algorithm selection** (`AlgorithmVerb`, `GroupBy`) — the terminal verb drives algorithm selection.
- **Post-processing** (`Top`, `Label`) — applied to `AnalysisOutput` after the algorithm runs.

**jaq boundary (future)**: Pre-processing stages that return `Value` are the natural domain of jaq. The `Stage` enum reserves a `Jaq` variant so the boundary is explicit in the type system. When integrated: jaq handles pre-processing stages; txtfold takes over at the first algorithm verb.

## Discover

`discover.rs` implements a fast structural scan on the full document before any analysis. Produces `DiscoverOutput`: one `FieldSummary` per unique field path (JSON) or token slot position (line/block). For JSON, array indices are normalized to `[*]`. Records value types, cardinality (capped at 10,000), up to 5 samples, and `present_in_pct`.

## Cost Preview

`cost_preview.rs` runs the full analysis pipeline and walks `AnalysisOutput` to compute a field-level token breakdown. Token count estimated as `chars / 4`. Fields consuming >20% of the total are flagged as noise candidates with a `del(...)` suggestion.

## Algorithms

**Template Extraction** — tokenizes entries, extracts patterns with variable slots. Best for structured logs. Typical reduction: 30–40%.

**Edit Distance Clustering** — groups similar entries by Levenshtein distance. Configurable via `similar(t)`. Typical reduction: 70–80%.

**N-gram Outlier Detection** — identifies rare word combinations. Auto-threshold flags bottom ~5%. Typical reduction: 2–5% (outliers only).

**JSON Schema Clustering** — groups JSON objects by structural similarity (field names + types). Singletons flagged as outliers. `--depth N` compares nested schemas.

**Subtree** — walks an arbitrary JSON document, collects every object at every depth, clusters by schema similarity, reports which paths each pattern appears at.

**Value Group-by** — partitions entries by the string value of a field and produces a frequency table. Selected via `group_by(.field)`.

## Output Types

Each algorithm produces a variant of `AlgorithmResults`:

| Variant | Produced by |
|---|---|
| `Grouped` | template, clustering, group_by |
| `OutlierFocused` | ngram |
| `SchemaGrouped` | schema clustering |
| `PathGrouped` | subtree |

`output-schema.json` is the checked-in JSON Schema for `AnalysisOutput`, `DiscoverOutput`, and `CostPreviewOutput`. It is the stable contract for language bindings.

## Metadata System

Each algorithm, formatter, and input format declares a `const` metadata struct alongside its implementation. The registry aggregates these. Adding a new algorithm automatically propagates to CLI help text, `schema.json`, and generated bindings — no manual sync.

## Language Bindings

Both bindings expose a six-function API: `process()`, `processMarkdown()`, `discover()`, `discoverMarkdown()`, `costPreview()`, `costPreviewMarkdown()`. Output types are generated from `output-schema.json` by `tools/gen-types.ts`. `ProcessOptions` (input-side) is hand-written in `gen-types.ts`.

**Python (`bindings/python/`)** — PyO3 native extension built with maturin.

**npm (`bindings/npm/`)** — WASM core compiled with `wasm-pack --target nodejs`, wrapped in TypeScript. Browser target uses `--target web` with async init.

## Workspace Structure

```
core/                    Core library (algorithms, parsers, pipeline, formatters, registry)
cli/                     Command-line interface
tools/dump-schema/       Dev tool: serializes registry → schema.json + output-schema.json
tools/sample-generator/  Dev tool: generates synthetic logs and JSON for testing
tools/gen-types.ts       Dev tool: generates typed output wrappers from output-schema.json
web/                     Web UI (Crank + TypeScript + Tailwind, WASM backend)
bindings/python/         Python binding (PyO3 + maturin, published to PyPI)
bindings/npm/            npm binding (WASM + TypeScript, published to npm)
output-schema.json       Checked-in JSON Schema for output types (contract for bindings)
```

- **Language**: Rust (performance + determinism + WASM support)
- **License**: MIT OR Apache-2.0
