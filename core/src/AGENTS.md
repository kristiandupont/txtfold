# core/src

Rust library crate (`txtfold-core`). Public API is in `lib.rs` — three entry points: `process()`, `discover()`, `cost_preview()`.

## Modules

| Module | Role |
|---|---|
| `lib.rs` | Public API + WASM bindings |
| `pipeline/` | Pipeline DSL — parser, executor, public types |
| `output/` | Analysis result types, builders (one per algorithm), post-processing |
| `formatter.rs` | Markdown/JSON output rendering |
| `discover.rs` | Structural scan (field paths, types, cardinality) |
| `cost_preview.rs` | Token budget estimation |
| `parser.rs` | Entry splitting (SingleLine / MultiLine / Auto) |
| `template.rs` | Template extraction algorithm |
| `clustering.rs` | Edit-distance clustering algorithm |
| `ngram.rs` | N-gram outlier detection algorithm |
| `schema_clustering.rs` | JSON schema clustering algorithm |
| `subtree.rs` | JSON subtree walk algorithm |
| `schema.rs` | JSON schema extraction + similarity |
| `patterns.rs` | Pattern building utilities |
| `tokenizer.rs` | Text tokenizer (Number, Timestamp, Identifier, etc.) |
| `entry.rs` | `Entry` type — content + line-number metadata |
| `metadata.rs` | Algorithm / formatter / input-format metadata structs |
| `registry.rs` | `ALL_ALGORITHMS`, `ALL_FORMATTERS`, `ALL_INPUT_FORMATS` arrays |
