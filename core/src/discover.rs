//! Structural discovery for JSON, line, and block formats.
//!
//! The `discover()` function performs a fast structural scan and returns a compact
//! schema map (~300 tokens) describing field paths, types, cardinality, and sample
//! values. Output is designed to be read by a human or LLM to understand document
//! structure before writing a pipeline expression.
//!
//! Discovery always runs on the full document with no filtering.

use crate::parser::{is_json_map, parse_json_map, EntryMode, EntryParser};
use crate::tokenizer::{Token, Tokenizer};
use crate::InputFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

// ── Output types ──────────────────────────────────────────────────────────────

/// Summary of a single field/slot discovered in the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FieldSummary {
    /// Normalized path, e.g. `$.diagnostics[*].category` or `slot[0]`
    pub path: String,
    /// Value types seen at this path, e.g. `["string", "null"]`
    pub types: Vec<String>,
    /// Number of distinct values seen (capped at 10 000)
    pub cardinality: usize,
    /// Up to 5 representative values
    pub samples: Vec<String>,
    /// Fraction of entries that contain this field (0.0–1.0)
    pub present_in_pct: f32,
}

/// Output of the discover operation — a compact structural schema map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DiscoverOutput {
    /// Input format: `"json"`, `"line"`, or `"block"`
    pub format: String,
    /// Total number of top-level entries processed
    pub entry_count: usize,
    /// Per-field summaries
    pub fields: Vec<FieldSummary>,
}

// Pipeline syntax instructions printed with "--syntax" — extracted from README.md at build time.
include!(concat!(env!("OUT_DIR"), "/hints_text.rs"));

impl DiscoverOutput {
    /// Render a compact markdown table.
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();

        // When there is exactly one entry and a nested array is present, the
        // entry count is technically correct (one root object) but misleading —
        // the user should use a path selector to reach the real entries.
        let root_object_note = if self.format == "json"
            && self.entry_count == 1
            && pipeline_selector(&self.fields).is_some()
        {
            "  (root object — use path selector below)"
        } else {
            ""
        };

        writeln!(
            out,
            "Format: {}  |  Entries: {}{}",
            self.format, self.entry_count, root_object_note
        )
        .unwrap();

        if self.fields.is_empty() {
            out.push_str("No fields found.\n");
            return out;
        }

        // Compute column widths.
        let path_w = self
            .fields
            .iter()
            .map(|f| f.path.len())
            .max()
            .unwrap_or(4)
            .max(4);
        let types_w = self
            .fields
            .iter()
            .map(|f| f.types.join(", ").len())
            .max()
            .unwrap_or(5)
            .max(5);

        writeln!(
            out,
            "{:<path_w$}  {:<types_w$}  {:>11}  Samples",
            "Path",
            "Types",
            "Cardinality",
            path_w = path_w,
            types_w = types_w,
        )
        .unwrap();
        writeln!(out, "{}", "-".repeat(path_w + types_w + 30)).unwrap();

        for field in &self.fields {
            let types_str = field.types.join(", ");
            let samples_str = if field.samples.is_empty() {
                String::new()
            } else {
                let joined = field
                    .samples
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                if field.cardinality > field.samples.len() {
                    format!("{}, \u{2026}", joined)
                } else {
                    joined
                }
            };

            writeln!(
                out,
                "{:<path_w$}  {:<types_w$}  {:>11}  {}",
                field.path,
                types_str,
                field.cardinality,
                samples_str,
                path_w = path_w,
                types_w = types_w,
            )
            .unwrap();
        }

        // Pipeline selector hint — only meaningful for JSON input.
        if self.format == "json" {
            out.push('\n');
            match pipeline_selector(&self.fields) {
                Some(selector) => {
                    writeln!(out, "Pipeline selector: {}", selector).unwrap();
                }
                None => {
                    out.push_str(
                        "Pipeline selector: \
                         (top-level array — no path selection needed; use verbs directly)\n",
                    );
                }
            }
        }

        out
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run structural discovery on `input` and return a compact schema map.
pub fn discover(input: &str, input_format: InputFormat) -> Result<DiscoverOutput, String> {
    match input_format {
        InputFormat::Json => discover_json(input),
        InputFormat::Line => {
            let parser = EntryParser::new(EntryMode::SingleLine);
            let entries = parser.parse(input);
            Ok(discover_entries(entries, "line"))
        }
        InputFormat::Block { entry_pattern } => {
            let parser = if let Some(ref pattern) = entry_pattern {
                EntryParser::new(EntryMode::MultiLine).with_entry_pattern(pattern)?
            } else {
                EntryParser::new(EntryMode::MultiLine)
            };
            let entries = parser.parse(input);
            Ok(discover_entries(entries, "block"))
        }
    }
}

// ── JSON discovery ────────────────────────────────────────────────────────────

fn discover_json(input: &str) -> Result<DiscoverOutput, String> {
    // Normalise map-style JSON into an array so all top-level keys share the
    // same `$[*]` prefix, giving aggregated field statistics.
    let (root, entry_count): (Value, usize) = {
        let raw: Value = serde_json::from_str(input)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        match raw {
            Value::Array(ref arr) => {
                let n = arr.len();
                (raw, n)
            }
            Value::Object(_) if is_json_map(input) => {
                let (values, _) = parse_json_map(input)
                    .map_err(|e| format!("Failed to parse JSON map: {}", e))?;
                let n = values.len();
                (Value::Array(values), n)
            }
            other => (other, 1),
        }
    };

    let mut field_data: BTreeMap<String, FieldData> = BTreeMap::new();
    // Tracks how many array elements appear at each normalized array path.
    let mut array_counts: BTreeMap<String, usize> = BTreeMap::new();

    collect_leaf_fields(&root, "$", &mut field_data, &mut array_counts);

    let fields: Vec<FieldSummary> = field_data
        .into_iter()
        .map(|(path, data)| {
            // present_in_pct: compare this field's occurrence count against the
            // total number of elements in the nearest enclosing array.
            let present_in_pct = if let Some(array_path) = nearest_array_path(&path) {
                let parent_count = array_counts.get(array_path).copied().unwrap_or(1);
                if parent_count > 0 {
                    (data.count as f32 / parent_count as f32).min(1.0)
                } else {
                    1.0
                }
            } else {
                // No array ancestor — always present in this single document.
                1.0
            };

            let mut types: Vec<String> = data.types.into_keys().collect();
            types.sort();

            FieldSummary {
                path,
                types,
                cardinality: data.distinct_values.len(),
                samples: data.samples,
                present_in_pct,
            }
        })
        .collect();
    // BTreeMap iterates in sorted key order, so `fields` is already sorted.

    Ok(DiscoverOutput {
        format: "json".to_string(),
        entry_count,
        fields,
    })
}

/// Walk `value` at `path`, recording every leaf field into `field_data` and
/// every array element count into `array_counts`.
fn collect_leaf_fields(
    value: &Value,
    path: &str,
    field_data: &mut BTreeMap<String, FieldData>,
    array_counts: &mut BTreeMap<String, usize>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{}.{}", path, key);
                collect_leaf_fields(child, &child_path, field_data, array_counts);
            }
        }
        Value::Array(arr) => {
            let element_path = format!("{}[*]", path);
            *array_counts.entry(element_path.clone()).or_insert(0) += arr.len();
            for item in arr {
                collect_leaf_fields(item, &element_path, field_data, array_counts);
            }
        }
        leaf => {
            let type_name = json_type_name(leaf);
            let value_str = json_value_sample(leaf);
            field_data
                .entry(path.to_string())
                .or_insert_with(FieldData::new)
                .record(type_name, value_str);
        }
    }
}

/// Return the part of `path` up to and including the last `[*]`, if any.
///
/// Examples:
/// - `"$.diagnostics[*].category"` → `Some("$.diagnostics[*]")`
/// - `"$.foo[*].bar[*].baz"` → `Some("$.foo[*].bar[*]")`
/// - `"$.name"` → `None`
fn nearest_array_path(path: &str) -> Option<&str> {
    path.rfind("[*]").map(|idx| &path[..idx + 3])
}

/// Derive a pipeline-ready path selector from a list of discovered fields.
///
/// Returns `Some(".foo[]")` when entries are nested inside a named array, or
/// `None` when they live at the top level of a JSON array (no path selection
/// needed — the user can write verbs directly).
///
/// The shallowest (shortest) array path is used because that is usually the
/// natural "entry array" the user wants to analyse.
fn pipeline_selector(fields: &[FieldSummary]) -> Option<String> {
    // Collect every unique array-container path seen in the field list.
    let mut array_paths: std::collections::BTreeSet<String> = Default::default();
    for field in fields {
        if let Some(p) = nearest_array_path(&field.path) {
            array_paths.insert(p.to_string());
        }
    }
    // Pick the shallowest path (shortest string length).
    let shallowest = array_paths.iter().min_by_key(|p| p.len())?;

    if shallowest == "$[*]" {
        // Top-level array — no path selection needed.
        None
    } else {
        // Strip the leading `$` and replace `[*]` with `[]` to produce a
        // valid pipeline path expression, e.g. `$.foo[*]` → `.foo[]`.
        let selector = shallowest
            .strip_prefix('$')
            .unwrap_or(shallowest)
            .replace("[*]", "[]");
        Some(selector)
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn json_value_sample(value: &Value) -> String {
    const MAX_CHARS: usize = 100;
    match value {
        Value::String(s) => truncate_sample(s, MAX_CHARS),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(_) => "[\u{2026}]".to_string(),
        Value::Object(_) => "{\u{2026}}".to_string(),
    }
}

// ── Line / block discovery ────────────────────────────────────────────────────

fn discover_entries(entries: Vec<crate::entry::Entry>, format_name: &str) -> DiscoverOutput {
    let entry_count = entries.len();

    if entries.is_empty() {
        return DiscoverOutput {
            format: format_name.to_string(),
            entry_count: 0,
            fields: vec![],
        };
    }

    // Per-slot accumulators. A "slot" is a non-whitespace, non-punctuation
    // token position within a line.
    let mut slots: Vec<FieldData> = Vec::new();

    for entry in &entries {
        // Tokenize the first (header) line of each entry.
        let line = entry.first_line().unwrap_or("");
        let tokens = Tokenizer::tokenize(line);

        // Skip whitespace and punctuation when assigning slot indices.
        let meaningful: Vec<&Token> = tokens
            .iter()
            .filter(|t| !matches!(t, Token::Whitespace | Token::Punctuation(_)))
            .collect();

        for (slot_idx, token) in meaningful.iter().enumerate() {
            if slot_idx >= slots.len() {
                slots.resize_with(slot_idx + 1, FieldData::new);
            }
            slots[slot_idx].record(token_type_name(token), token_value_str(token));
        }
    }

    let fields: Vec<FieldSummary> = slots
        .into_iter()
        .enumerate()
        .map(|(i, data)| {
            let present_in_pct = data.count as f32 / entry_count as f32;
            let mut types: Vec<String> = data.types.into_keys().collect();
            types.sort();
            FieldSummary {
                path: format!("slot[{}]", i),
                types,
                cardinality: data.distinct_values.len(),
                samples: data.samples,
                present_in_pct,
            }
        })
        .collect();

    DiscoverOutput {
        format: format_name.to_string(),
        entry_count,
        fields,
    }
}

fn token_type_name(token: &Token) -> &'static str {
    match token {
        Token::Literal(_) => "literal",
        Token::Number(_) => "number",
        Token::Timestamp(_) | Token::Date(_) => "timestamp",
        Token::Identifier(_) => "identifier",
        Token::Duration(_) => "duration",
        Token::IpAddress(_) => "ip_address",
        Token::Whitespace => "whitespace",
        Token::Punctuation(_) => "punctuation",
    }
}

fn token_value_str(token: &Token) -> String {
    match token {
        Token::Literal(s)
        | Token::Number(s)
        | Token::Timestamp(s)
        | Token::Date(s)
        | Token::Identifier(s)
        | Token::Duration(s)
        | Token::IpAddress(s) => truncate_sample(s, 100),
        Token::Whitespace => " ".to_string(),
        Token::Punctuation(c) => c.to_string(),
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Per-field accumulator used during tree/token walking.
struct FieldData {
    types: HashMap<String, usize>,
    distinct_values: HashSet<String>,
    samples: Vec<String>,
    count: usize,
}

/// Maximum number of distinct values tracked per field (memory guard).
const DISTINCT_CAP: usize = 10_000;

impl FieldData {
    fn new() -> Self {
        FieldData {
            types: HashMap::new(),
            distinct_values: HashSet::new(),
            samples: Vec::new(),
            count: 0,
        }
    }

    fn record(&mut self, type_name: &str, value: String) {
        self.count += 1;
        *self.types.entry(type_name.to_string()).or_insert(0) += 1;

        if self.distinct_values.len() < DISTINCT_CAP {
            self.distinct_values.insert(value.clone());
        }

        if self.samples.len() < 5 && !self.samples.contains(&value) {
            self.samples.push(value);
        }
    }
}

/// Truncate a string to at most `max_chars` Unicode scalar values, appending
/// `…` if truncated. Safe for multi-byte strings.
fn truncate_sample(s: &str, max_chars: usize) -> String {
    if let Some((byte_pos, _)) = s.char_indices().nth(max_chars) {
        format!("{}\u{2026}", &s[..byte_pos])
    } else {
        s.to_string()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InputFormat;

    #[test]
    fn test_discover_json_array() {
        let input = r#"[
            {"category": "error", "code": 1},
            {"category": "warning", "code": 2},
            {"category": "error"}
        ]"#;

        let out = discover(input, InputFormat::Json).unwrap();
        assert_eq!(out.format, "json");
        assert_eq!(out.entry_count, 3);

        let category = out.fields.iter().find(|f| f.path == "$[*].category").unwrap();
        assert_eq!(category.types, vec!["string"]);
        assert_eq!(category.cardinality, 2);
        assert!((category.present_in_pct - 1.0).abs() < 0.01);

        let code = out.fields.iter().find(|f| f.path == "$[*].code").unwrap();
        assert_eq!(code.types, vec!["number"]);
        // 2 out of 3 entries have "code"
        assert!((code.present_in_pct - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_discover_json_nested() {
        let input = r#"{"diagnostics": [{"severity": 1}, {"severity": 2}]}"#;
        let out = discover(input, InputFormat::Json).unwrap();
        // Root is a single object → entry_count = 1
        assert_eq!(out.entry_count, 1);

        let sev = out
            .fields
            .iter()
            .find(|f| f.path.contains("severity"))
            .unwrap();
        assert_eq!(sev.types, vec!["number"]);
        assert_eq!(sev.cardinality, 2);
        // Both elements have severity → 100%
        assert!((sev.present_in_pct - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_discover_line_format() {
        let input = "2024-01-15 10:00:00 ERROR Something failed\n\
                     2024-01-15 10:00:01 INFO  All good\n\
                     2024-01-15 10:00:02 WARN  Watch out\n";

        let out = discover(input, InputFormat::Line).unwrap();
        assert_eq!(out.format, "line");
        assert_eq!(out.entry_count, 3);
        // First slot should be timestamps
        let slot0 = &out.fields[0];
        assert_eq!(slot0.path, "slot[0]");
        assert!(slot0.present_in_pct > 0.99);
    }

    #[test]
    fn test_discover_empty_input() {
        let out = discover("", InputFormat::Line).unwrap();
        assert_eq!(out.entry_count, 0);
        assert!(out.fields.is_empty());
    }

    #[test]
    fn test_to_markdown() {
        let input = r#"[{"name": "alice", "age": 30}, {"name": "bob", "age": 25}]"#;
        let out = discover(input, InputFormat::Json).unwrap();
        let md = out.to_markdown();
        assert!(md.contains("Format: json"));
        assert!(md.contains("Entries: 2"));
        assert!(md.contains("$[*].name"));
    }

    #[test]
    fn test_pipeline_selector_nested_array() {
        let input = r#"{"diagnostics": [{"category": "error"}, {"category": "warning"}]}"#;
        let out = discover(input, InputFormat::Json).unwrap();
        let md = out.to_markdown();
        assert!(md.contains("Pipeline selector: .diagnostics[]"), "nested array should show selector");
    }

    #[test]
    fn test_pipeline_selector_top_level_array() {
        let input = r#"[{"name": "alice"}, {"name": "bob"}]"#;
        let out = discover(input, InputFormat::Json).unwrap();
        let md = out.to_markdown();
        assert!(md.contains("top-level array"), "top-level array should show no-selection-needed message");
    }

    #[test]
    fn test_pipeline_selector_not_shown_for_line_format() {
        let input = "INFO foo\nERROR bar\n";
        let out = discover(input, InputFormat::Line).unwrap();
        let md = out.to_markdown();
        assert!(!md.contains("Pipeline selector"), "line format should not show pipeline selector");
    }

    #[test]
    fn test_nearest_array_path() {
        assert_eq!(
            nearest_array_path("$.diagnostics[*].category"),
            Some("$.diagnostics[*]")
        );
        assert_eq!(
            nearest_array_path("$.foo[*].bar[*].baz"),
            Some("$.foo[*].bar[*]")
        );
        assert_eq!(nearest_array_path("$.name"), None);
        assert_eq!(nearest_array_path("$[*].name"), Some("$[*]"));
    }

    #[test]
    fn test_mixed_types() {
        // A field that is sometimes string, sometimes null
        let input = r#"[{"val": "hello"}, {"val": null}, {"val": "world"}]"#;
        let out = discover(input, InputFormat::Json).unwrap();
        let val = out.fields.iter().find(|f| f.path == "$[*].val").unwrap();
        assert!(val.types.contains(&"string".to_string()));
        assert!(val.types.contains(&"null".to_string()));
    }
}
