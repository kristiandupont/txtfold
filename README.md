# txtfold

Identifies repeated patterns and surfaces outliers in large log files and structured data. Converts thousands of lines into a human- or LLM-readable summary.

No ML, no fuzzy logic — same input always produces the same output.

## Why

Log files and structured records are repetitive by nature. When you need to reason over them — manually, or as context for an LLM — most of the content is noise. txtfold collapses the repetition, keeps the signal, and tells you exactly what it did.

**One-off analysis**: summarize a large log file before reading it, or before handing it to an LLM.

**In a pipeline**: reduce logs before they reach a context window; deduplicate records before embedding them; surface anomalies for a monitoring agent.

## Use cases

**Reducing logs for LLM context windows** — you have 50K lines of logs and a bounded context window. Run txtfold first, feed the summary. Works well in CI pipelines (reduce test output before sending to a triage agent), incident response (reduce recent logs before root-cause analysis), or anywhere you need to fit large text into a prompt.

**RAG pre-indexing** — before embedding log lines or JSON records, collapse near-duplicates into canonical representatives. You embed the representative + count rather than thousands of repetitive chunks, reducing both embedding cost and retrieval noise. The outliers txtfold flags are often the semantically interesting entries worth indexing separately.

**Anomaly detection for monitoring agents** — an LLM-based monitoring agent doesn't need every log line, it needs to know what changed. The n-gram algorithm already identifies statistically rare entries; combined with a diff across two time windows it becomes a "what changed since last deploy?" primitive.

**Agent tool integration** — LLM agents can call txtfold as a tool when they fetch logs, database exports, or API responses too large to reason over directly. The Python and TypeScript/npm bindings are the natural surface for this.

**Fine-tuning dataset deduplication** — before sampling training data from logs or structured text, use txtfold to cluster and deduplicate. One representative per cluster rather than thousands of copies of the same line.

## Quick start

```sh
cargo install txtfold
txtfold server.log
```

Output defaults to Markdown. For machine-readable output:

```sh
txtfold --format json server.log
```

Pipe from stdin:

```sh
cat server.log | txtfold
```

## Algorithms

txtfold automatically selects an algorithm based on your input. You can override with `--algorithm`.

| Algorithm    | Best for                                              | Typical reduction    |
| ------------ | ----------------------------------------------------- | -------------------- |
| `template`   | Structured logs with clear token patterns             | 30–40%               |
| `clustering` | Entries differing only in IDs, numbers, service names | 70–80%               |
| `ngram`      | Finding unusual entries in otherwise uniform logs     | 2–5% (outliers only) |
| `schema`     | JSON arrays or maps with varying field sets           | varies               |
| `subtree`    | Single JSON documents with repeated sub-schemas       | varies               |

**template** — extracts patterns with variable slots:

```
[<TIMESTAMP>] INFO GET /api/users 200 <NUM>ms   (×2847)
[<TIMESTAMP>] WARN GET /api/orders 404          (×312)
```

**clustering** — groups similar entries by edit distance, showing one representative per cluster.

**ngram** — scores entries by how unusual their word combinations are. Only reports the bottom ~5% (auto-tuned). Good for finding the needle in the haystack.

**schema** — for JSON input. Groups objects by structural similarity (which fields are present and what types they have). Singletons are flagged as outliers. Use `--depth N` (default 1) to compare nested object schemas — objects that look identical at the top level but have structurally different sub-objects will be placed in separate clusters.

**subtree** — for a single arbitrary JSON document. Walks the entire tree, collects every object at every depth, and clusters them by schema similarity. Reports which paths each structural pattern appears at:

```
Pattern (47 objects)
Schema: { id: number, name: string, email: string }
Appears at:
  $.users[*]
  $.team.members[*]
  $.config.owner
```

Useful for API responses, config files, or exports where the same shape recurs at unpredictable locations.

### Parameters

```sh
--threshold 0.8          # Clustering/schema similarity threshold (0.0–1.0)
--depth 1                # Schema nesting depth for --algorithm schema (0 = flat)
--ngram-size 2           # N-gram window size
--outlier-threshold 0.0  # N-gram cutoff (0.0 = auto)
--entry-mode multiline   # Force multi-line entry parsing (for stack traces)
```

## Input formats

txtfold auto-detects whether input is plain text, a JSON array, or a JSON map. Override with `--input-format`.

Multi-line entries (stack traces, structured log blocks) are detected automatically via timestamp boundary detection.

## Output formats

- `markdown` (default) — human-readable summary with reduction stats
- `json` — structured output, suitable for downstream processing

The JSON output schema is documented in `output-schema.json`.

## Compared to alternatives

**`sort | uniq -c`** — handles exact duplicates only. txtfold's template and clustering algorithms collapse near-duplicates: entries that differ only in timestamps, IDs, or numeric values still count as the same pattern.

**Drain, logmine, LogPai** — ML-based log parsers that require fitting a model to your data. Results vary across runs and deployments. txtfold is fully deterministic: the same input always produces the same output, which matters in CI pipelines and monitoring workflows.

**Embeddings + vector search** — a valid approach when you want to query logs semantically, but requires infrastructure (an embedding model, a vector store) and produces results only a model can consume. txtfold's output is plain text readable by both humans and LLMs, with no external dependencies and no per-run cost.

**lnav, grep, awk** — tools for navigating and filtering logs, not summarizing them. They show you lines that match a pattern; txtfold tells you what the patterns are.

## Project status

Core algorithms, CLI, and language bindings (Python/PyPI, TypeScript/npm) are complete and tested. Web UI is working. Packaging (Homebrew, APT, GitHub releases) and hosted docs are next.

See [ARCHITECTURE.md](ARCHITECTURE.md) for how it works internally.
