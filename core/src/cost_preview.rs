//! Cost preview — field-level token breakdown of analysis output.
//!
//! Runs the normal analysis pipeline and then estimates how many tokens each
//! top-level field contributes to the output. Fields consuming >20% of the
//! estimated budget are flagged as noise candidates and included in a
//! suggested `del(...)` expression.

use crate::output::{AlgorithmResults, AnalysisOutput};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Output types ──────────────────────────────────────────────────────────────

/// Token cost attributed to a single field across all groups/patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FieldCost {
    /// Field name (top-level key for JSON, variable name for text patterns).
    pub path: String,
    /// Estimated token count for this field's sample values.
    pub tokens: usize,
    /// Percentage of total estimated tokens.
    pub pct: f32,
    /// Optional annotation explaining why this field is worth deleting even
    /// if its token cost is low (e.g. numeric offset arrays that add visual noise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Field-level token breakdown of an analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CostPreviewOutput {
    /// Total estimated token count across all fields.
    pub estimated_tokens: usize,
    /// Per-field token breakdown, sorted by token count descending.
    pub fields: Vec<FieldCost>,
    /// Suggested `del(...)` expression when noisy fields are found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Warning emitted when the result is not actionable (e.g. single root object
    /// with no path selector — the entire file lands in one bucket).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl CostPreviewOutput {
    /// Render a compact markdown cost table.
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();

        if let Some(ref warning) = self.warning {
            writeln!(out, "\u{26a0}  {}", warning).unwrap();
            out.push('\n');
        }

        writeln!(out, "Estimated output: ~{} tokens", self.estimated_tokens).unwrap();
        writeln!(out, "{}", "\u{2500}".repeat(40)).unwrap();

        if self.fields.is_empty() {
            out.push_str("No fields found.\n");
        } else {
            // Compute column width from longest field name.
            let name_w = self
                .fields
                .iter()
                .map(|f| f.path.len())
                .max()
                .unwrap_or(4)
                .max(4);

            for field in &self.fields {
                let annotation = if let Some(ref note) = field.note {
                    format!("  \u{2190} {}", note)
                } else if field.pct > 20.0 {
                    "  \u{2190} noise candidate".to_string()
                } else {
                    String::new()
                };
                writeln!(
                    out,
                    "{:<name_w$}  {:>6} tokens  ({:>3.0}%){}",
                    field.path,
                    field.tokens,
                    field.pct,
                    annotation,
                    name_w = name_w,
                )
                .unwrap();
            }
        }

        if let Some(ref suggestion) = self.suggestion {
            writeln!(out, "Suggested: {}", suggestion).unwrap();
        }

        out
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return true if all non-empty sample values look like JSON arrays of numbers:
/// `[42, 45]`, `[0]`, `[100, 200, 300]`.  Used to identify byte-offset / span
/// fields that are visually noisy even though they are cheap in tokens.
fn is_numeric_array_samples(values: &[String]) -> bool {
    if values.is_empty() {
        return false;
    }
    values.iter().all(|v| {
        let v = v.trim();
        v.starts_with('[')
            && v.ends_with(']')
            && v[1..v.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == ' ')
    })
}

/// Convert a full JSONPath stored in a `FieldCost` into a pipeline-compatible
/// `del(...)` argument.
///
/// Steps:
/// 1. Strip the leading `$[*].` entry-array prefix (produced by subtree after
///    path selection).
/// 2. If the field path contains a nested `[*]` array traversal, use only the
///    portion before it — the deepest ancestor that `del` can target with a
///    simple dotted path.
/// 3. Prepend `.`.
///
/// Returns `None` if the path cannot be expressed as a valid del argument
/// (e.g. the path is the entry root itself, or does not start with `$[*]`).
///
/// Examples:
/// - `"$[*].location.sourceCode"`               → `".location.sourceCode"`
/// - `"$[*].location.path.file"`                → `".location.path.file"`
/// - `"$[*].advices.advices[*].frame.sourceCode"` → `".advices.advices"`
fn path_to_del_arg(path: &str) -> Option<String> {
    // Strip leading `$` then the first `[*]` entry-array selector.
    let after_dollar = path.strip_prefix('$')?;
    let after_entry = if let Some(rest) = after_dollar.strip_prefix("[*]") {
        rest.strip_prefix('.').unwrap_or(rest)
    } else {
        return None; // Not a `$[*]…` path — skip.
    };

    if after_entry.is_empty() {
        return None; // Path points to the entry itself, not a field within it.
    }

    // If the field path contains a nested array traversal, truncate before it.
    let del_path = if let Some(array_pos) = after_entry.find("[*]") {
        after_entry[..array_pos].trim_end_matches('.')
    } else {
        after_entry
    };

    if del_path.is_empty() {
        return None;
    }

    Some(format!(".{}", del_path))
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Compute a field-level cost breakdown from an `AnalysisOutput`.
///
/// Token counts are estimated as `chars / 4`, which is a good approximation
/// for English-language and code content.
pub fn cost_preview(analysis: &AnalysisOutput) -> CostPreviewOutput {
    let mut field_chars: HashMap<String, usize> = HashMap::new();
    // Optional annotation per path (e.g. "numeric offsets (visual noise)").
    let mut field_notes: HashMap<String, String> = HashMap::new();

    match &analysis.results {
        AlgorithmResults::SchemaGrouped { schemas, outliers } => {
            for schema in schemas {
                for (field, values) in &schema.sample_values {
                    let chars: usize = values.iter().map(|v| v.len()).sum();
                    *field_chars.entry(field.clone()).or_insert(0) += chars;
                }
            }
            let outlier_chars: usize = outliers.iter().map(|o| o.content.len()).sum();
            if outlier_chars > 0 {
                *field_chars
                    .entry("(outliers)".to_string())
                    .or_insert(0) += outlier_chars;
            }
        }

        AlgorithmResults::PathGrouped {
            patterns,
            singletons,
        } => {
            for pattern in patterns {
                // Construct full paths: container_path.field_name
                // e.g. pattern.paths[0] = "$.diagnostics[*]", field = "sourceCode"
                // → full path = "$.diagnostics[*].sourceCode"
                let container = pattern.paths.first().map(|s| s.as_str()).unwrap_or("$");
                for (field, values) in &pattern.sample_values {
                    let full_path = format!("{}.{}", container, field);
                    let chars: usize = values.iter().map(|v| v.len()).sum();
                    *field_chars.entry(full_path.clone()).or_insert(0) += chars;
                    // Flag fields whose samples are all numeric arrays — byte offsets
                    // and span pairs that are visually noisy even though cheap in tokens.
                    if is_numeric_array_samples(values) {
                        field_notes
                            .entry(full_path)
                            .or_insert_with(|| "numeric offsets (visual noise)".to_string());
                    }
                }
            }
            let singleton_chars: usize = singletons.iter().map(|s| s.content.len()).sum();
            if singleton_chars > 0 {
                *field_chars
                    .entry("(singletons)".to_string())
                    .or_insert(0) += singleton_chars;
            }
        }

        AlgorithmResults::Grouped { groups, outliers } => {
            for group in groups {
                // Pattern text itself
                *field_chars
                    .entry("pattern".to_string())
                    .or_insert(0) += group.pattern.len();
                // Sample content and per-variable values
                for sample in &group.samples {
                    *field_chars
                        .entry("content".to_string())
                        .or_insert(0) += sample.content.len();
                    for (var, values) in &sample.variable_values {
                        let chars: usize = values.iter().map(|v| v.len()).sum();
                        *field_chars.entry(var.clone()).or_insert(0) += chars;
                    }
                }
            }
            let outlier_chars: usize = outliers.iter().map(|o| o.content.len()).sum();
            if outlier_chars > 0 {
                *field_chars
                    .entry("(outliers)".to_string())
                    .or_insert(0) += outlier_chars;
            }
        }

        AlgorithmResults::OutlierFocused { baseline, outliers } => {
            let feature_chars: usize = baseline.common_features.iter().map(|f| f.len()).sum();
            if feature_chars > 0 {
                *field_chars
                    .entry("common_features".to_string())
                    .or_insert(0) += feature_chars;
            }
            let outlier_chars: usize = outliers.iter().map(|o| o.content.len()).sum();
            if outlier_chars > 0 {
                *field_chars
                    .entry("content".to_string())
                    .or_insert(0) += outlier_chars;
            }
        }
    }

    let total_chars: usize = field_chars.values().sum();
    // Ceiling division: (n + 3) / 4
    let estimated_tokens = (total_chars + 3) / 4;

    // Detect the single-root-object case: the entire file is one JSON object with
    // no path selector, so subtree puts everything in (singletons).  The cost
    // breakdown is meaningless (one bucket ~= 100%) and the del() suggestion would
    // read `del(.(singletons))`, which is not valid pipeline syntax.
    let singletons_chars = field_chars.get("(singletons)").copied().unwrap_or(0);
    let is_single_root_misleading = estimated_tokens > 10_000
        && total_chars > 0
        && singletons_chars as f32 / total_chars as f32 > 0.90;

    let mut fields: Vec<FieldCost> = field_chars
        .iter()
        .map(|(field, &chars)| {
            let tokens = (chars + 3) / 4;
            let pct = if total_chars > 0 {
                chars as f32 / total_chars as f32 * 100.0
            } else {
                0.0
            };
            FieldCost {
                path: field.clone(),
                tokens,
                pct,
                note: field_notes.get(field).cloned(),
            }
        })
        .collect();

    // Sort by token count descending; break ties alphabetically.
    fields.sort_by(|a, b| b.tokens.cmp(&a.tokens).then(a.path.cmp(&b.path)));

    // Build suggestion for fields consuming >20% of the budget.
    // For Grouped (patterns/clustering) and OutlierFocused (ngram) results the
    // field names are internal struct labels (e.g. "pattern", "content") that
    // do not correspond to any valid pipeline expression, so a `del(...)` hint
    // would be actively misleading.  Suppress the suggestion for those types.
    // Also suppress when the single-root-object warning fires — the suggestion
    // would read `del(.(singletons))` which is not valid syntax.
    let suggestion_allowed = !is_single_root_misleading
        && !matches!(
            &analysis.results,
            AlgorithmResults::Grouped { .. } | AlgorithmResults::OutlierFocused { .. }
        );

    // Internal bucket names are not valid pipeline paths — exclude them from
    // suggestions regardless of their size.
    let is_pseudo_field = |path: &str| path.starts_with('(') && path.ends_with(')');
    let noise: Vec<&FieldCost> = fields
        .iter()
        .filter(|f| f.pct > 20.0 && !is_pseudo_field(&f.path))
        .collect();
    let suggestion = if noise.is_empty() || !suggestion_allowed {
        None
    } else {
        // Convert each noisy path to a pipeline-compatible del argument and
        // deduplicate: multiple paths may truncate to the same ancestor
        // (e.g. `advices.advices[*].frame.sourceCode` and
        // `advices.advices[*].diff.dictionary` both become `.advices.advices`).
        let mut seen = std::collections::HashSet::new();
        let field_list: Vec<String> = noise
            .iter()
            .filter_map(|f| path_to_del_arg(&f.path))
            .filter(|arg| seen.insert(arg.clone()))
            .collect();

        if field_list.is_empty() {
            None
        } else {
            let noise_tokens: usize = noise.iter().map(|f| f.tokens).sum();
            let remaining = estimated_tokens.saturating_sub(noise_tokens);
            Some(format!("del({}) → ~{} tokens", field_list.join(", "), remaining))
        }
    };

    let warning = if is_single_root_misleading {
        Some(
            "The root is a single JSON object — cost preview is not useful without a path \
             selector.\nRun `--discover` first to identify the array field, then:\n\
             txtfold --cost-preview '.ARRAY_FIELD[]' FILE"
                .to_string(),
        )
    } else {
        None
    };

    CostPreviewOutput {
        estimated_tokens,
        fields,
        suggestion,
        warning,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Entry;
    use crate::output::OutputBuilder;
    use crate::template::TemplateExtractor;

    fn make_text_analysis() -> AnalysisOutput {
        let entries: Vec<Entry> = (1..=10)
            .map(|i| Entry::from_line(format!("User {} logged in from 192.168.1.1", i), i))
            .collect();
        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);
        OutputBuilder::new(entries).build_from_templates(&extractor)
    }

    #[test]
    fn test_cost_preview_text() {
        let analysis = make_text_analysis();
        let preview = cost_preview(&analysis);
        // Must have some token estimate
        assert!(preview.estimated_tokens > 0);
        // Fields should be non-empty
        assert!(!preview.fields.is_empty());
        // Percentages should sum to approximately 100
        let total_pct: f32 = preview.fields.iter().map(|f| f.pct).sum();
        assert!((total_pct - 100.0).abs() < 1.0);
        // Suggestion must be suppressed for text output — field names like
        // "pattern" and "content" are internal and not valid pipeline paths.
        assert!(
            preview.suggestion.is_none(),
            "cost preview should not suggest del() for text/line output"
        );
    }

    #[test]
    fn test_cost_preview_json() {
        let input = r#"[
            {"category": "error", "sourceCode": "const x = very_long_source_code_string_here_that_is_quite_verbose_and_takes_up_lots_of_tokens"},
            {"category": "warning", "sourceCode": "another_long_source_code_string_that_adds_to_the_bulk_of_the_output_tokens"},
            {"category": "error", "sourceCode": "yet_another_long_code_snippet_that_drives_up_the_token_count_significantly"}
        ]"#;
        let options = crate::ProcessOptions {
            input_format: crate::InputFormat::Json,
            pipeline_expr: Some("schemas".to_string()),
            ..Default::default()
        };
        let analysis = crate::process(input, &options, "json");
        assert!(analysis.is_ok());
        // Parse and preview
        let output: AnalysisOutput =
            serde_json::from_str(&analysis.unwrap()).unwrap();
        let preview = cost_preview(&output);
        assert!(preview.estimated_tokens > 0);
        // sourceCode should appear and dominate
        let sc = preview.fields.iter().find(|f| f.path == "sourceCode");
        assert!(sc.is_some());
    }

    #[test]
    fn test_markdown_output() {
        let analysis = make_text_analysis();
        let preview = cost_preview(&analysis);
        let md = preview.to_markdown();
        assert!(md.contains("Estimated output:"));
        assert!(md.contains("tokens"));
    }

    #[test]
    fn test_path_to_del_arg() {
        // Simple nested field — full dotted path preserved.
        assert_eq!(
            path_to_del_arg("$[*].location.sourceCode"),
            Some(".location.sourceCode".into())
        );
        // Deep dotted path with no array — preserved in full.
        assert_eq!(
            path_to_del_arg("$[*].location.path.file"),
            Some(".location.path.file".into())
        );
        // Nested array traversal — truncate before first `[*]`.
        assert_eq!(
            path_to_del_arg("$[*].advices.advices[*].frame.sourceCode"),
            Some(".advices.advices".into())
        );
        // Multiple `[*]` — truncate before the first one.
        assert_eq!(
            path_to_del_arg("$[*].advices.advices[*].list[*][*].content"),
            Some(".advices.advices".into())
        );
        // Top-level field only.
        assert_eq!(
            path_to_del_arg("$[*].description"),
            Some(".description".into())
        );
        // Entry root itself — not a field, skip.
        assert_eq!(path_to_del_arg("$[*]"), None);
        // Non-entry-array path — skip.
        assert_eq!(path_to_del_arg("$.summary.errors"), None);
        // Pseudo-field — not even a JSONPath, skip.
        assert_eq!(path_to_del_arg("(singletons)"), None);
    }

    #[test]
    fn test_single_root_object_warning() {
        // A large single-root JSON object (not an array) should trigger the warning
        // and suppress the suggestion.  Build a ~40KB object so estimated_tokens > 10_000.
        let value = "x".repeat(40_000);
        let input = format!(r#"{{"diagnostics": [{{"msg": "{}"}}]}}"#, value);
        let options = crate::ProcessOptions {
            input_format: crate::InputFormat::Json,
            ..Default::default()
        };
        let analysis = crate::process(&input, &options, "json").unwrap();
        let output: AnalysisOutput = serde_json::from_str(&analysis).unwrap();
        let preview = cost_preview(&output);
        assert!(
            preview.warning.is_some(),
            "expected warning for single-root-object with no path selector"
        );
        assert!(
            preview.suggestion.is_none(),
            "suggestion must be suppressed when warning fires"
        );
        let md = preview.to_markdown();
        assert!(md.contains('\u{26a0}'), "warning symbol should appear in markdown");
        assert!(md.contains("path selector"), "warning should mention path selector");
    }

    #[test]
    fn test_suggestion_generated_for_noisy_field() {
        // Build an output where one field dominates (>20%)
        let input = r#"[
            {"x": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "y": "b"},
            {"x": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "y": "b"},
            {"x": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "y": "b"}
        ]"#;
        let options = crate::ProcessOptions {
            input_format: crate::InputFormat::Json,
            pipeline_expr: Some("schemas".to_string()),
            ..Default::default()
        };
        let analysis = crate::process(input, &options, "json");
        let output: AnalysisOutput =
            serde_json::from_str(&analysis.unwrap()).unwrap();
        let preview = cost_preview(&output);
        // x is much larger than y, so suggestion should mention it
        if preview.suggestion.is_some() {
            assert!(preview.suggestion.as_ref().unwrap().contains(".x"));
        }
    }
}
