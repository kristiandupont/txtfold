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
use txtfold::parser::{is_json, is_json_map, parse_json_array, parse_json_map, EntryMode, EntryParser};
use txtfold::registry::{ALL_ALGORITHMS, ALL_FORMATTERS, ALL_INPUT_FORMATS};
use txtfold::schema_clustering::SchemaClusterer;
use txtfold::subtree::SubtreeFinder;
use txtfold::template::TemplateExtractor;

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

    // --- valid values for --format ---
    let mut format_values: Vec<&'static str> = vec![];
    for fmt in ALL_FORMATTERS {
        format_values.push(fmt.name);
        format_values.extend_from_slice(fmt.aliases);
    }

    // --- valid values for --input-format ---
    let mut input_format_values: Vec<&'static str> = vec!["auto"];
    for fmt in ALL_INPUT_FORMATS {
        input_format_values.push(fmt.name);
        input_format_values.extend_from_slice(fmt.aliases);
    }

    // --- valid values for --entry-mode (from text format sub-options) ---
    let entry_mode_values: Vec<&'static str> = ALL_INPUT_FORMATS
        .iter()
        .find(|f| f.name == "text")
        .and_then(|f| f.sub_options.first())
        .map(|o| o.values.to_vec())
        .unwrap_or_default();

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
            Arg::new("format")
                .short('f')
                .long("format")
                .default_value("markdown")
                .value_parser(PossibleValuesParser::new(format_values))
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
            Arg::new("input-format")
                .short('i')
                .long("input-format")
                .default_value("auto")
                .value_parser(PossibleValuesParser::new(input_format_values))
                .help("Input format (auto-detected if omitted)"),
        )
        .arg(
            Arg::new("entry-mode")
                .short('e')
                .long("entry-mode")
                .default_value("auto")
                .value_parser(PossibleValuesParser::new(entry_mode_values))
                .help("How to split text into entries (text inputs only)"),
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
        );

    for arg in param_args {
        cmd = cmd.arg(arg);
    }

    cmd
}

fn main() -> Result<()> {
    let matches = build_cli().get_matches();

    let format = matches.get_one::<String>("format").unwrap().as_str();
    let algorithm = matches.get_one::<String>("algorithm").unwrap().as_str();
    let input_format_arg = matches.get_one::<String>("input-format").unwrap().as_str();
    let entry_mode_arg = matches.get_one::<String>("entry-mode").unwrap().as_str();
    let budget: Option<usize> = matches.get_one::<usize>("budget").copied();

    // Read input (file or stdin)
    let (content, filename) = if let Some(path_str) = matches.get_one::<String>("input") {
        let input_path = PathBuf::from(path_str);
        let content = fs::read_to_string(&input_path)
            .with_context(|| format!("Failed to read input file: {input_path:?}"))?;
        let filename = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        (content, filename)
    } else {
        let mut content = String::new();
        io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
        (content, None)
    };

    // Resolve input format
    let input_format = if input_format_arg == "auto" {
        if is_json(&content) {
            if is_json_map(&content) { "json-map" } else { "json-array" }
        } else {
            "text"
        }
    } else {
        input_format_arg
    };

    // Resolve algorithm
    let algorithm = if algorithm == "auto" {
        match input_format {
            "json-array" | "json-map" => "schema",
            _ => "template",
        }
    } else {
        algorithm
    };

    // Run
    let output = if algorithm == "schema" {
        let values = match input_format {
            "json-map" => {
                let (values, _keys) = parse_json_map(&content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse JSON map: {e}"))?;
                values
            }
            _ => parse_json_array(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON array: {e}"))?,
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
    } else if algorithm == "subtree" {
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
        let entry_mode = match entry_mode_arg {
            "single" => EntryMode::SingleLine,
            "multiline" => EntryMode::MultiLine,
            _ => EntryMode::Auto,
        };
        let parser = EntryParser::new(entry_mode);
        let entries = parser.parse(&content);

        if entries.is_empty() {
            eprintln!("Warning: input file is empty");
            return Ok(());
        }

        match algorithm {
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
        }
    };

    // Format
    let formatted = match format {
        "json" => serde_json::to_string_pretty(&output).context("Failed to serialize JSON")?,
        _ => MarkdownFormatter::format(&output),
    };

    // Write
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
