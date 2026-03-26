//! txtfold: Deterministic pattern summarization for log files and structured data
//!
//! This library provides algorithms for identifying patterns in large text files,
//! extracting templates, and detecting outliers.

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod clustering;
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

/// Run an analysis and return formatted output.
///
/// - `algorithm`: `"auto"`, `"template"`, `"clustering"`, `"ngram"`, `"schema"`, `"subtree"`
/// - `format`: `"json"` or `"markdown"`
pub fn process(
    input: &str,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    format: &str,
) -> Result<String, String> {
    use crate::clustering::EditDistanceClusterer;
    use crate::ngram::NgramOutlierDetector;
    use crate::output::OutputBuilder;
    use crate::parser::{is_json, is_json_map, parse_json_array, parse_json_map, EntryParser, EntryMode};
    use crate::schema_clustering::SchemaClusterer;
    use crate::template::TemplateExtractor;

    let is_json_input = is_json(input);
    let is_map = is_json_input && is_json_map(input);

    let algo = if algorithm == "auto" {
        if is_json_input { "schema" } else { "template" }
    } else {
        algorithm
    };

    let output = if algo == "schema" {
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
        let builder = OutputBuilder::new(vec![]);
        builder.build_from_schemas(&clusterer, &values)
    } else if algo == "subtree" {
        use crate::subtree::SubtreeFinder;
        let root: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let mut finder = SubtreeFinder::new(threshold);
        finder.process(&root);
        let builder = OutputBuilder::new(vec![]);
        builder.build_from_subtree(&finder, &root)
    } else {
        let parser = EntryParser::new(EntryMode::Auto);
        let entries = parser.parse(input);
        if entries.is_empty() {
            return Err("Input is empty".to_string());
        }
        match algo {
            "template" => {
                let mut extractor = TemplateExtractor::new();
                extractor.process(&entries);
                let builder = OutputBuilder::new(entries);
                builder.build_from_templates(&extractor)
            }
            "clustering" => {
                let mut clusterer = EditDistanceClusterer::new(threshold);
                clusterer.process(&entries);
                let builder = OutputBuilder::new(entries);
                builder.build_from_clusters(&clusterer)
            }
            "ngram" => {
                let mut detector = NgramOutlierDetector::new(ngram_size, outlier_threshold);
                detector.process(&entries);
                let builder = OutputBuilder::new(entries);
                builder.build_from_ngrams(&detector)
            }
            _ => return Err(format!("Unknown algorithm: {}", algo)),
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

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn process_text(
    input: &str,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    format: &str,
) -> Result<String, String> {
    process(input, algorithm, threshold, ngram_size, outlier_threshold, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
