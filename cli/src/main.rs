use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use txtfold::entry::Entry;
use txtfold::formatter::MarkdownFormatter;
use txtfold::output::OutputBuilder;
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

fn main() -> Result<()> {
    let args = Args::parse();

    // Read input file
    let content = fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {:?}", args.input))?;

    // Parse lines into entries
    let entries: Vec<Entry> = content
        .lines()
        .enumerate()
        .map(|(idx, line)| Entry::from_line(line.to_string(), idx + 1))
        .collect();

    if entries.is_empty() {
        eprintln!("Warning: Input file is empty");
        return Ok(());
    }

    // Run template extraction
    let mut extractor = TemplateExtractor::new();
    extractor.process(&entries);

    // Build output
    let filename = args
        .input
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    let mut builder = OutputBuilder::new(entries);
    if let Some(name) = filename {
        builder = builder.with_input_file(name);
    }
    let output = builder.build(&extractor);

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
