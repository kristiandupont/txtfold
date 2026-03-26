# txtfold: Plan

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the system works.

## What exists

Core library with five algorithms (template, clustering, ngram, schema, subtree), a fully functional CLI driven by the registry, a working web UI (Crank + WASM), a metadata/registry system, and `output-schema.json` as a checked-in contract for external consumers. 123 unit tests passing.

**Nested JSON support** is complete:
- `schema --depth N` (default 1): compares nested object schemas recursively up to N levels. Objects with the same top-level field set but structurally different sub-objects are placed in separate clusters.
- `subtree`: walks a single arbitrary JSON document, collects every object at every depth, clusters by schema similarity, and reports which normalized paths (`$.users[*]`, `$.team.members[*]`, …) each structural pattern appears at.

**Language bindings** are complete:
- `bindings/python/` — PyO3 native extension, published as `txtfold` on PyPI. `process()` returns a typed `AnalysisOutput` dict; `process_markdown()` returns a string.
- `bindings/npm/` — WASM core (`wasm-pack --target nodejs`) + TypeScript wrapper, published as `txtfold` on npm. Same API shape. Both bindings ship short READMEs that defer full documentation to the hosted docs site.
- `tools/gen-types.ts` — generates `bindings/npm/src/types.ts` (TypeScript interfaces) and `bindings/python/txtfold/_types.py` (TypedDicts) from `output-schema.json`. Run with `bun tools/gen-types.ts`. Wired into the npm build as `bun run gen-types`.

## Todo

**Packaging and Launch** — Homebrew, APT, GitHub releases, man page generated from `schema.json`. README (use cases, quick start, examples), CI/CD (build + test + release binaries for CLI; per-platform wheel matrix for PyPI; single WASM bundle for npm), crates.io publish, web UI deployment. These are deferred until the above is solid.

## Future work

**Pipeline features** — oriented around AI/RAG use cases:

- `--budget` / `--max-output-lines`: caller specifies context window size, txtfold chooses reduction aggressiveness
- `--sample N`: emit N representatives per cluster instead of 1, for dataset diversity
- Diff mode: given two inputs, report patterns that appeared, disappeared, or changed frequency ("what changed since last deploy?")
- Stable pattern fingerprints: hashed template IDs that persist across runs for pattern lifetime tracking

**Sectioning** — detect distinct sections in a file, apply different algorithms per section.

**Additional formats** — CSV, XML/YAML, Parquet/Arrow.

**Streaming** — for very large files (>1GB), process without loading fully into memory.
