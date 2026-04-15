//! txtfold: Deterministic pattern summarization for log files and structured data
//!
//! This library provides algorithms for identifying patterns in large text files,
//! extracting templates, and detecting outliers.

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod clustering;
pub mod cost_preview;
pub mod discover;
pub mod entry;
pub mod formatter;
pub mod metadata;
pub mod ngram;
pub mod output;
pub mod parser;
pub mod pipeline;
pub mod patterns;
pub mod registry;
pub mod schema;
pub mod schema_clustering;
pub mod subtree;
pub mod template;
pub mod tokenizer;

/// Core library functionality
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn wasm_version() -> String {
    version().to_string()
}

/// Explicit input format declaration.
#[derive(Debug, Clone)]
pub enum InputFormat {
    /// JSON input (array or map/object) — path selection applies.
    Json,
    /// Line-delimited input — one entry per line (logs, CSV).
    Line,
    /// Block input — multi-line entries (stack traces, Terraform plans).
    /// Entry boundaries are detected by `entry_pattern` regex, or by the
    /// multiline heuristic (timestamp + indentation) as a fallback.
    Block {
        /// Optional regex pattern that matches the start of each new entry.
        entry_pattern: Option<String>,
    },
}

/// Options for a `process()` call.
///
/// The algorithm is selected by the terminal verb of `pipeline_expr`
/// (or `summarize` by default, which maps per format:
/// json → subtree, line/block → template).
/// `--algorithm` and `--threshold` flags are no longer separate options;
/// use `similar(0.8)` etc. in the pipeline expression instead.
pub struct ProcessOptions {
    /// Explicit input format (required).
    pub input_format: InputFormat,
    /// Optional pipeline expression (e.g. `"del(.x) | schemas"`).
    pub pipeline_expr: Option<String>,
    /// Maximum output lines (`None` = unlimited).
    pub budget: Option<usize>,
    /// N-gram size for the `outliers` algorithm (default 2).
    pub ngram_size: usize,
    /// Outlier score threshold for the `outliers` algorithm (0.0 = auto).
    pub outlier_threshold: f64,
    /// Nesting depth for the `subtree` algorithm (default 1).
    pub depth: usize,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        ProcessOptions {
            input_format: InputFormat::Line,
            pipeline_expr: None,
            budget: None,
            ngram_size: 2,
            outlier_threshold: 0.0,
            depth: 1,
        }
    }
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Run a text algorithm on `entries` and return `AnalysisOutput`.
fn run_text_algorithm(
    directive: &pipeline::AlgorithmDirective,
    entries: Vec<crate::entry::Entry>,
    ngram_size: usize,
    outlier_threshold: f64,
    budget: Option<usize>,
) -> Result<crate::output::AnalysisOutput, String> {
    use crate::clustering::EditDistanceClusterer;
    use crate::ngram::NgramOutlierDetector;
    use crate::output::OutputBuilder;
    use crate::template::TemplateExtractor;
    use pipeline::AlgorithmDirective;

    Ok(match directive {
        AlgorithmDirective::Summarize | AlgorithmDirective::Patterns => {
            let mut extractor = TemplateExtractor::new();
            extractor.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_templates(&extractor)
        }
        AlgorithmDirective::Similar(threshold) => {
            let mut clusterer = EditDistanceClusterer::new(*threshold);
            clusterer.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_clusters(&clusterer)
        }
        AlgorithmDirective::Outliers => {
            let mut detector = NgramOutlierDetector::new(ngram_size, outlier_threshold);
            detector.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_ngrams(&detector)
        }
        AlgorithmDirective::Schemas | AlgorithmDirective::Subtree => {
            return Err(
                "the 'schemas' and 'subtree' algorithms require JSON input; \
                 use --format json"
                    .to_string(),
            );
        }
    })
}

/// Run a JSON algorithm on `values` and return `AnalysisOutput`.
fn run_json_algorithm(
    directive: &pipeline::AlgorithmDirective,
    group_by_field: Option<&str>,
    values: Vec<serde_json::Value>,
    threshold: f64,
    depth: usize,
    budget: Option<usize>,
    input_file: Option<String>,
) -> Result<crate::output::AnalysisOutput, String> {
    use crate::output::OutputBuilder;
    use crate::schema_clustering::SchemaClusterer;
    use crate::subtree::SubtreeFinder;
    use pipeline::AlgorithmDirective;

    // group_by overrides the algorithm entirely.
    if let Some(field) = group_by_field {
        let (groups, ungrouped) = pipeline::partition_by_field(&values, field);
        let mut builder = OutputBuilder::new(vec![]);
        if let Some(name) = input_file { builder = builder.with_input_file(name); }
        if let Some(b) = budget { builder = builder.with_budget(b); }
        return Ok(builder.build_from_value_groups(field, &groups, &ungrouped, &values));
    }

    Ok(match directive {
        AlgorithmDirective::Summarize | AlgorithmDirective::Subtree => {
            // Build a single-element root from the values array for SubtreeFinder.
            let root = serde_json::Value::Array(values.clone());
            let mut finder = SubtreeFinder::new(threshold);
            finder.process(&root);
            let mut builder = OutputBuilder::new(vec![]);
            if let Some(name) = input_file { builder = builder.with_input_file(name); }
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_subtree(&finder, &root)
        }
        AlgorithmDirective::Schemas => {
            let mut clusterer = SchemaClusterer::new(threshold, depth);
            clusterer.process(&values);
            let mut builder = OutputBuilder::new(vec![]);
            if let Some(name) = input_file { builder = builder.with_input_file(name); }
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_schemas(&clusterer, &values)
        }
        AlgorithmDirective::Similar(t) => {
            // clustering on JSON objects by serialised form
            let mut clusterer = SchemaClusterer::new(*t, depth);
            clusterer.process(&values);
            let mut builder = OutputBuilder::new(vec![]);
            if let Some(name) = input_file { builder = builder.with_input_file(name); }
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_schemas(&clusterer, &values)
        }
        AlgorithmDirective::Patterns => {
            return Err(
                "the 'patterns' algorithm requires line or block input; \
                 use --format line or --format block"
                    .to_string(),
            );
        }
        AlgorithmDirective::Outliers => {
            return Err(
                "the 'outliers' algorithm requires line or block input; \
                 use --format line or --format block"
                    .to_string(),
            );
        }
    })
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run an analysis and return structured `AnalysisOutput`.
pub fn process_to_output(
    input: &str,
    options: &ProcessOptions,
    input_file: Option<String>,
) -> Result<crate::output::AnalysisOutput, String> {
    use crate::output::{apply_label, apply_top};
    use crate::parser::{is_json_map, parse_json_array, parse_json_map, EntryMode, EntryParser};
    use pipeline::{apply_pipeline, parse_pipeline, AlgorithmDirective, PipelineInput};

    // Default threshold for schema/subtree/similar when not given via pipeline.
    const DEFAULT_THRESHOLD: f64 = 0.8;

    // Parse the pipeline expression (or use the default: implicit `summarize`).
    let (stages, has_pipeline) = if let Some(ref expr) = options.pipeline_expr {
        (parse_pipeline(expr).map_err(|e| e.to_string())?, true)
    } else {
        (vec![], false)
    };

    // Build the initial PipelineInput from the raw text.
    let initial_input = match &options.input_format {
        InputFormat::Json => {
            let is_map = is_json_map(input);
            let values = if is_map {
                let (values, _keys) = parse_json_map(input)
                    .map_err(|e| format!("Failed to parse JSON map: {}", e))?;
                values
            } else {
                parse_json_array(input)
                    .map_err(|e| format!("Failed to parse JSON array: {}", e))?
            };
            if values.is_empty() {
                return Err("No JSON objects found in input".to_string());
            }
            PipelineInput::Json(values)
        }

        InputFormat::Line => {
            let parser = EntryParser::new(EntryMode::SingleLine);
            let entries = parser.parse(input);
            if entries.is_empty() {
                return Err("Input is empty".to_string());
            }
            PipelineInput::Text(entries)
        }

        InputFormat::Block { entry_pattern } => {
            let parser = if let Some(ref pattern) = entry_pattern {
                EntryParser::new(EntryMode::MultiLine)
                    .with_entry_pattern(pattern)?
            } else {
                EntryParser::new(EntryMode::MultiLine)
            };
            let entries = parser.parse(input);
            if entries.is_empty() {
                return Err("Input is empty".to_string());
            }
            PipelineInput::Text(entries)
        }
    };

    // Execute pipeline pre-processing and extract directive + modifiers.
    let (directive, group_by_field, top, label, transformed_input) = if has_pipeline {
        let result = apply_pipeline(&stages, initial_input)?;
        (
            result.algorithm,
            result.group_by_field,
            result.top,
            result.label,
            result.input,
        )
    } else {
        // No pipeline — default summarize.
        let dir = match &options.input_format {
            InputFormat::Json => AlgorithmDirective::Subtree,
            _ => AlgorithmDirective::Patterns,
        };
        (dir, None, None, None, initial_input)
    };

    // Dispatch to the appropriate algorithm.
    let mut output = match transformed_input {
        PipelineInput::Json(values) => run_json_algorithm(
            &directive,
            group_by_field.as_deref(),
            values,
            DEFAULT_THRESHOLD,
            options.depth,
            options.budget,
            input_file,
        )?,
        PipelineInput::Text(entries) => run_text_algorithm(
            &directive,
            entries,
            options.ngram_size,
            options.outlier_threshold,
            options.budget,
        )?,
    };

    // Apply post-processing modifiers.
    if let Some(n) = top {
        apply_top(&mut output, n);
    }
    if let Some(ref field) = label {
        apply_label(&mut output, field);
    }

    Ok(output)
}

/// Run an analysis and return formatted output.
///
/// - `output_format`: `"json"` or `"markdown"`
pub fn process(
    input: &str,
    options: &ProcessOptions,
    output_format: &str,
) -> Result<String, String> {
    let output = process_to_output(input, options, None)?;
    format_output(&output, output_format)
}

fn format_output(output: &crate::output::AnalysisOutput, format: &str) -> Result<String, String> {
    match format {
        "json" => serde_json::to_string_pretty(output)
            .map_err(|e| format!("Failed to serialize output: {}", e)),
        "markdown" | "md" => {
            use crate::formatter::MarkdownFormatter;
            Ok(MarkdownFormatter::format(output))
        }
        _ => Err(format!("Unknown format: {}. Use 'json' or 'markdown'", format)),
    }
}

/// Compute a field-level cost breakdown from an analysis run.
pub fn cost_preview(
    input: &str,
    options: &ProcessOptions,
) -> Result<cost_preview::CostPreviewOutput, String> {
    let analysis = process_to_output(input, options, None)?;
    Ok(cost_preview::cost_preview(&analysis))
}

/// Run the cost-preview pass and return serialized output.
pub fn cost_preview_formatted(
    input: &str,
    options: &ProcessOptions,
    output_format: &str,
) -> Result<String, String> {
    let output = cost_preview(input, options)?;
    match output_format {
        "json" => serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {}", e)),
        "markdown" | "md" => Ok(output.to_markdown()),
        _ => Err(format!(
            "Unknown format: {}. Use 'json' or 'markdown'",
            output_format
        )),
    }
}

/// Run structural discovery and return the raw `DiscoverOutput`.
pub fn discover(input: &str, input_format: InputFormat) -> Result<discover::DiscoverOutput, String> {
    discover::discover(input, input_format)
}

/// Run structural discovery and return serialized output.
pub fn discover_formatted(
    input: &str,
    input_format: InputFormat,
    format: &str,
) -> Result<String, String> {
    let output = discover::discover(input, input_format)?;
    match format {
        "json" => serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {}", e)),
        "markdown" | "md" => Ok(output.to_markdown()),
        _ => Err(format!(
            "Unknown format: {}. Use 'json' or 'markdown'",
            format
        )),
    }
}

/// Parse an input format string into an `InputFormat` enum.
pub fn input_format_from_str(s: &str) -> Result<InputFormat, String> {
    match s {
        "json" => Ok(InputFormat::Json),
        "line" | "text" | "log" => Ok(InputFormat::Line),
        "block" | "multiline" => Ok(InputFormat::Block { entry_pattern: None }),
        _ => Err(format!(
            "Unknown input format: '{}'. Use 'json', 'line', or 'block'",
            s
        )),
    }
}

// ── WASM bindings ─────────────────────────────────────────────────────────────

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn process_text(
    input: &str,
    input_format: &str,
    pipeline_expr: &str,
    ngram_size: usize,
    outlier_threshold: f64,
    depth: usize,
    budget: Option<usize>,
    format: &str,
) -> Result<String, String> {
    let input_fmt = input_format_from_str(input_format)?;
    let options = ProcessOptions {
        input_format: input_fmt,
        pipeline_expr: if pipeline_expr.is_empty() { None } else { Some(pipeline_expr.to_string()) },
        budget,
        ngram_size,
        outlier_threshold,
        depth,
    };
    process(input, &options, format)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn discover_text(
    input: &str,
    input_format: &str,
    format: &str,
) -> Result<String, String> {
    let fmt = input_format_from_str(input_format)?;
    discover_formatted(input, fmt, format)
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn cost_preview_text(
    input: &str,
    input_format: &str,
    pipeline_expr: &str,
    ngram_size: usize,
    outlier_threshold: f64,
    depth: usize,
    format: &str,
) -> Result<String, String> {
    let input_fmt = input_format_from_str(input_format)?;
    let options = ProcessOptions {
        input_format: input_fmt,
        pipeline_expr: if pipeline_expr.is_empty() { None } else { Some(pipeline_expr.to_string()) },
        budget: None,
        ngram_size,
        outlier_threshold,
        depth,
    };
    cost_preview_formatted(input, &options, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_input_format_from_str() {
        assert!(matches!(input_format_from_str("json"), Ok(InputFormat::Json)));
        assert!(matches!(input_format_from_str("line"), Ok(InputFormat::Line)));
        assert!(matches!(
            input_format_from_str("block"),
            Ok(InputFormat::Block { entry_pattern: None })
        ));
        assert!(input_format_from_str("auto").is_err());
        assert!(input_format_from_str("bogus").is_err());
    }

    #[test]
    fn test_process_line_default() {
        let input = "INFO user logged in\nINFO user logged in\nERROR disk full\n";
        let options = ProcessOptions {
            input_format: InputFormat::Line,
            ..Default::default()
        };
        let result = process(input, &options, "json");
        assert!(result.is_ok(), "{:?}", result);
    }

    #[test]
    fn test_process_with_pipeline_del() {
        let input = r#"[{"a":1,"secret":99},{"a":2,"secret":88}]"#;
        let options = ProcessOptions {
            input_format: InputFormat::Json,
            pipeline_expr: Some("del(.secret) | schemas".to_string()),
            ..Default::default()
        };
        let result = process(input, &options, "json").unwrap();
        assert!(!result.contains("secret"));
    }

    #[test]
    fn test_process_with_group_by() {
        let input = r#"[
            {"level":"error","msg":"a"},
            {"level":"warn","msg":"b"},
            {"level":"error","msg":"c"}
        ]"#;
        let options = ProcessOptions {
            input_format: InputFormat::Json,
            pipeline_expr: Some("group_by(.level)".to_string()),
            ..Default::default()
        };
        let result = process(input, &options, "json").unwrap();
        assert!(result.contains("error"));
        assert!(result.contains("warn"));
    }

    #[test]
    fn test_process_with_top() {
        let input = "a a a a a\nb b b b b\nc c c c c\n";
        let options = ProcessOptions {
            input_format: InputFormat::Line,
            pipeline_expr: Some("patterns | top(1)".to_string()),
            ..Default::default()
        };
        let output = process_to_output(input, &options, None).unwrap();
        if let crate::output::AlgorithmResults::Grouped { groups, .. } = &output.results {
            assert!(groups.len() <= 1);
        }
    }
}
