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

/// Run an analysis on text-format entries using the specified algorithm.
fn run_text_algorithm(
    algo: &str,
    entries: Vec<crate::entry::Entry>,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    budget: Option<usize>,
) -> Result<crate::output::AnalysisOutput, String> {
    use crate::clustering::EditDistanceClusterer;
    use crate::ngram::NgramOutlierDetector;
    use crate::output::OutputBuilder;
    use crate::template::TemplateExtractor;

    Ok(match algo {
        "auto" | "template" => {
            let mut extractor = TemplateExtractor::new();
            extractor.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_templates(&extractor)
        }
        "clustering" => {
            let mut clusterer = EditDistanceClusterer::new(threshold);
            clusterer.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_clusters(&clusterer)
        }
        "ngram" => {
            let mut detector = NgramOutlierDetector::new(ngram_size, outlier_threshold);
            detector.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_ngrams(&detector)
        }
        _ => return Err(format!("Unknown algorithm: {}", algo)),
    })
}

/// Run an analysis and return formatted output.
///
/// - `input_format`: Explicit format declaration (`InputFormat::Json`, `Line`, or `Block`).
/// - `algorithm`: `"auto"`, `"template"`, `"clustering"`, `"ngram"`, `"schema"`, `"subtree"`
/// - `budget`: maximum output lines (`None` = unlimited)
/// - `format`: `"json"` or `"markdown"`
pub fn process(
    input: &str,
    input_format: InputFormat,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    budget: Option<usize>,
    format: &str,
) -> Result<String, String> {
    use crate::output::OutputBuilder;
    use crate::parser::{is_json_map, parse_json_array, parse_json_map, EntryMode, EntryParser};
    use crate::schema_clustering::SchemaClusterer;

    let output = match input_format {
        InputFormat::Json => {
            // Internal heuristic to distinguish array vs map — not exposed to callers.
            let is_map = is_json_map(input);

            if algorithm == "subtree" {
                use crate::subtree::SubtreeFinder;
                let root: serde_json::Value = serde_json::from_str(input)
                    .map_err(|e| format!("Failed to parse JSON: {}", e))?;
                let mut finder = SubtreeFinder::new(threshold);
                finder.process(&root);
                let mut builder = OutputBuilder::new(vec![]);
                if let Some(b) = budget { builder = builder.with_budget(b); }
                builder.build_from_subtree(&finder, &root)
            } else {
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
                let mut clusterer = SchemaClusterer::new(threshold, 1);
                clusterer.process(&values);
                let mut builder = OutputBuilder::new(vec![]);
                if let Some(b) = budget { builder = builder.with_budget(b); }
                builder.build_from_schemas(&clusterer, &values)
            }
        }

        InputFormat::Line => {
            let parser = EntryParser::new(EntryMode::SingleLine);
            let entries = parser.parse(input);
            if entries.is_empty() {
                return Err("Input is empty".to_string());
            }
            let algo = if algorithm == "auto" { "template" } else { algorithm };
            run_text_algorithm(algo, entries, threshold, ngram_size, outlier_threshold, budget)?
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
            let algo = if algorithm == "auto" { "template" } else { algorithm };
            run_text_algorithm(algo, entries, threshold, ngram_size, outlier_threshold, budget)?
        }
    };

    match format {
        "json" => serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {}", e)),
        "markdown" | "md" => {
            use crate::formatter::MarkdownFormatter;
            Ok(MarkdownFormatter::format(&output))
        }
        _ => Err(format!("Unknown format: {}. Use 'json' or 'markdown'", format)),
    }
}

/// Compute a field-level cost breakdown from an analysis run.
///
/// Runs the full analysis pipeline on `input` (same arguments as `process()`)
/// and returns a `CostPreviewOutput` showing which fields consume the most
/// tokens, together with a suggested `del(...)` expression for noisy fields.
pub fn cost_preview(
    input: &str,
    input_format: InputFormat,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
) -> Result<cost_preview::CostPreviewOutput, String> {
    use crate::output::OutputBuilder;
    use crate::parser::{is_json_map, parse_json_array, parse_json_map, EntryMode, EntryParser};
    use crate::schema_clustering::SchemaClusterer;

    let analysis = match input_format {
        InputFormat::Json => {
            let is_map = is_json_map(input);

            if algorithm == "subtree" {
                use crate::subtree::SubtreeFinder;
                let root: serde_json::Value = serde_json::from_str(input)
                    .map_err(|e| format!("Failed to parse JSON: {}", e))?;
                let mut finder = SubtreeFinder::new(threshold);
                finder.process(&root);
                OutputBuilder::new(vec![]).build_from_subtree(&finder, &root)
            } else {
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
                let mut clusterer = SchemaClusterer::new(threshold, 1);
                clusterer.process(&values);
                OutputBuilder::new(vec![]).build_from_schemas(&clusterer, &values)
            }
        }

        InputFormat::Line => {
            let parser = EntryParser::new(EntryMode::SingleLine);
            let entries = parser.parse(input);
            if entries.is_empty() {
                return Err("Input is empty".to_string());
            }
            let algo = if algorithm == "auto" { "template" } else { algorithm };
            run_text_algorithm(algo, entries, threshold, ngram_size, outlier_threshold, None)?
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
            let algo = if algorithm == "auto" { "template" } else { algorithm };
            run_text_algorithm(algo, entries, threshold, ngram_size, outlier_threshold, None)?
        }
    };

    Ok(cost_preview::cost_preview(&analysis))
}

/// Run the cost-preview pass and return serialized output.
///
/// - `format`: `"json"` or `"markdown"`
pub fn cost_preview_formatted(
    input: &str,
    input_format: InputFormat,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    format: &str,
) -> Result<String, String> {
    let output = cost_preview(input, input_format, algorithm, threshold, ngram_size, outlier_threshold)?;
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

/// Run structural discovery and return the raw `DiscoverOutput`.
pub fn discover(input: &str, input_format: InputFormat) -> Result<discover::DiscoverOutput, String> {
    discover::discover(input, input_format)
}

/// Run structural discovery and return serialized output.
///
/// - `format`: `"json"` or `"markdown"`
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
///
/// Valid values: `"json"`, `"line"`, `"block"`.
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

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn process_text(
    input: &str,
    input_format: &str,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    budget: Option<usize>,
    format: &str,
) -> Result<String, String> {
    let fmt = input_format_from_str(input_format)?;
    process(input, fmt, algorithm, threshold, ngram_size, outlier_threshold, budget, format)
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
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    format: &str,
) -> Result<String, String> {
    let fmt = input_format_from_str(input_format)?;
    cost_preview_formatted(input, fmt, algorithm, threshold, ngram_size, outlier_threshold, format)
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
}
