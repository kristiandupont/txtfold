use schemars::schema::{Schema, SchemaObject};
use schemars::schema_for;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use txtfold::cost_preview::CostPreviewOutput;
use txtfold::discover::DiscoverOutput;
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

    // Output schema: combined JSON Schema for all public output types.
    //
    // Each type is generated independently (so its own definitions are complete),
    // then merged into a single `definitions` map.  Root types are placed in
    // `definitions` alongside their dependencies; the `roots` array names which
    // definitions are top-level output types vs. supporting sub-types.
    let roots = ["AnalysisOutput", "DiscoverOutput", "CostPreviewOutput"];

    let analysis     = schema_for!(AnalysisOutput);
    let discover     = schema_for!(DiscoverOutput);
    let cost_preview = schema_for!(CostPreviewOutput);

    // Merge all supporting-type definitions from every root schema.
    let mut defs: BTreeMap<String, Schema> = BTreeMap::new();
    for (k, v) in analysis.definitions.iter()
        .chain(discover.definitions.iter())
        .chain(cost_preview.definitions.iter())
    {
        defs.entry(k.clone()).or_insert_with(|| v.clone());
    }

    // Insert the three root types themselves.
    let insert_root = |defs: &mut BTreeMap<String, Schema>, name: &str, schema: SchemaObject| {
        defs.entry(name.to_string()).or_insert(Schema::Object(schema));
    };
    insert_root(&mut defs, "AnalysisOutput",    analysis.schema);
    insert_root(&mut defs, "DiscoverOutput",    discover.schema);
    insert_root(&mut defs, "CostPreviewOutput", cost_preview.schema);

    let combined = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "roots": roots,
        "definitions": defs,
    });

    let output_json = serde_json::to_string_pretty(&combined)
        .expect("output schema serialization is infallible");
    fs::write(&output_path, output_json)
        .unwrap_or_else(|e| panic!("failed to write {output_path:?}: {e}"));

    eprintln!("wrote {config_path:?} and {output_path:?}");
}
