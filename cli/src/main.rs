use anyhow::{Context, Result};
use clap::builder::PossibleValuesParser;
use clap::{Arg, Command};
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use txtfold::registry::{ALL_FORMATTERS, ALL_INPUT_FORMATS};
use txtfold::{InputFormat, ProcessOptions};

fn build_cli() -> Command {
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
        input_format_values.extend_from_slice(fmt.aliases);
    }

    Command::new("txtfold")
        .about("Identify patterns and outliers in large log files and structured data")
        .version(txtfold::version())
        // Positional arg 1: optional PIPELINE expression.
        // Positional arg 2: optional FILE path.
        // Clap can't disambiguate these at the type level, so we accept two
        // optional positional strings and resolve them ourselves in main().
        .arg(
            Arg::new("pos1")
                .value_name("PIPELINE_OR_FILE")
                .required(false)
                .index(1)
                .help("Pipeline expression or input file (see disambiguation below)"),
        )
        .arg(
            Arg::new("pos2")
                .value_name("FILE")
                .required(false)
                .index(2)
                .help("Input file (when pos1 is a pipeline expression)"),
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
        .arg(
            Arg::new("budget")
                .short('b')
                .long("budget")
                .value_name("LINES")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum output lines. The most important groups are shown first; \
                       output is trimmed when the budget is reached."),
        )
        .arg(
            Arg::new("ngram-size")
                .long("ngram-size")
                .default_value("2")
                .value_parser(clap::value_parser!(usize))
                .help("N-gram size for the 'outliers' algorithm"),
        )
        .arg(
            Arg::new("outlier-threshold")
                .long("outlier-threshold")
                .default_value("0.0")
                .value_parser(clap::value_parser!(f64))
                .help("Outlier score threshold for the 'outliers' algorithm (0.0 = auto-detect)"),
        )
        .arg(
            Arg::new("depth")
                .long("depth")
                .default_value("1")
                .value_parser(clap::value_parser!(usize))
                .help("Nesting depth for the 'subtree' algorithm"),
        )
        .arg(
            Arg::new("discover")
                .long("discover")
                .action(clap::ArgAction::SetTrue)
                .help("Run structural discovery instead of pattern analysis. \
                       Outputs a compact schema map showing field paths, types, \
                       cardinality, and sample values."),
        )
        .arg(
            Arg::new("cost-preview")
                .long("cost-preview")
                .action(clap::ArgAction::SetTrue)
                .help("Run full analysis then emit a field-level token breakdown. \
                       Shows where the output budget is going and suggests \
                       del(...) candidates for noisy fields."),
        )
        .arg(
            Arg::new("syntax")
                .long("syntax")
                .action(clap::ArgAction::SetTrue)
                .help("Print pipeline syntax reference and exit."),
        )
        .after_help(
            "PIPELINE EXPRESSIONS\n\
             \n\
             The optional PIPELINE argument selects the algorithm and pre-processes input.\n\
             Examples:\n\
             \n\
             \x20 txtfold 'outliers' app.log\n\
             \x20 txtfold 'similar(0.8) | top(20)' --format line app.log\n\
             \x20 txtfold '.diagnostics[] | del(.sourceCode) | group_by(.category)' biome.json\n\
             \n\
             Terminal verbs: summarize (default), similar(t), patterns, outliers, schemas, subtree, group_by(.f)\n\
             Modifiers: del(.f, ...), top(N), label(.f)\n\
             Path selection: .field[], .field[*], .field[N], .a.b[]\n\
             \n\
             DISAMBIGUATION (one positional argument)\n\
             \n\
             If the argument is a readable file → treated as FILE.\n\
             If it starts with '.' or contains '|' or is a known verb → treated as PIPELINE.\n",
        )
}

/// Decide whether a single positional argument is a pipeline expression or a file path.
fn is_pipeline_expr(s: &str) -> bool {
    // A readable file always wins.
    if PathBuf::from(s).is_file() {
        return false;
    }
    // Path expression, pipe, or known verb name.
    s.starts_with('.')
        || s.contains('|')
        || txtfold::pipeline::is_verb_name(s)
        || s.starts_with("similar(")
        || s.starts_with("top(")
        || s.starts_with("del(")
        || s.starts_with("group_by(")
        || s.starts_with("label(")
}

fn main() -> Result<()> {
    let matches = build_cli().get_matches();

    // ── --syntax ──────────────────────────────────────────────────────────────
    if matches.get_flag("syntax") {
        print!("{}", txtfold::discover::HINTS_TEXT);
        println!();
        return Ok(());
    }

    let output_format = matches.get_one::<String>("output-format").unwrap().as_str();
    let budget: Option<usize> = matches.get_one::<usize>("budget").copied();
    let ngram_size = *matches.get_one::<usize>("ngram-size").unwrap();
    let outlier_threshold = *matches.get_one::<f64>("outlier-threshold").unwrap();
    let depth = *matches.get_one::<usize>("depth").unwrap();

    // ── Resolve positional args ───────────────────────────────────────────────
    let pos1 = matches.get_one::<String>("pos1").map(|s| s.as_str());
    let pos2 = matches.get_one::<String>("pos2").map(|s| s.as_str());

    let (pipeline_expr, file_arg): (Option<&str>, Option<&str>) = match (pos1, pos2) {
        (None, _) => (None, None),
        (Some(a), None) => {
            if is_pipeline_expr(a) {
                (Some(a), None)
            } else {
                (None, Some(a))
            }
        }
        (Some(a), Some(b)) => (Some(a), Some(b)),
    };

    // ── Read input ────────────────────────────────────────────────────────────
    // If there is no file argument and stdin is a TTY (interactive), the user
    // ran txtfold with no input — show help rather than hanging waiting for
    // stdin that will never come.
    if file_arg.is_none() && io::stdin().is_terminal() {
        build_cli().print_help()?;
        println!();
        return Ok(());
    }

    let (content, filename, extension) = if let Some(path_str) = file_arg {
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

    // ── Resolve input format ──────────────────────────────────────────────────
    let format_flag = matches
        .get_one::<String>("format")
        .or_else(|| matches.get_one::<String>("input-format"))
        .map(|s| s.as_str());

    let entry_pattern = matches.get_one::<String>("entry-pattern").cloned();

    let input_format: InputFormat = if let Some(flag) = format_flag {
        match flag {
            "json" | "json-array" | "json-map" => InputFormat::Json,
            "line" | "log" | "logs" | "text" => InputFormat::Line,
            "block" | "multiline" | "multi-line" => {
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
        anyhow::bail!("Cannot infer format from stdin; pass --format json|line|block");
    };

    // ── --discover ────────────────────────────────────────────────────────────
    if matches.get_flag("discover") {
        let output = txtfold::discover(&content, input_format)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let formatted = match output_format {
            "json" => serde_json::to_string_pretty(&output)
                .context("Failed to serialize JSON")?,
            _ => output.to_markdown(),
        };

        return write_output(formatted, matches.get_one::<String>("output"));
    }

    // ── --cost-preview ────────────────────────────────────────────────────────
    if matches.get_flag("cost-preview") {
        let options = ProcessOptions {
            input_format,
            pipeline_expr: pipeline_expr.map(|s| s.to_string()),
            budget: None, // cost preview always runs unbounded
            ngram_size,
            outlier_threshold,
            depth,
        };

        let output = txtfold::cost_preview(&content, &options)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let formatted = match output_format {
            "json" => serde_json::to_string_pretty(&output)
                .context("Failed to serialize JSON")?,
            _ => output.to_markdown(),
        };

        return write_output(formatted, matches.get_one::<String>("output"));
    }

    // ── Normal analysis ───────────────────────────────────────────────────────
    let options = ProcessOptions {
        input_format,
        pipeline_expr: pipeline_expr.map(|s| s.to_string()),
        budget,
        ngram_size,
        outlier_threshold,
        depth,
    };

    let output = txtfold::process_to_output(&content, &options, filename)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let formatted = match output_format {
        "json" => serde_json::to_string_pretty(&output).context("Failed to serialize JSON")?,
        _ => txtfold::formatter::MarkdownFormatter::format(&output),
    };

    write_output(formatted, matches.get_one::<String>("output"))
}

fn write_output(formatted: String, output_path: Option<&String>) -> Result<()> {
    if let Some(path_str) = output_path {
        let path = PathBuf::from(path_str);
        fs::write(&path, formatted)
            .with_context(|| format!("Failed to write output to {path:?}"))?;
        eprintln!("Output written to {path:?}");
    } else {
        println!("{formatted}");
    }
    Ok(())
}

