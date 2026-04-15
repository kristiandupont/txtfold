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

/// Pipe `input` to `txtfold --output-format json [extra_args]` and return parsed JSON.
fn run_txtfold(input: &str, extra_args: &[&str]) -> serde_json::Value {
    let mut child = Command::new(txtfold_bin())
        .args(["--output-format", "json"])
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

    let ratio = output["metadata"]["reduction_ratio"]
        .as_f64()
        .unwrap_or_else(|| panic!("{label}: missing metadata.reduction_ratio"));
    assert!(
        ratio > 0.0 && ratio < 2.0,
        "{label}: reduction_ratio {ratio:.3} out of expected range (0, 2)"
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
    let output = run_txtfold(&data, &["--format", "line"]);

    assert_coarse_invariants(&output, "web/template");
    assert_eq!(output["metadata"]["algorithm"], "template_extraction");
    assert_eq!(output["metadata"]["total_entries"].as_u64().unwrap(), 500);
}

#[test]
fn test_app_preset_template() {
    let data = generate_sample("app", 500, 42);
    let output = run_txtfold(&data, &["--format", "line"]);

    assert_coarse_invariants(&output, "app/template");
    assert_eq!(output["metadata"]["algorithm"], "template_extraction");
}

#[test]
fn test_noisy_preset_clustering() {
    let data = generate_sample("noisy", 200, 42);
    let output = run_txtfold(&data, &["similar(0.8)", "--format", "line"]);

    assert_coarse_invariants(&output, "noisy/clustering");
    assert_eq!(output["metadata"]["algorithm"], "edit_distance_clustering");
}

#[test]
fn test_multiline_preset() {
    let data = generate_sample("multiline", 100, 42);
    let output = run_txtfold(&data, &["--format", "block"]);

    assert_coarse_invariants(&output, "multiline");
    // Block mode groups lines into multi-line entries, so entry count < raw line count.
    let total_entries = output["metadata"]["total_entries"].as_u64().unwrap();
    assert!(total_entries > 0 && total_entries <= 100);
}

// ── nested JSON tests ─────────────────────────────────────────────────────────
//
// The json-records preset generates an envelope-pattern array where every record
// has the same top-level schema {type, data, meta} regardless of record type.
// This makes it a good litmus test for depth-aware schema clustering:
//   flat (depth=0) → 1-2 clusters (envelope dominates)
//   depth=1        → 3+ clusters (user/order/error data sub-schemas diverge)
//
// The json-document preset generates a single JSON object where the same schema
// appears at several distinct paths, which is what the subtree algorithm targets.

/// Flat schema on json-records: the envelope pattern {type,data,meta} dominates,
/// so nearly all records land in one cluster. Tests that existing schema clustering
/// works correctly with the new preset (should pass before any new features land).
#[test]
fn test_json_records_schema_flat() {
    let data = generate_sample("json-records", 200, 42);
    let output = run_txtfold(&data, &["schemas", "--format", "json", "--depth", "0"]);

    assert_coarse_invariants(&output, "json-records/schema-flat");
    assert_eq!(output["metadata"]["algorithm"], "schema_clustering");

    // At flat depth all three event types look like {type: string, data: object,
    // meta: object} → they merge into one large cluster.  System events (no meta)
    // form a separate tiny cluster.  So we expect ≤3 groups total.
    let schemas = output["results"]["schemas"].as_array()
        .expect("json-records/schema-flat: results.schemas missing");
    assert!(
        schemas.len() <= 3,
        "flat schema should find ≤3 top-level shapes, got {}",
        schemas.len()
    );

    // The dominant cluster should contain most of the 200 records (~97%).
    let largest = schemas[0]["count"].as_u64().unwrap_or(0);
    assert!(
        largest > 150,
        "dominant cluster should contain >150 of 200 records, got {}",
        largest
    );
}

/// Depth-1 schema on json-records: the three distinct `data` sub-schemas
/// (user/order/error) should produce separate clusters once nested schemas
/// are compared.
///
/// This test is EXPECTED TO FAIL until --depth is implemented in the schema
/// algorithm.
#[test]
fn test_json_records_schema_depth1() {
    let data = generate_sample("json-records", 200, 42);
    let output = run_txtfold(&data, &["schemas", "--format", "json", "--depth", "1"]);

    assert_coarse_invariants(&output, "json-records/schema-depth1");
    assert_eq!(output["metadata"]["algorithm"], "schema_clustering");

    // With depth=1 the user/order/error data sub-schemas are structurally distinct
    // enough (different field sets) to fall below the default 0.8 threshold →
    // at least 3 separate clusters.
    let schemas = output["results"]["schemas"].as_array()
        .expect("json-records/schema-depth1: results.schemas missing");
    assert!(
        schemas.len() >= 3,
        "depth-1 schema should find ≥3 groups (user/order/error), got {}",
        schemas.len()
    );

    // No single cluster should swallow >80% of records (user events are ~60%, not
    // order+error).
    let largest = schemas[0]["count"].as_u64().unwrap_or(0);
    assert!(
        largest < 160,
        "with depth=1 no cluster should contain >160 of 200 records, got {}",
        largest
    );
}

/// Subtree algorithm on json-document: the same schema appears at multiple
/// distinct paths and should be reported as one pattern with several locations.
///
/// This test is EXPECTED TO FAIL until the subtree algorithm is implemented.
#[test]
fn test_json_document_subtree() {
    let data = generate_sample("json-document", 100, 42);
    let output = run_txtfold(&data, &["subtree", "--format", "json"]);

    assert_coarse_invariants(&output, "json-document/subtree");
    assert_eq!(output["metadata"]["algorithm"], "subtree");

    let patterns = output["results"]["patterns"].as_array()
        .expect("json-document/subtree: results.patterns missing");
    assert!(!patterns.is_empty(), "subtree should find at least one pattern");

    // The user shape {id, name, email} appears at $.users[*], $.team.members[*],
    // and $.config.owner → at least one pattern must list ≥3 distinct paths.
    let wide_pattern = patterns.iter().find(|p| {
        p["paths"].as_array().map(|v| v.len() >= 3).unwrap_or(false)
    });
    assert!(
        wide_pattern.is_some(),
        "expected a pattern appearing at ≥3 distinct paths (user shape)"
    );

    // The order shape {order_id, amount, status, category} appears at $.orders[*]
    // and $.archive[*] → at least one pattern with ≥2 paths.
    let two_path_pattern = patterns.iter().find(|p| {
        p["paths"].as_array().map(|v| v.len() >= 2).unwrap_or(false)
    });
    assert!(
        two_path_pattern.is_some(),
        "expected a pattern appearing at ≥2 distinct paths (order shape)"
    );
}

#[test]
fn test_deterministic_with_seed() {
    let data1 = generate_sample("web", 200, 99);
    let data2 = generate_sample("web", 200, 99);
    assert_eq!(data1, data2, "same seed must produce identical data");

    let out1 = run_txtfold(&data1, &["--format", "line"]);
    let out2 = run_txtfold(&data2, &["--format", "line"]);
    assert_eq!(out1, out2, "same input must produce identical analysis");
}
