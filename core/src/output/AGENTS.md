# output

Structured analysis result types and construction logic.

## Files

- `mod.rs` — re-exports everything public
- `types.rs` — all serialisable output types (`AnalysisOutput`, `AlgorithmResults` and its variants, `GroupOutput`, `OutlierOutput`, `SchemaGroupOutput`, `PathPatternOutput`, metadata/summary structs)
- `builder.rs` — `OutputBuilder`: one `build_from_*` method per algorithm; handles budget trimming; tests co-located
- `post_processing.rs` — `apply_top` and `apply_label` (called by `lib.rs` after algorithm runs)

## Key invariant

`summary.*` fields always reflect pre-budget counts so callers can tell how much was elided.
