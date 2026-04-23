# pipeline

Pipeline DSL — parses and executes `|`-separated stage expressions like `.items[] | del(.x) | schemas`.

## Files

- `mod.rs` — public types (`Stage`, `AlgorithmDirective`, `PipelineInput`, `PipelineResult`, `ParseError`, etc.) + re-exports + tests
- `tokenizer.rs` — internal lexer (`Token` enum + `Tokenizer`); `pub(super)` only
- `parser.rs` — recursive-descent `Parser`; `pub(super)` only
- `executor.rs` — `parse_pipeline`, `apply_pipeline`, `partition_by_field`, `is_verb_name` + pre-processing helpers

## Stage taxonomy

- **Pre-processing** (`PathSelect`, `Del`, `Where`) — transform JSON input before the algorithm
- **Algorithm** (`AlgorithmVerb`, `GroupBy`) — terminal, selects which algorithm runs
- **Post-processing** (`Top`, `Label`) — applied after the algorithm by the caller
