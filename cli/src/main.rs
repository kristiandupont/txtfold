use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use txtfold::clustering::EditDistanceClusterer;
use txtfold::formatter::MarkdownFormatter;
use txtfold::ngram::NgramOutlierDetector;
use txtfold::output::OutputBuilder;
use txtfold::parser::{is_json, is_json_map, parse_json_array, parse_json_map, EntryMode, EntryParser};
use txtfold::schema_clustering::SchemaClusterer;
use txtfold::template::TemplateExtractor;

/// txtfold - Deterministic text compression for log analysis
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file to analyze
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Output format (json or markdown)
    #[arg(short, long, default_value = "markdown")]
    format: OutputFormat,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Input format (text, json-array, json-map, or auto-detect)
    #[arg(short = 'i', long = "input-format", default_value = "auto")]
    input_format: InputFormatArg,

    /// Entry parsing mode (single, multiline, or auto) - for text inputs only
    #[arg(short = 'e', long, default_value = "auto")]
    entry_mode: EntryModeArg,

    /// Algorithm to use (template, clustering, ngram, schema, or auto)
    #[arg(short = 'a', long, default_value = "auto")]
    algorithm: AlgorithmArg,

    /// Similarity threshold (0.0-1.0, for clustering and schema algorithms)
    /// - clustering: how similar entries must be to group
    /// - schema: fraction of fields that must match (1.0 = exact, 0.8 = 80% match)
    #[arg(long, default_value = "0.8")]
    threshold: f64,

    /// N-gram size (for ngram algorithm, word-level)
    #[arg(long, default_value = "2")]
    ngram_size: usize,

    /// Outlier threshold (for ngram algorithm). Use 0 for auto-detection (bottom ~5%)
    #[arg(long, default_value = "0.0")]
    outlier_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            _ => Err(format!("Invalid format: {}. Use 'json' or 'markdown'", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputFormatArg {
    Auto,
    Text,
    JsonArray,
    JsonMap,
}

impl std::str::FromStr for InputFormatArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(InputFormatArg::Auto),
            "text" | "log" | "logs" => Ok(InputFormatArg::Text),
            "json-array" | "json_array" | "jsonarray" | "array" => Ok(InputFormatArg::JsonArray),
            "json-map" | "json_map" | "jsonmap" | "map" => Ok(InputFormatArg::JsonMap),
            _ => Err(format!(
                "Invalid input format: {}. Use 'auto', 'text', 'json-array', or 'json-map'",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryModeArg {
    Single,
    MultiLine,
    Auto,
}

impl std::str::FromStr for EntryModeArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "single" | "single-line" => Ok(EntryModeArg::Single),
            "multi" | "multiline" | "multi-line" => Ok(EntryModeArg::MultiLine),
            "auto" => Ok(EntryModeArg::Auto),
            _ => Err(format!(
                "Invalid entry mode: {}. Use 'single', 'multiline', or 'auto'",
                s
            )),
        }
    }
}

impl From<EntryModeArg> for EntryMode {
    fn from(arg: EntryModeArg) -> Self {
        match arg {
            EntryModeArg::Single => EntryMode::SingleLine,
            EntryModeArg::MultiLine => EntryMode::MultiLine,
            EntryModeArg::Auto => EntryMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlgorithmArg {
    Auto,
    Template,
    Clustering,
    Ngram,
    Schema,
}

impl std::str::FromStr for AlgorithmArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(AlgorithmArg::Auto),
            "template" | "templates" => Ok(AlgorithmArg::Template),
            "cluster" | "clustering" | "edit-distance" => Ok(AlgorithmArg::Clustering),
            "ngram" | "n-gram" | "ngrams" => Ok(AlgorithmArg::Ngram),
            "schema" | "json" => Ok(AlgorithmArg::Schema),
            _ => Err(format!(
                "Invalid algorithm: {}. Use 'auto', 'template', 'clustering', 'ngram', or 'schema'",
                s
            )),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read input file
    let content = fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {:?}", args.input))?;

    // Get filename for metadata
    let filename = args
        .input
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    // Detect input format
    let input_format = match args.input_format {
        InputFormatArg::Auto => {
            if is_json(&content) {
                if is_json_map(&content) {
                    InputFormatArg::JsonMap
                } else {
                    InputFormatArg::JsonArray
                }
            } else {
                InputFormatArg::Text
            }
        }
        other => other,
    };

    // Auto-select algorithm based on input format
    let algorithm = match args.algorithm {
        AlgorithmArg::Auto => {
            match input_format {
                InputFormatArg::JsonArray | InputFormatArg::JsonMap => AlgorithmArg::Schema,
                InputFormatArg::Text => AlgorithmArg::Template,
                InputFormatArg::Auto => unreachable!("Auto resolved above"),
            }
        }
        other => other,
    };

    // Run selected algorithm
    let output = if algorithm == AlgorithmArg::Schema {
        // JSON/Schema path
        let values = match input_format {
            InputFormatArg::JsonMap => {
                let (values, _keys) = parse_json_map(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse JSON map: {}", e))?;
                values
            }
            InputFormatArg::JsonArray => {
                parse_json_array(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse JSON array: {}", e))?
            }
            _ => anyhow::bail!("Schema algorithm requires JSON input"),
        };

        if values.is_empty() {
            anyhow::bail!("No JSON objects found in input");
        }

        let mut clusterer = SchemaClusterer::new(args.threshold);
        clusterer.process(&values);

        let mut builder = OutputBuilder::new(vec![]); // Empty entries for JSON
        if let Some(name) = filename {
            builder = builder.with_input_file(name);
        }
        builder.build_from_schemas(&clusterer, &values)
    } else {
        // Text log path
        let parser = EntryParser::new(args.entry_mode.into());
        let entries = parser.parse(&content);

        if entries.is_empty() {
            eprintln!("Warning: Input file is empty");
            return Ok(());
        }

        match algorithm {
            AlgorithmArg::Template => {
                let mut extractor = TemplateExtractor::new();
                extractor.process(&entries);

                let mut builder = OutputBuilder::new(entries);
                if let Some(name) = filename {
                    builder = builder.with_input_file(name);
                }
                builder.build_from_templates(&extractor)
            }
            AlgorithmArg::Clustering => {
                let mut clusterer = EditDistanceClusterer::new(args.threshold);
                clusterer.process(&entries);

                let mut builder = OutputBuilder::new(entries);
                if let Some(name) = filename {
                    builder = builder.with_input_file(name);
                }
                builder.build_from_clusters(&clusterer)
            }
            AlgorithmArg::Ngram => {
                let mut detector = NgramOutlierDetector::new(args.ngram_size, args.outlier_threshold);
                detector.process(&entries);

                let mut builder = OutputBuilder::new(entries);
                if let Some(name) = filename {
                    builder = builder.with_input_file(name);
                }
                builder.build_from_ngrams(&detector)
            }
            AlgorithmArg::Schema => unreachable!("Schema handled above"),
            AlgorithmArg::Auto => unreachable!("Auto resolved above"),
        }
    };

    // Format output
    let formatted = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&output)
            .context("Failed to serialize output to JSON")?,
        OutputFormat::Markdown => MarkdownFormatter::format(&output),
    };

    // Write output
    if let Some(output_path) = args.output {
        fs::write(&output_path, formatted)
            .with_context(|| format!("Failed to write output to {:?}", output_path))?;
        eprintln!("Output written to {:?}", output_path);
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!(
            "markdown".parse::<OutputFormat>().unwrap(),
            OutputFormat::Markdown
        );
        assert_eq!(
            "md".parse::<OutputFormat>().unwrap(),
            OutputFormat::Markdown
        );
        assert!("invalid".parse::<OutputFormat>().is_err());
    }
}
