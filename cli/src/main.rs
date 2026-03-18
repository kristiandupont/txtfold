use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use txtfold::clustering::EditDistanceClusterer;
use txtfold::formatter::MarkdownFormatter;
use txtfold::ngram::NgramOutlierDetector;
use txtfold::output::OutputBuilder;
use txtfold::parser::{EntryMode, EntryParser};
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

    /// Entry parsing mode (single, multiline, or auto)
    #[arg(short = 'e', long, default_value = "auto")]
    entry_mode: EntryModeArg,

    /// Algorithm to use (template, clustering, or ngram)
    #[arg(short = 'a', long, default_value = "template")]
    algorithm: AlgorithmArg,

    /// Clustering threshold (0.0-1.0, only for clustering algorithm)
    #[arg(long, default_value = "0.2")]
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
    Template,
    Clustering,
    Ngram,
}

impl std::str::FromStr for AlgorithmArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "template" | "templates" => Ok(AlgorithmArg::Template),
            "cluster" | "clustering" | "edit-distance" => Ok(AlgorithmArg::Clustering),
            "ngram" | "n-gram" | "ngrams" => Ok(AlgorithmArg::Ngram),
            _ => Err(format!(
                "Invalid algorithm: {}. Use 'template', 'clustering', or 'ngram'",
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

    // Parse entries using the specified mode
    let parser = EntryParser::new(args.entry_mode.into());
    let entries = parser.parse(&content);

    if entries.is_empty() {
        eprintln!("Warning: Input file is empty");
        return Ok(());
    }

    // Get filename for metadata
    let filename = args
        .input
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    // Run selected algorithm
    let output = match args.algorithm {
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
