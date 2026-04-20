# txtfold Experiment Plan

txtfold is a tool that enables you to surface important information in large files, suitable for LLM consumption

A repeatable process for exploring an unfamiliar file and iterating toward a useful, readable analysis. The goal is output that is small enough to read in full but surfaces the most important structure, patterns, and outliers.

Note: You can run txtfold from the ./target/debug folder.

---

## Step 0 — Remind yourself of the syntax

If you need a reference at any point:

```sh
txtfold --syntax
```

---

## Step 1 — Discover the structure

Run discover to get a schema map of the file. This tells you what fields exist, their types, cardinalities, and sample values — without running a full analysis.

```sh
txtfold --discover FILE
```

**What to look for:**

- **High-cardinality string fields** (cardinality close to entry count): these are likely unique identifiers, source code snippets, or free-form text. They are noise candidates — `del` them before analysis.
- **Low-cardinality string fields** (cardinality 2–50): these are good `group_by` candidates (status codes, categories, severity levels, event types).
- **Fields present in <100% of entries**: may indicate optional structure worth understanding separately.
- **Very long sample values**: a sign that a field contains embedded blobs (source code, stack traces, base64). Delete these.

For JSON: note the path prefix that contains the interesting entries (e.g. `.diagnostics[]`, `.events[]`, `.Records[]`). For line/block: note which token slots carry meaningful signal.

---

## Step 2 — Estimate the cost before committing

Before running a full analysis, check where the output budget will go:

```sh
txtfold --cost-preview FILE
txtfold --cost-preview 'YOUR_PIPELINE' FILE   # with a draft pipeline
```

The output shows estimated token counts per field and suggests `del(...)` candidates. Use this to validate that your `del` choices actually remove the bulk of the noise before spending time reading a large output.

A good target is under ~2,000 tokens of output (roughly what fits comfortably in one screen or one LLM context slot). If the estimate is much larger, add more `del` clauses or narrow the path selection.

---

## Step 3 — First analysis pass

Run a full analysis with the fields identified in steps 1–2 removed.

**For JSON files:**

```sh
txtfold '.root_array[] | del(.noisy_field, .another_noisy_field) | summarize' FILE
```

If you already know a good grouping key from discover:

```sh
txtfold '.root_array[] | del(.noisy_field) | group_by(.category_field)' FILE
```

**For line/block logs:**

```sh
txtfold 'patterns' FILE
txtfold 'similar(0.8)' FILE      # if entries are very similar to each other
txtfold 'outliers' FILE          # if you are looking for anomalies
```

Read the output and note:

- Are the groups meaningful or are they still too wide (i.e., one catch-all group)?
- Are there groups that are large but uninformative (all entries identical after del)?
- Are there outliers that look important?
- Is the output still too long to read comfortably?

---

## Step 4 — Iterate

Tighten the pipeline based on what you observed in step 3.

**Common adjustments:**

| Observation                                          | Action                                                                    |
| ---------------------------------------------------- | ------------------------------------------------------------------------- |
| Groups are too broad, all entries look the same      | Switch from `summarize` to `group_by(.field)` with a more specific field  |
| Output is too long                                   | Add `top(N)` to focus on the largest groups; or add more `del` fields     |
| One group dominates and is uninteresting             | `del` the field that defines that group, or filter it with path selection |
| Groups are too narrow (every entry is its own group) | Switch algorithm: try `patterns` or `similar(0.8)`                        |
| Important detail is hidden inside a kept field       | Accept the size cost, or open the raw file to read that field directly    |
| Outliers look noisy                                  | Add `top(N)` or `del` the fields driving outlier detection                |

Re-run `--cost-preview` after any change to the pipeline before doing the full run.

---

## Step 5 — Check the result

A good final output has:

- **One line of context** at the top: entry count, group count, algorithm.
- **Groups with distinct, readable descriptions** — if every group looks the same, more `del` or a different `group_by` key is needed.
- **Outliers that stand out** from the groups — if there are none, that is a result too (the file is uniform).
- **Fits on one screen** or close to it — if not, add `top(N)` or reduce further.

---

## Reference: common pipeline shapes

```sh
# JSON — group by a category field, strip source code and advice blobs
txtfold '.diagnostics[] | del(.sourceCode, .advices, .location) | group_by(.category)' FILE

# JSON — explore schema variety across entries
txtfold '.items[] | schemas' FILE

# JSON — find repeated subtree shapes in a single document
txtfold 'subtree' FILE

# Line logs — template extraction (default for line format)
txtfold 'patterns' FILE

# Line logs — cluster by similarity, focus on the top 20 groups
txtfold 'similar(0.8) | top(20)' FILE

# Line logs — surface rare entries
txtfold 'outliers' FILE

# Block logs (e.g. stack traces) — declare entry boundaries by timestamp prefix
txtfold --format block --entry-pattern '^\d{4}-\d{2}-\d{2}' 'patterns' FILE

# Narrow a JSON document before analysis
txtfold '.response.events[] | del(.timestamp, .requestId) | group_by(.type)' FILE
```
