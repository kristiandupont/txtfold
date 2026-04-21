# txtfold-core

Identifies patterns and outliers in large log files and structured data. No ML, no fuzzy logic — same input always produces the same output.

## Installation

```toml
[dependencies]
txtfold-core = "0.1"
```

## Quick start

```rust
use txtfold_core::{process, ProcessOptions, InputFormat};

// Auto-detect format and algorithm
let output = process(text, &ProcessOptions::default())?;

// Or specify explicitly
let options = ProcessOptions {
    input_format: Some(InputFormat::Line),
    ..Default::default()
};
let output = process(text, &options)?;
```

<!-- docs:syntax-start -->

## Pipeline expressions

The optional first argument selects the algorithm and pre-processes input — like jq's filter argument, but for summarization:

```sh
txtfold 'outliers' app.log
txtfold 'similar(0.8) | top(20)' --format line app.log
txtfold '.diagnostics[] | del(.sourceCode) | group_by(.category)' biome.json
```

If omitted, the default is `summarize` (json → subtree, line/block → template).

**Algorithm verbs** (terminal, selects the algorithm):

| Verb                | Algorithm                                             |
| ------------------- | ----------------------------------------------------- |
| `summarize`         | default per format                                    |
| `similar(t)`        | edit-distance clustering at threshold `t`             |
| `patterns`          | template extraction                                   |
| `outliers`          | n-gram outlier detection                              |
| `schemas`           | JSON schema clustering                                |
| `subtree`           | JSON subtree algorithm                                |
| `group_by(.field)`  | value-based frequency table (JSON)                    |
| `group_by(slot[N])` | value-based frequency table by Nth token (line/block) |

**Pre-processing stages** (JSON only):

- `.field[]` / `.field[*]` / `.field[N]` — navigate into a JSON subtree
- `del(.field, .nested.field, ...)` — remove fields from each JSON object; dotted paths supported
- `where(.field == "value")` — keep only entries matching a condition; operators: `==`, `!=`, `contains`, `starts_with`, `ends_with`

**Post-processing modifiers**:

- `top(N)` — keep the N largest groups
- `label(.field)` — relabel groups using a field value

<!-- docs:syntax-end -->

## Documentation

Full documentation — algorithms, parameters, and output schema — is at **https://kristiandupont.github.io/txtfold/**.
