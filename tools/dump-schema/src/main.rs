use serde::Serialize;
use txtfold::registry::{ALL_ALGORITHMS, ALL_FORMATTERS, ALL_INPUT_FORMATS};

#[derive(Serialize)]
struct Schema {
    version: &'static str,
    algorithms: &'static [txtfold::metadata::AlgorithmMetadata],
    formatters: &'static [txtfold::metadata::FormatterMetadata],
    input_formats: &'static [txtfold::metadata::InputFormatMetadata],
}

fn main() {
    let schema = Schema {
        version: txtfold::version(),
        algorithms: ALL_ALGORITHMS,
        formatters: ALL_FORMATTERS,
        input_formats: ALL_INPUT_FORMATS,
    };

    println!("{}", serde_json::to_string_pretty(&schema).expect("schema serialization is infallible"));
}
