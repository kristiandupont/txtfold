use anyhow::{Context, Result};
use clap::builder::PossibleValuesParser;
use clap::{value_parser, Arg, Command};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use txtfold::clustering::EditDistanceClusterer;
use txtfold::formatter::MarkdownFormatter;
use txtfold::metadata::{ParamDefault, ParamType};
use txtfold::ngram::NgramOutlierDetector;
use txtfold::output::OutputBuilder;
use txtfold::parser::{EntryMode, EntryParser};
use txtfold::registry::{ALL_ALGORITHMS, ALL_FORMATTERS, ALL_INPUT_FORMATS};
use txtfold::schema_clustering::SchemaClusterer;
use txtfold::subtree::SubtreeFinder;
use txtfold::template::TemplateExtractor;
use txtfold::InputFormat;

/// Leak a String to produce a `&'static str`.
///
/// Used only for the small number of default-value strings we format at startup.
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn build_cli() -> Command {
    // --- valid values for --algorithm ---
    let mut algo_values: Vec<&'static str> = vec!["auto"];
    let mut algo_help = String::from("Algorithm to use [auto");
    for algo in ALL_ALGORITHMS {
        algo_values.push(algo.name);
        algo_values.extend_from_slice(algo.aliases);
        algo_help.push_str(&format!("|{}", algo.name));
    }
    algo_help.push(']');
    for algo in ALL_ALGORITHMS {
        algo_help.push_str(&format!("\n  {}: {}", algo.name, algo.best_for));
    }

    // --- valid values for --output-format ---
    let mut output_format_values: Vec<&'static str> = vec![];
    for fmt in ALL_FORMATTERS {
        output_format_values.push(fmt.name);
        output_format_values.extend_from_slice(fmt.aliases);
    }

    // --- valid values for --format (input format families) ---
    let mut input_format_values: Vec<&'static str> = vec![];
    for fmt in ALL_INPUT_FORMATS {
        input_format_values.push(fmt.name);
        // Include aliases as valid values too
        input_format_values.extend_from_slice(fmt.aliases);
    }

    // --- one Arg per unique algorithm parameter ---
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
    let mut param_args: Vec<Arg> = vec![];
    for algo in ALL_ALGORITHMS {
        for param in algo.parameters {
            if !seen.insert(param.name) {
                continue;
            }
            let long_flag: &'static str = leak_str(param.name.replace('_', "-"));
            let default_str: &'static str = match param.default {
                ParamDefault::Float(v) => leak_str(format!("{v}")),
                ParamDefault::USize(v) => leak_str(format!("{v}")),
                ParamDefault::Bool(v) => leak_str(format!("{v}")),
                ParamDefault::Str(v) => v,
            };
            let arg = Arg::new(param.name)
                .long(long_flag)
                .default_value(default_str)
                .help(param.description);
            let arg = match param.type_info {
                ParamType::Float => arg.value_parser(value_parser!(f64)),
                ParamType::USize => arg.value_parser(value_parser!(usize)),
                ParamType::Bool => arg.value_parser(value_parser!(bool)),
                ParamType::String | ParamType::Enum(_) => arg.value_parser(value_parser!(String)),
            };
            param_args.push(arg);
        }
    }

    let mut cmd = Command::new("txtfold")
        .about("Identify patterns and outliers in large log files and structured data")
        .version(txtfold::version())
        .arg(
            Arg::new("input")
                .value_name("FILE")
                .required(false)
                .index(1)
                .help("Input file to analyze (reads from stdin if omitted)"),
        )
        .arg(
            Arg::new("output-format")
                .short('f')
                .long("output-format")
                .default_value("markdown")
                .value_parser(PossibleValuesParser::new(output_format_values))
                .help("Output format"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file (default: stdout)"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(PossibleValuesParser::new(input_format_values.clone()))
                .help("Input format: json | line | block. \
                       Inferred from file extension when omitted (.json → json, other → line). \
                       Required when reading from stdin with no file extension."),
        )
        // Hidden backwards-compat alias for --format
        .arg(
            Arg::new("input-format")
                .long("input-format")
                .value_parser(PossibleValuesParser::new(input_format_values))
                .hide(true),
        )
        .arg(
            Arg::new("entry-pattern")
                .long("entry-pattern")
                .value_name("REGEX")
                .help("Regex that marks the start of a new entry (block format only)"),
        )
        // Hidden backwards-compat: --entry-mode (single|multiline|auto)
        .arg(
            Arg::new("entry-mode")
                .short('e')
                .long("entry-mode")
                .value_parser(["auto", "single", "multiline"])
                .hide(true),
        )
        .arg(
            Arg::new("algorithm")
                .short('a')
                .long("algorithm")
                .default_value("auto")
                .value_parser(PossibleValuesParser::new(algo_values))
                .help(leak_str(algo_help)),
        )
        .arg(
            Arg::new("budget")
                .short('b')
                .long("budget")
                .value_name("LINES")
                .value_parser(value_parser!(usize))
                .help("Maximum output lines. The most important groups are shown first; output is trimmed when the budget is reached."),
        )
        ;

    for arg in param_args {
        cmd = cmd.arg(arg);
    }

    cmd
}

fn main() -> Result<()> {
    let matches = build_cli().get_matches();

    let output_format = matches.get_one::<String>("output-format").unwrap().as_str();
    let algorithm = matches.get_one::<String>("algorithm").unwrap().as_str();
    let budget: Option<usize> = matches.get_one::<usize>("budget").copied();

    // Read input (file or stdin)
    let (content, filename, extension) =
        if let Some(path_str) = matches.get_one::<String>("input") {
            let input_path = PathBuf::from(path_str);
            let content = fs::read_to_string(&input_path)
                .with_context(|| format!("Failed to read input file: {input_path:?}"))?;
            let filename = input_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            let ext = input_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());
            (content, filename, ext)
        } else {
            let mut content = String::new();
            io::stdin()
                .read_to_string(&mut content)
                .context("Failed to read from stdin")?;
            (content, None, None)
        };

    // Resolve input format:
    // 1. Explicit --format flag takes priority.
    // 2. Deprecated --input-format flag (hidden alias).
    // 3. File extension inference (.json → json, else → line).
    // 4. Stdin with no flag and no file extension → error.
    let format_flag = matches
        .get_one::<String>("format")
        .or_else(|| matches.get_one::<String>("input-format"))
        .map(|s| s.as_str());

    let input_format: InputFormat = if let Some(flag) = format_flag {
        match flag {
            "json" | "json-array" | "json-map" => InputFormat::Json,
            "line" | "log" | "logs" | "text" => InputFormat::Line,
            "block" | "multiline" | "multi-line" => {
                let entry_pattern = matches
                    .get_one::<String>("entry-pattern")
                    .map(|s| s.clone());
                InputFormat::Block { entry_pattern }
            }
            other => anyhow::bail!(
                "Unknown input format '{}'. Use json, line, or block.",
                other
            ),
        }
    } else if let Some(ref ext) = extension {
        match ext.as_str() {
            "json" => InputFormat::Json,
            _ => InputFormat::Line,
        }
    } else {
        // Stdin with no --format flag and no file extension
        anyhow::bail!(
            "Cannot infer format from stdin; pass --format json|line|block"
        );
    };

    // Backwards-compat: honour --entry-mode when --format block is not in use.
    // If the user passed --entry-mode multiline without an explicit --format,
    // we switch the already-inferred format to Block.
    let input_format = if let Some(entry_mode_arg) =
        matches.get_one::<String>("entry-mode").map(|s| s.as_str())
    {
        match (entry_mode_arg, input_format) {
            ("multiline", InputFormat::Line) => {
                let entry_pattern =
                    matches.get_one::<String>("entry-pattern").map(|s| s.clone());
                InputFormat::Block { entry_pattern }
            }
            ("single", _) => InputFormat::Line,
            (_, other) => other,
        }
    } else {
        input_format
    };

    // Resolve algorithm
    let algorithm = if algorithm == "auto" {
        match &input_format {
            InputFormat::Json => "schema",
            _ => "template",
        }
    } else {
        algorithm
    };

    // Run
    let output = match &input_format {
        InputFormat::Json => {
            use txtfold::parser::{is_json_map, parse_json_array, parse_json_map};
            let is_map = is_json_map(&content);

            if algorithm == "subtree" {
                let root: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {e}"))?;

                let threshold = *matches.get_one::<f64>("threshold").unwrap();
                let mut finder = SubtreeFinder::new(threshold);
                finder.process(&root);

                let mut builder = OutputBuilder::new(vec![]);
                if let Some(name) = filename { builder = builder.with_input_file(name); }
                if let Some(b) = budget { builder = builder.with_budget(b); }
                builder.build_from_subtree(&finder, &root)
            } else {
                let values = if is_map {
                    let (values, _keys) = parse_json_map(&content)
                        .map_err(|e| anyhow::anyhow!("Failed to parse JSON map: {e}"))?;
                    values
                } else {
                    parse_json_array(&content)
                        .map_err(|e| anyhow::anyhow!("Failed to parse JSON array: {e}"))?
                };

                if values.is_empty() {
                    anyhow::bail!("No JSON objects found in input");
                }

                let threshold = *matches.get_one::<f64>("threshold").unwrap();
                let depth = *matches.get_one::<usize>("depth").unwrap();
                let mut clusterer = SchemaClusterer::new(threshold, depth);
                clusterer.process(&values);

                let mut builder = OutputBuilder::new(vec![]);
                if let Some(name) = filename { builder = builder.with_input_file(name); }
                if let Some(b) = budget { builder = builder.with_budget(b); }
                builder.build_from_schemas(&clusterer, &values)
            }
        }

        InputFormat::Line => {
            let parser = EntryParser::new(EntryMode::SingleLine);
            let entries = parser.parse(&content);

            if entries.is_empty() {
                eprintln!("Warning: input file is empty");
                return Ok(());
            }

            run_text_algorithm(algorithm, &matches, entries, filename, budget)?
        }

        InputFormat::Block { entry_pattern } => {
            let parser = if let Some(pattern) = entry_pattern {
                EntryParser::new(EntryMode::MultiLine)
                    .with_entry_pattern(pattern)
                    .map_err(|e| anyhow::anyhow!("{e}"))?
            } else {
                EntryParser::new(EntryMode::MultiLine)
            };
            let entries = parser.parse(&content);

            if entries.is_empty() {
                eprintln!("Warning: input file is empty");
                return Ok(());
            }

            run_text_algorithm(algorithm, &matches, entries, filename, budget)?
        }
    };

    // Format output
    let formatted = match output_format {
        "json" => serde_json::to_string_pretty(&output).context("Failed to serialize JSON")?,
        _ => MarkdownFormatter::format(&output),
    };

    // Write output
    if let Some(output_path) = matches.get_one::<String>("output") {
        let path = PathBuf::from(output_path);
        fs::write(&path, formatted)
            .with_context(|| format!("Failed to write output to {path:?}"))?;
        eprintln!("Output written to {path:?}");
    } else {
        println!("{formatted}");
    }

    Ok(())
}

fn run_text_algorithm(
    algorithm: &str,
    matches: &clap::ArgMatches,
    entries: Vec<txtfold::entry::Entry>,
    filename: Option<String>,
    budget: Option<usize>,
) -> Result<txtfold::output::AnalysisOutput> {
    Ok(match algorithm {
        "template" => {
            let mut extractor = TemplateExtractor::new();
            extractor.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(name) = filename { builder = builder.with_input_file(name); }
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_templates(&extractor)
        }
        "clustering" => {
            let threshold = *matches.get_one::<f64>("threshold").unwrap();
            let mut clusterer = EditDistanceClusterer::new(threshold);
            clusterer.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(name) = filename { builder = builder.with_input_file(name); }
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_clusters(&clusterer)
        }
        "ngram" => {
            let ngram_size = *matches.get_one::<usize>("ngram_size").unwrap();
            let outlier_threshold = *matches.get_one::<f64>("outlier_threshold").unwrap();
            let mut detector = NgramOutlierDetector::new(ngram_size, outlier_threshold);
            detector.process(&entries);
            let mut builder = OutputBuilder::new(entries);
            if let Some(name) = filename { builder = builder.with_input_file(name); }
            if let Some(b) = budget { builder = builder.with_budget(b); }
            builder.build_from_ngrams(&detector)
        }
        other => anyhow::bail!("Unknown algorithm: {other}"),
    })
}
