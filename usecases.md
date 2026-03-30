# txtfold: Use Cases

A collection of concrete situations where txtfold is useful, organized by domain. Each entry describes what to feed it, which algorithm tends to work best, and what to look for when assessing quality.

---

## Infrastructure and Config

### Terraform plan compression

`terraform plan` on a large system produces thousands of lines where most changes are structural boilerplate: resource blocks that differ only in names, IDs, or tag values. The meaningful changes — new resource types, destroyed resources, policy changes — are buried.

**Input**: the output of `terraform plan` (plain text)
**Algorithm**: `template` or `clustering`
**Try**: `terraform plan | txtfold` or save to file first
**Look for**: whether structurally identical change blocks (e.g., 40 identical `aws_iam_policy_attachment` updates) collapse into a single representative with a count, while genuinely novel changes (a new `aws_rds_instance`) surface as outliers or distinct patterns
**Improvement signal**: if the reviewer has to scroll past repetitive blocks to find the important change, the reduction isn't aggressive enough

### Kubernetes manifest clustering

A repo with 50 deployments, services, and configmaps. Most share the same schema; a handful are structurally unusual (missing a liveness probe, extra annotations, unexpected field types).

**Input**: a directory of YAML files concatenated, or a single large `kubectl get all -o json` export
**Algorithm**: `schema` (for JSON export) or `template` (for YAML text)
**Try**: `kubectl get deployments -A -o json | jq '.items' | txtfold --algorithm schema`
**Look for**: the schema outliers are your misconfigured or special-cased resources
**Note**: YAML support is not yet implemented; JSON export via `kubectl` is the current path

### Helm chart output analysis

`helm template` renders a chart to hundreds of lines of Kubernetes YAML. Most of it is predictable; the interesting parts are the resources that don't match the common shape.

**Input**: `helm template my-chart | txtfold`
**Algorithm**: `template`
**Look for**: reduction ratio and whether the rendered output's structure is as uniform as expected

### Environment config drift (dev / staging / prod)

You have configs for three environments stored as JSON. They should be structurally identical but in practice aren't — fields added to prod but forgotten elsewhere, optional fields present in some but not all.

**Input**: concatenated JSON objects from each environment as a JSON array
**Algorithm**: `schema` with `--depth 2` or higher
**Look for**: schema clusters where count is not a multiple of your environment count; singletons are drift

### Ansible playbook task analysis

A large playbook with many tasks. Tasks often reuse the same module with varying parameters. Understanding the structural diversity of tasks helps find redundancy or gaps.

**Input**: `ansible-playbook --list-tasks | txtfold` or parse the YAML task list
**Algorithm**: `template`
**Look for**: task name patterns, repeated module usage collapsed to templates

---

## Logs and Observability

### CI test output folding

A failing test suite dumps thousands of lines. Most failures are the same assertion pattern across different test names. The novel failure — the one that explains the root cause — is invisible.

**Input**: captured stdout/stderr from `pytest`, `cargo test`, `jest`, etc.
**Algorithm**: `template` or `clustering`
**Try**: `cargo test 2>&1 | txtfold` or `pytest 2>&1 | txtfold`
**Look for**: whether "FAILED test_X: AssertionError: X != Y" variants collapse correctly, and whether a structurally different failure surfaces as an outlier

### Application log summarization

A server log with millions of lines. Most are routine INFO entries. Errors and warnings are mixed in but hard to find.

**Input**: `server.log` or `journalctl -u myservice | txtfold`
**Algorithm**: `ngram` (to surface rare entries) or `template` (to understand the overall shape)
**Look for**: ngram should surface the genuinely unusual lines — unexpected status codes, new error messages, novel request patterns

### Deployment log compression

After a deploy, you want to know what happened without reading 10,000 lines. Did anything unexpected occur?

**Input**: deployment pipeline output (GitHub Actions logs, CircleCI output, etc.)
**Algorithm**: `template`
**Look for**: reduction ratio; novel lines (new warnings, unfamiliar service names) should surface as outliers

### Incident investigation

Something went wrong. You have logs from the 30 minutes before the incident. What was structurally different from normal?

**Current approach** (manual diff): save "normal" and "incident" log samples; run txtfold on each; compare the summaries manually
**Future**: diff mode (see PLAN.md)
**Algorithm**: `ngram` on the incident window to surface rare entries

### Docker / container event logs

`docker events` or container runtime logs during a problematic deployment. Container restarts, OOM kills, and network errors have distinctive structures.

**Input**: `docker events --format '{{json .}}' | txtfold --input-format json-array`
**Algorithm**: `schema`
**Look for**: event type distribution, structural outliers corresponding to unusual events

---

## APIs and Data

### API response schema analysis

An API that returns slightly different shapes depending on code path, feature flags, or data state. You have a large sample of responses and want to understand the structural variation.

**Input**: a JSON array of API responses captured from a proxy or test suite
**Algorithm**: `schema` with `--depth 2` or `3`
**Look for**: how many distinct schemas exist, what fields are optional in practice vs in the docs, which responses are singletons (edge cases)

### Database query log normalization

A PostgreSQL or MySQL slow query log. Hundreds of queries that are structurally identical except for the literal values. You want to know the distinct query shapes and their frequency.

**Input**: extracted query strings, one per line
**Algorithm**: `template`
**Look for**: query templates with high counts (optimization candidates), novel query shapes (possibly unintended, possibly new features)
**Try**: `grep "Query" /var/log/mysql/slow.log | awk '{$1=$2=""; print}' | txtfold`

### ETL pipeline error clustering

A data pipeline that runs nightly and emits errors. After a schema change upstream, errors multiply. You want to understand the error landscape without reading every line.

**Input**: error log or exception output
**Algorithm**: `clustering` (errors differ mostly in values, not structure)
**Look for**: how many distinct error patterns exist; the rarest ones are worth investigating first

### Webhook payload analysis

You receive webhooks from a third-party system and want to understand the payload schema diversity — which event types send which fields, which are optional.

**Input**: logged webhook payloads as a JSON array
**Algorithm**: `schema` with `--depth 2`
**Look for**: whether event types cluster correctly, which fields are structurally inconsistent

### REST API traffic analysis

HTTP access logs from an API server. You want to understand traffic patterns: which endpoints are called, with what methods, with what response codes.

**Input**: access log (Apache/nginx format)
**Algorithm**: `template`
**Look for**: URL templates (e.g., `GET /api/users/<ID>/orders`) with counts; novel URL patterns as outliers (undocumented endpoints, bots)

---

## LLM and AI Workflows

### LLM context window reduction (general)

Any large text artifact that needs to fit into a prompt: logs, API responses, database exports, error output. Run txtfold first, feed the summary.

**Input**: whatever the LLM would otherwise receive
**Algorithm**: auto
**Look for**: does the summary preserve the information the LLM needs to answer the question? Are there important details that got collapsed?

### RAG pre-indexing deduplication

Before embedding log lines or JSON records, collapse near-duplicates. Embed the representative + count rather than thousands of identical chunks.

**Input**: the corpus you're about to embed
**Algorithm**: `clustering` or `template`
**Look for**: reduction ratio; the outliers txtfold flags are often the semantically interesting entries worth indexing separately

### Fine-tuning dataset deduplication

Before submitting training data, check for structural duplication. High compression ratio = low diversity = potentially degraded fine-tune.

**Input**: training examples (one per line, or as a JSON array)
**Algorithm**: `clustering`
**Look for**: compression ratio as a diversity score; if txtfold can reduce by 70%, your dataset has less diversity than you think

### LLM output diversity measurement

Run the same prompt N times; are the responses structurally diverse or collapsing to a small number of templates?

**Input**: N LLM responses, one per line or as a JSON array
**Algorithm**: `clustering` or `template`
**Look for**: compression ratio as a model behavior metric; low ratio = diverse outputs, high ratio = the model is templating

### Agent tool integration

An LLM agent fetches logs, a database export, or an API response that's too large to reason over directly. txtfold is called as a tool before the result is returned to the agent.

**Try**: Python binding (`from txtfold import process_markdown`) or npm binding for inline use in agent code

---

## Security

### Auth log pattern extraction

Authentication logs have a small number of structural patterns: successful login, failed login, lockout, token refresh. Outliers are often attacks or bugs.

**Input**: auth service logs (syslog format or JSON)
**Algorithm**: `template` to see all patterns, `ngram` to surface the rare ones
**Look for**: novel structural patterns (new endpoints, new field combinations) that don't match expected auth flows

### SIEM pre-processing

Before feeding logs to an expensive ML correlation engine, reduce the volume. Only the structural outliers need deep analysis; the common patterns are already understood.

**Input**: raw log stream for a time window
**Algorithm**: `ngram` (outlier-focused)
**Look for**: what percentage of lines are flagged as outliers; this is your "interesting events" rate

### Network traffic log analysis

DNS query logs, firewall logs, or proxy logs. Most traffic is routine; lateral movement and exfiltration have unusual structural patterns.

**Input**: network log lines
**Algorithm**: `ngram` or `template`
**Look for**: rare destination patterns, unusual query structures, frequency spikes in otherwise-rare templates

---

## Development Workflows

### PR diff compression

A large pull request with hundreds of changed lines. Most are mechanical: import reordering, test boilerplate, type annotation updates. The meaningful logic changes are hard to find.

**Input**: `git diff main...feature-branch | txtfold`
**Algorithm**: `template`
**Look for**: whether structurally identical change lines collapse, leaving the structurally novel hunks visible

### Commit message archaeology

Understand the shape of a project's history without reading every commit.

**Input**: `git log --format="%s" | txtfold`
**Algorithm**: `template` or `clustering`
**Look for**: dominant commit message patterns (e.g., `fix: <COMPONENT>`, `feat: add <NOUN>`), outlier messages that don't follow conventions

### Build output analysis

`cargo build`, `make`, `gradle` — build output is extremely repetitive across incremental builds. Surface warnings and errors that aren't just repeated noise.

**Input**: build stdout/stderr
**Algorithm**: `template`
**Look for**: whether warnings collapse to templates with counts, and whether novel warnings surface as outliers

### Stack trace deduplication

An exception tracker or crash reporter accumulates thousands of stack traces. Most are the same underlying issue at different call sites.

**Input**: stack traces (multi-line entries)
**Algorithm**: `clustering` with `--entry-mode multiline`
**Look for**: how many distinct crash patterns exist; the rare ones may be more interesting than the frequent ones

---

## Data Quality and Pipelines

### Database table export analysis

Export a table as JSON; run schema analysis. Fields that are null in some rows but present in others, unexpected type variations, rows that don't match the common shape — these are data quality issues.

**Input**: `SELECT * FROM table | json` export
**Algorithm**: `schema` with `--depth 1` or `2`
**Look for**: schema outliers = data quality problems; field presence inconsistency = nullable fields that shouldn't be

### CSV/structured data profiling

Before running an ML pipeline or data transformation, understand the structural diversity of your data.

**Input**: convert CSV to JSON array first (`csvjson` or pandas), then run schema analysis
**Algorithm**: `schema`
**Look for**: unexpected structural variation that would cause downstream failures

### Event sourcing audit

In an event-sourced system, the event log is the source of truth but grows enormous. Understand the event type distribution and surface structural anomalies before replay or audit.

**Input**: event log as JSON array
**Algorithm**: `schema`
**Look for**: event types that appear only once (potential data corruption), unexpected field combinations

---

## Scientific and Research

### Experiment run log summarization

Hyperparameter search or HPC job output. Hundreds of runs produce repetitive output; you want the failure patterns and the outlier results.

**Input**: run output logs
**Algorithm**: `clustering` or `ngram`
**Look for**: distinct failure patterns; runs that produced structurally unusual output (worth investigating)

### Benchmark output analysis

Performance benchmark output across many configurations. Most results follow a pattern; outliers are anomalies worth investigating.

**Input**: benchmark output (one result per line or as JSON)
**Algorithm**: `template` (text) or `schema` (JSON)
**Look for**: whether the expected result structure dominates, and what the outlier distribution looks like

---

## Assessing Current Quality

When trying a use case, consider:

1. **Reduction ratio** — is it meaningful? A 10% reduction on logs that should be 80% repetitive means the algorithm isn't matching the structure well.
2. **Outlier relevance** — do the flagged outliers actually matter? If the most "unusual" lines are mundane, the scoring needs tuning.
3. **Representative quality** — does the chosen representative for each cluster actually look like the others in the cluster?
4. **Information loss** — is anything important missing from the summary that was present in the original?
5. **Algorithm selection** — did auto-selection pick the right algorithm? If not, what signal would have indicated the better choice?
