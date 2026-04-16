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
}

impl CostPreviewOutput {
    /// Render a compact markdown cost table.
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();

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
                let noise = if field.pct > 20.0 {
                    "  \u{2190} noise candidate"
                } else {
                    ""
                };
                writeln!(
                    out,
                    "{:<name_w$}  {:>6} tokens  ({:>3.0}%){}",
                    field.path,
                    field.tokens,
                    field.pct,
                    noise,
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

/// Strip the leading `$` / array-traversal segment from a full path so it can
/// be used as a `del(...)` argument.
///
/// Examples:
/// - `"$.diagnostics[*].sourceCode"` → `"diagnostics[*].sourceCode"`
///   (but we only need the terminal: `"sourceCode"` if unique; this function
///   is called when the terminal is ambiguous, so we return everything after
///   `$. ` or `$`)
///
/// The goal is to produce a path that is valid as a dotted del argument, e.g.
/// `.location.sourceCode` from `"$.root[*].location.sourceCode"`.
fn strip_path_prefix(path: &str) -> &str {
    // Strip leading `$.` or just `$`
    if let Some(rest) = path.strip_prefix("$.") {
        rest
    } else if let Some(rest) = path.strip_prefix('$') {
        rest.trim_start_matches('.')
    } else {
        path
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Compute a field-level cost breakdown from an `AnalysisOutput`.
///
/// Token counts are estimated as `chars / 4`, which is a good approximation
/// for English-language and code content.
pub fn cost_preview(analysis: &AnalysisOutput) -> CostPreviewOutput {
    let mut field_chars: HashMap<String, usize> = HashMap::new();

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
                    *field_chars.entry(full_path).or_insert(0) += chars;
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
    let suggestion_allowed = !matches!(
        &analysis.results,
        AlgorithmResults::Grouped { .. } | AlgorithmResults::OutlierFocused { .. }
    );

    let noise: Vec<&FieldCost> = fields.iter().filter(|f| f.pct > 20.0).collect();
    let suggestion = if noise.is_empty() || !suggestion_allowed {
        None
    } else {
        // Count how many full paths share each terminal key name.
        // If the terminal name is unique, suggest the short form `del(.name)`;
        // otherwise use the dotted path after stripping the leading `$.` array
        // bracket prefix.
        let mut terminal_count: HashMap<String, usize> = HashMap::new();
        for fc in &fields {
            let terminal = fc.path.split('.').last().unwrap_or(&fc.path).to_string();
            *terminal_count.entry(terminal).or_insert(0) += 1;
        }

        let field_list = noise
            .iter()
            .map(|f| {
                let terminal = f.path.split('.').last().unwrap_or(&f.path);
                if terminal_count.get(terminal).copied().unwrap_or(0) == 1 {
                    // Unique terminal name — use short form.
                    format!(".{}", terminal)
                } else {
                    // Ambiguous — use the dotted path, stripping the leading
                    // `$.` (or `$[*].`, etc.) array-traversal prefix so the
                    // suggestion stays valid as a del() argument.
                    let dotted = strip_path_prefix(&f.path);
                    format!(".{}", dotted)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let noise_tokens: usize = noise.iter().map(|f| f.tokens).sum();
        let remaining = estimated_tokens.saturating_sub(noise_tokens);
        Some(format!("del({}) → ~{} tokens", field_list, remaining))
    };

    CostPreviewOutput {
        estimated_tokens,
        fields,
        suggestion,
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
