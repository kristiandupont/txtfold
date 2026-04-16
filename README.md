# txtfold

Identifies repeated patterns and surfaces outliers in large log files and structured data. Converts thousands of lines into a human- or LLM-readable summary.

No ML, no fuzzy logic — same input always produces the same output.

## Why

Log files and structured records are repetitive by nature. When you need to reason over them — manually, or as context for an LLM — most of the content is noise. txtfold collapses the repetition, keeps the signal, and tells you exactly what it did.

**One-off analysis**: summarize a large log file before reading it, or before handing it to an LLM.

**In a pipeline**: reduce logs before they reach a context window; deduplicate records before embedding them; surface anomalies for a monitoring agent.

## Quick start

```sh
cargo install txtfold
txtfold server.log
```

Output defaults to Markdown. For machine-readable output:

```sh
txtfold --output-format json server.log
```

Pipe from stdin (format must be declared explicitly):

```sh
cat server.log | txtfold --format line
```

## Pipeline expressions

The optional first argument selects the algorithm and pre-processes input — like jq's filter argument, but for summarization:

```sh
txtfold 'outliers' app.log
txtfold 'similar(0.8) | top(20)' --format line app.log
txtfold '.diagnostics[] | del(.sourceCode) | group_by(.category)' biome.json
```

If omitted, the default is `summarize` (json → subtree, line/block → template).

**Algorithm verbs** (terminal, selects the algorithm):

| Verb | Algorithm |
|---|---|
| `summarize` | default per format |
| `similar(t)` | edit-distance clustering at threshold `t` |
| `patterns` | template extraction |
| `outliers` | n-gram outlier detection |
| `schemas` | JSON schema clustering |
| `subtree` | JSON subtree algorithm |
| `group_by(.field)` | value-based frequency table (JSON) |
| `group_by(slot[N])` | value-based frequency table by Nth token (line/block) |

**Pre-processing stages** (JSON only):
- `.field[]` / `.field[*]` / `.field[N]` — navigate into a JSON subtree
- `del(.field, .nested.field, ...)` — remove fields from each JSON object; dotted paths supported

**Post-processing modifiers**:
- `top(N)` — keep the N largest groups
- `label(.field)` — relabel groups using a field value

## Discover mode

Before running analysis, use `--discover` to get a compact structural map of your data — field paths, types, cardinality, and sample values:

```sh
txtfold --discover biome-output.json
```

```
Format: json  |  Entries: 1,842
Path                            Types    Cardinality  Samples
$.diagnostics[*].category       string             6  "error", "warning", …
$.diagnostics[*].severity       number             3  "1", "2", "3"
$.diagnostics[*].sourceCode     string          1842  "const x = …", …

Pipeline selector: .diagnostics[]
```

Use this first when you don't know the document structure, or to understand which fields are worth keeping before writing a pipeline expression.

## Cost preview

Use `--cost-preview` to see where the output token budget is going:

```sh
txtfold --cost-preview biome-output.json
```

```
Estimated output: ~9,200 tokens
────────────────────────────────────────
sourceCode    6,100 tokens  ( 66%)  ← noise candidate
dictionary    1,800 tokens  ( 20%)  ← noise candidate
category         80 tokens  (  1%)
...
Suggested: del(.sourceCode, .dictionary) → ~1,300 tokens
```

Fields consuming more than 20% of the estimated token budget are flagged as noise candidates. Use this before handing output to an LLM to avoid burning context on high-cardinality fields.

## Algorithms

**template** — extracts patterns with variable slots:

```
[<TIMESTAMP>] INFO GET /api/users 200 <NUM>ms   (×2847)
[<TIMESTAMP>] WARN GET /api/orders 404          (×312)
```

**similar(t)** — groups entries by edit distance, showing one representative per cluster. `t` is the similarity threshold (0.0–1.0, default 0.8).

**outliers** — scores entries by how unusual their word combinations are. Reports the bottom ~5% (auto-tuned). Good for finding the needle in the haystack.

**schemas** — for JSON input. Groups objects by structural similarity (which fields are present and what types they have). Use `--depth N` (default 1) to compare nested object schemas.

**subtree** — for a single arbitrary JSON document. Walks the entire tree and reports which paths each structural pattern appears at:

```
Pattern (47 objects)
Schema: { id: number, name: string, email: string }
Appears at: $.users[*], $.team.members[*], $.config.owner
```

**group_by(.field)** — partitions entries by the value of a field and produces a frequency table.

## Input formats

Declare the format explicitly with `--format`:

| Format  | Use for | Entry splitting |
|---|---|---|
| `line` | Plain text logs, CSV | One entry per line |
| `block` | Stack traces, multi-line log blocks | `--entry-pattern <regex>`, or timestamp heuristic |
| `json` | JSON arrays or maps | One array element / map value per entry |

For files, format is inferred from the extension (`.json` → json, anything else → line). For stdin, `--format` is required.

## Parameters

```sh
--depth 1                # Schema nesting depth for schemas/subtree (0 = flat)
--ngram-size 2           # N-gram window size for 'outliers'
--outlier-threshold 0.0  # N-gram cutoff (0.0 = auto)
--entry-pattern <regex>  # Regex marking the start of each entry (block format)
--budget <N>             # Maximum output lines
--syntax                 # Print pipeline syntax reference and exit
```

## Output formats

- `markdown` (default) — human-readable summary with a compact header line (`N entries → M groups  (algorithm)`) followed by pattern groups or outliers
- `json` — structured output matching `output-schema.json`

## Language bindings

**Python**:

```python
import txtfold
result = txtfold.process(data, input_format="json", pipeline="del(.sourceCode) | schemas")
```

**TypeScript/Node**:

```ts
import { process } from "txtfold";
const result = process(data, { inputFormat: "json", pipeline: "del(.sourceCode) | schemas" });
```

## Compared to alternatives

**`sort | uniq -c`** — handles exact duplicates only. txtfold's template and clustering algorithms collapse near-duplicates: entries that differ only in timestamps, IDs, or numeric values count as the same pattern.

**Drain, logmine, LogPai** — ML-based log parsers that require fitting a model to your data. txtfold is fully deterministic: the same input always produces the same output.

**Embeddings + vector search** — requires infrastructure (an embedding model, a vector store) and produces results only a model can consume. txtfold's output is plain text readable by both humans and LLMs.

**lnav, grep, awk** — tools for filtering logs, not summarizing them. They show you lines that match a pattern; txtfold tells you what the patterns are.

## Project status

Core algorithms, CLI, pipeline expressions, and language bindings (Python/PyPI, TypeScript/npm) are complete and tested. Web UI is working. Packaging (Homebrew, APT, GitHub releases) and hosted docs are next.

See [ARCHITECTURE.md](ARCHITECTURE.md) for how it works internally.
