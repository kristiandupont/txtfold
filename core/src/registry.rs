//! Registry of all algorithms and formatters
//!
//! This module provides centralized access to metadata for all components
//! in the txtfold library. It doesn't "own" the metadata - it just collects
//! references to metadata defined alongside each component's implementation.

use crate::clustering::EditDistanceClusterer;
use crate::formatter::{JsonFormatter, MarkdownFormatter};
use crate::metadata::{AlgorithmMetadata, FormatterMetadata, InputFormatMetadata};
use crate::ngram::NgramOutlierDetector;
use crate::parser::EntryParser;
use crate::schema_clustering::SchemaClusterer;
use crate::template::TemplateExtractor;

/// All available algorithms in the library
pub const ALL_ALGORITHMS: &[AlgorithmMetadata] = &[
    TemplateExtractor::METADATA,
    EditDistanceClusterer::METADATA,
    NgramOutlierDetector::METADATA,
    SchemaClusterer::METADATA,
];

/// All available output formatters in the library
pub const ALL_FORMATTERS: &[FormatterMetadata] = &[
    MarkdownFormatter::METADATA,
    JsonFormatter::METADATA,
];

/// All available input formats in the library
pub const ALL_INPUT_FORMATS: &[InputFormatMetadata] = &[
    EntryParser::TEXT_FORMAT,
    EntryParser::JSON_ARRAY_FORMAT,
    EntryParser::JSON_MAP_FORMAT,
];

/// Find an algorithm by name or alias
pub fn find_algorithm(name: &str) -> Option<&'static AlgorithmMetadata> {
    let name_lower = name.to_lowercase();
    ALL_ALGORITHMS.iter().find(|algo| {
        algo.name == name_lower
            || algo.aliases.iter().any(|alias| alias.to_lowercase() == name_lower)
    })
}

/// Find a formatter by name or alias
pub fn find_formatter(name: &str) -> Option<&'static FormatterMetadata> {
    let name_lower = name.to_lowercase();
    ALL_FORMATTERS.iter().find(|fmt| {
        fmt.name == name_lower
            || fmt.aliases.iter().any(|alias| alias.to_lowercase() == name_lower)
    })
}

/// Find an input format by name or alias
pub fn find_input_format(name: &str) -> Option<&'static InputFormatMetadata> {
    let name_lower = name.to_lowercase();
    ALL_INPUT_FORMATS.iter().find(|fmt| {
        fmt.name == name_lower
            || fmt.aliases.iter().any(|alias| alias.to_lowercase() == name_lower)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_algorithms_registered() {
        assert_eq!(ALL_ALGORITHMS.len(), 4);

        // Check names
        let names: Vec<&str> = ALL_ALGORITHMS.iter().map(|a| a.name).collect();
        assert!(names.contains(&"template"));
        assert!(names.contains(&"clustering"));
        assert!(names.contains(&"ngram"));
        assert!(names.contains(&"schema"));
    }

    #[test]
    fn test_all_formatters_registered() {
        assert_eq!(ALL_FORMATTERS.len(), 2);

        let names: Vec<&str> = ALL_FORMATTERS.iter().map(|f| f.name).collect();
        assert!(names.contains(&"markdown"));
        assert!(names.contains(&"json"));
    }

    #[test]
    fn test_find_algorithm_by_name() {
        assert!(find_algorithm("template").is_some());
        assert!(find_algorithm("clustering").is_some());
        assert!(find_algorithm("ngram").is_some());
        assert!(find_algorithm("schema").is_some());
        assert!(find_algorithm("nonexistent").is_none());
    }

    #[test]
    fn test_find_algorithm_by_alias() {
        assert!(find_algorithm("templates").is_some());
        assert!(find_algorithm("cluster").is_some());
        assert!(find_algorithm("edit-distance").is_some());
        assert!(find_algorithm("n-gram").is_some());
        assert!(find_algorithm("json").is_some());
    }

    #[test]
    fn test_find_formatter_by_name() {
        assert!(find_formatter("markdown").is_some());
        assert!(find_formatter("json").is_some());
        assert!(find_formatter("nonexistent").is_none());
    }

    #[test]
    fn test_find_formatter_by_alias() {
        assert!(find_formatter("md").is_some());
    }

    #[test]
    fn test_algorithm_metadata_complete() {
        for algo in ALL_ALGORITHMS {
            // All should have non-empty names and descriptions
            assert!(!algo.name.is_empty());
            assert!(!algo.description.is_empty());
            assert!(!algo.best_for.is_empty());
            assert!(!algo.input_types.is_empty());

            // Parameters should have valid metadata
            for param in algo.parameters {
                assert!(!param.name.is_empty());
                assert!(!param.description.is_empty());
            }
        }
    }

    #[test]
    fn test_formatter_metadata_complete() {
        for fmt in ALL_FORMATTERS {
            assert!(!fmt.name.is_empty());
            assert!(!fmt.description.is_empty());
            assert!(!fmt.mime_type.is_empty());
            assert!(!fmt.file_extension.is_empty());
        }
    }

    #[test]
    fn test_all_input_formats_registered() {
        assert_eq!(ALL_INPUT_FORMATS.len(), 3);

        let names: Vec<&str> = ALL_INPUT_FORMATS.iter().map(|f| f.name).collect();
        assert!(names.contains(&"text"));
        assert!(names.contains(&"json-array"));
        assert!(names.contains(&"json-map"));
    }

    #[test]
    fn test_find_input_format_by_name() {
        assert!(find_input_format("text").is_some());
        assert!(find_input_format("json-array").is_some());
        assert!(find_input_format("json-map").is_some());
        assert!(find_input_format("nonexistent").is_none());
    }

    #[test]
    fn test_find_input_format_by_alias() {
        assert!(find_input_format("log").is_some());
        assert!(find_input_format("logs").is_some());
        assert!(find_input_format("array").is_some());
        assert!(find_input_format("map").is_some());
    }

    #[test]
    fn test_input_format_metadata_complete() {
        for fmt in ALL_INPUT_FORMATS {
            assert!(!fmt.name.is_empty());
            assert!(!fmt.description.is_empty());
        }
    }
}
