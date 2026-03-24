use schemars::schema_for;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use txtfold::output::AnalysisOutput;
use txtfold::registry::{ALL_ALGORITHMS, ALL_FORMATTERS, ALL_INPUT_FORMATS};

#[derive(Serialize)]
struct ConfigSchema {
    version: &'static str,
    algorithms: &'static [txtfold::metadata::AlgorithmMetadata],
    formatters: &'static [txtfold::metadata::FormatterMetadata],
    input_formats: &'static [txtfold::metadata::InputFormatMetadata],
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: dump-schema <config-schema-path> <output-schema-path>");
        std::process::exit(1);
    }
    let config_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);

    // Config schema: algorithms, formatters, input formats
    let config_schema = ConfigSchema {
        version: txtfold::version(),
        algorithms: ALL_ALGORITHMS,
        formatters: ALL_FORMATTERS,
        input_formats: ALL_INPUT_FORMATS,
    };
    let config_json = serde_json::to_string_pretty(&config_schema)
        .expect("config schema serialization is infallible");
    fs::write(&config_path, config_json)
        .unwrap_or_else(|e| panic!("failed to write {config_path:?}: {e}"));

    // Output schema: JSON Schema derived from AnalysisOutput
    let output_schema = schema_for!(AnalysisOutput);
    let output_json = serde_json::to_string_pretty(&output_schema)
        .expect("output schema serialization is infallible");
    fs::write(&output_path, output_json)
        .unwrap_or_else(|e| panic!("failed to write {output_path:?}: {e}"));

    eprintln!("wrote {config_path:?} and {output_path:?}");
}
