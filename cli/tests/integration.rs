use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};


// ── binary helpers ────────────────────────────────────────────────────────────

fn txtfold_bin() -> &'static str {
    env!("CARGO_BIN_EXE_txtfold")
}

fn sample_generator_bin() -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();

    let debug = workspace.join("target/debug/sample-generator");
    if debug.exists() {
        return debug;
    }
    let release = workspace.join("target/release/sample-generator");
    if release.exists() {
        return release;
    }
    panic!("sample-generator binary not found — run `cargo build -p sample-generator` first");
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Run sample-generator and return the generated log content via stdout.
fn generate_sample(preset: &str, lines: usize, seed: u64) -> String {
    let output = Command::new(sample_generator_bin())
        .args([
            "--preset",
            preset,
            "--lines",
            &lines.to_string(),
            "--seed",
            &seed.to_string(),
        ])
        .output()
        .expect("failed to spawn sample-generator");

    assert!(
        output.status.success(),
        "sample-generator exited with failure:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("sample-generator output is not UTF-8")
}

/// Pipe `input` to `txtfold --format json [extra_args]` and return parsed JSON.
fn run_txtfold(input: &str, extra_args: &[&str]) -> serde_json::Value {
    let mut child = Command::new(txtfold_bin())
        .args(["--format", "json"])
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn txtfold");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("write stdin");

    let output = child.wait_with_output().expect("wait for txtfold");
    assert!(
        output.status.success(),
        "txtfold exited with {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("txtfold output is not valid JSON")
}

/// Coarse invariants that every successful run must satisfy.
fn assert_coarse_invariants(output: &serde_json::Value, label: &str) {
    let total_entries = output["metadata"]["total_entries"]
        .as_u64()
        .unwrap_or_else(|| panic!("{label}: missing metadata.total_entries"));
    assert!(total_entries > 0, "{label}: total_entries should be > 0");

    let ratio = output["metadata"]["compression_ratio"]
        .as_f64()
        .unwrap_or_else(|| panic!("{label}: missing metadata.compression_ratio"));
    assert!(
        ratio > 0.0 && ratio < 2.0,
        "{label}: compression_ratio {ratio:.3} out of expected range (0, 2)"
    );

    // Grouped and schema_grouped results must contain at least one pattern.
    let result_type = output["results"]["type"].as_str().unwrap_or("");
    if matches!(result_type, "grouped" | "schema_grouped") {
        let unique_patterns = output["summary"]["unique_patterns"].as_u64().unwrap_or(0);
        assert!(unique_patterns > 0, "{label}: unique_patterns should be > 0");
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_web_preset_template() {
    let data = generate_sample("web", 500, 42);
    let output = run_txtfold(&data, &[]);

    assert_coarse_invariants(&output, "web/template");
    assert_eq!(output["metadata"]["algorithm"], "template_extraction");
    assert_eq!(output["metadata"]["total_entries"].as_u64().unwrap(), 500);
}

#[test]
fn test_app_preset_template() {
    let data = generate_sample("app", 500, 42);
    let output = run_txtfold(&data, &[]);

    assert_coarse_invariants(&output, "app/template");
    assert_eq!(output["metadata"]["algorithm"], "template_extraction");
}

#[test]
fn test_noisy_preset_clustering() {
    let data = generate_sample("noisy", 200, 42);
    let output = run_txtfold(&data, &["--algorithm", "clustering"]);

    assert_coarse_invariants(&output, "noisy/clustering");
    assert_eq!(output["metadata"]["algorithm"], "edit_distance_clustering");
}

#[test]
fn test_multiline_preset() {
    let data = generate_sample("multiline", 100, 42);
    let output = run_txtfold(&data, &["--entry-mode", "multiline"]);

    assert_coarse_invariants(&output, "multiline");
    // Multi-line mode groups lines into blocks, so entry count < raw line count.
    let total_entries = output["metadata"]["total_entries"].as_u64().unwrap();
    assert!(total_entries > 0 && total_entries <= 100);
}

#[test]
fn test_deterministic_with_seed() {
    let data1 = generate_sample("web", 200, 99);
    let data2 = generate_sample("web", 200, 99);
    assert_eq!(data1, data2, "same seed must produce identical data");

    let out1 = run_txtfold(&data1, &[]);
    let out2 = run_txtfold(&data2, &[]);
    assert_eq!(out1, out2, "same input must produce identical analysis");
}
