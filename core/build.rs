use std::env;
use std::fs;
use std::path::Path;

// This build script extracts the syntax hints from the README.md file and generates a Rust source file
fn main() {
    println!("cargo:rerun-if-changed=README.md");

    let readme = fs::read_to_string("README.md").expect("README.md not found");

    let start_marker = "<!-- docs:syntax-start -->";
    let end_marker = "<!-- docs:syntax-end -->";

    let start = readme
        .find(start_marker)
        .expect("docs:syntax-start marker not found in README.md")
        + start_marker.len();
    let end = readme
        .find(end_marker)
        .expect("docs:syntax-end marker not found in README.md");

    let syntax_text = readme[start..end].trim();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("hints_text.rs");
    fs::write(
        &dest_path,
        format!("pub const HINTS_TEXT: &str = {:?};\n", syntax_text),
    )
    .unwrap();
}
