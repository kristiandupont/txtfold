//! Pipeline expression parser and executor.
//!
//! A pipeline is a `|`-separated sequence of stages, e.g.:
//!
//! ```text
//! .diagnostics[] | del(.sourceCode) | group_by(.category)
//! similar(0.8) | top(20)
//! outliers
//! ```
//!
//! # Stage taxonomy
//!
//! Pre-processing stages transform the input before the algorithm sees it:
//! - `PathSelect` — navigate into a JSON subtree (`.foo[]`, `.foo[0]`)
//! - `Del` — remove fields from each JSON object
//!
//! Algorithm stages select the analysis algorithm (at most one per pipeline,
//! must be the terminal verb or the only non-modifier verb):
//! - `GroupBy` — value-based frequency table
//! - `AlgorithmVerb` — one of: `summarize`, `similar(t)`, `patterns`, `outliers`,
//!   `schemas`, `subtree`
//!
//! Post-processing stages modify the output after the algorithm runs:
//! - `Top(n)` — keep the N largest groups; move the rest to outliers
//! - `Label(field)` — relabel groups using the value of a field
//!
//! # jaq boundary (future)
//! Pre-processing stages that return `Value` (path selection, del, future
//! `select`, `map`) are the natural domain of jaq. The `Stage` enum reserves a
//! `Jaq` variant so the handoff point is explicit in the type system without
//! requiring a rewrite when jaq is integrated.

mod executor;
mod parser;
mod tokenizer;

pub use executor::{apply_pipeline, is_verb_name, parse_pipeline, partition_by_field};

use crate::entry::Entry;
use serde_json::Value;

// ── Public types ─────────────────────────────────────────────────────────────

/// A segment in a JSON path expression.
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// `.field` — navigate into an object field.
    Field(String),
    /// `[]` or `[*]` — iterate all elements of an array.
    All,
    /// `[n]` — select element at index n.
    Index(usize),
}

/// Comparison operator used in `where(...)` filter expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereOp {
    Eq,
    Ne,
    Contains,
    StartsWith,
    EndsWith,
}

/// Right-hand-side value in a `where(...)` filter expression.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereValue {
    String(String),
    Number(f64),
}

/// A single stage in a pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    // ── Pre-processing ───────────────────────────────────────────────────────
    /// Navigate into a JSON subtree before analysis. JSON-only.
    PathSelect(Vec<PathSegment>),
    /// Remove fields (by dotted path) from each JSON object. JSON-only.
    /// Each inner `Vec<String>` is the sequence of field-name segments in the path,
    /// e.g. `del(.location.sourceCode)` → `vec![vec!["location", "sourceCode"]]`.
    Del(Vec<Vec<String>>),
    /// Keep only entries where a field value matches a condition. JSON-only.
    Where {
        /// Dotted path to the field to test (same syntax as `del`).
        field: Vec<String>,
        op: WhereOp,
        value: WhereValue,
    },

    // ── Algorithm selection ──────────────────────────────────────────────────
    /// Value-based frequency table grouped by a field. JSON (and future line/block).
    GroupBy(String),
    /// One of the named algorithm verbs.
    AlgorithmVerb(AlgorithmDirective),

    // ── Post-processing ──────────────────────────────────────────────────────
    /// Keep the N largest groups; move the rest to a remainder bucket.
    Top(usize),
    /// Relabel each group using the value of a field.
    Label(String),

    /// Reserved for future jaq pre-processing integration.
    /// The parser never emits this variant today; it exists so the type system
    /// makes the jaq/txtfold boundary explicit when jaq is wired in.
    #[allow(dead_code)]
    Jaq(String),
}

/// Algorithm directive — the algorithm that should run.
#[derive(Debug, Clone, PartialEq)]
pub enum AlgorithmDirective {
    /// Default: fixed per-format table (json→subtree, line/block→template).
    Summarize,
    /// Edit-distance clustering at threshold `t`.
    Similar(f64),
    /// Template extraction algorithm.
    Patterns,
    /// N-gram outlier detection algorithm.
    Outliers,
    /// Schema clustering algorithm (JSON).
    Schemas,
    /// Subtree algorithm (JSON).
    Subtree,
}

/// Input handed to the pipeline executor.
#[derive(Debug, Clone)]
pub enum PipelineInput {
    Json(Vec<Value>),
    Text(Vec<Entry>),
}

/// Result returned by [`apply_pipeline`].
#[derive(Debug)]
pub struct PipelineResult {
    /// Transformed input after pre-processing stages.
    pub input: PipelineInput,
    /// Algorithm to run (from the terminal algorithm verb, or `Summarize`).
    pub algorithm: AlgorithmDirective,
    /// Optional value-based grouping field (from `group_by`).
    pub group_by_field: Option<String>,
    /// Truncate output to N groups after the algorithm runs.
    pub top: Option<usize>,
    /// Relabel groups by this field after the algorithm runs.
    pub label: Option<String>,
}

/// A parse error with a byte position and a human-readable hint.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Byte offset into the expression string where the problem was detected.
    pub position: usize,
    /// Human-readable description of the problem.
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at position {}: {}", self.position, self.message)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Entry;

    #[test]
    fn test_parse_single_verb() {
        let stages = parse_pipeline("outliers").unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Outliers));
    }

    #[test]
    fn test_parse_summarize() {
        let stages = parse_pipeline("summarize").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Summarize));
    }

    #[test]
    fn test_parse_similar() {
        let stages = parse_pipeline("similar(0.8)").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Similar(0.8)));
    }

    #[test]
    fn test_parse_similar_integer_threshold() {
        let stages = parse_pipeline("similar(1)").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Similar(1.0)));
    }

    #[test]
    fn test_parse_del() {
        let stages = parse_pipeline("del(.sourceCode, .dictionary)").unwrap();
        assert_eq!(
            stages[0],
            Stage::Del(vec![
                vec!["sourceCode".to_string()],
                vec!["dictionary".to_string()],
            ])
        );
    }

    #[test]
    fn test_parse_del_dotted_path() {
        let stages = parse_pipeline("del(.location.sourceCode)").unwrap();
        assert_eq!(
            stages[0],
            Stage::Del(vec![vec!["location".to_string(), "sourceCode".to_string()]])
        );
    }

    #[test]
    fn test_parse_del_mixed_paths() {
        let stages = parse_pipeline("del(.sourceCode, .location.file, .advices)").unwrap();
        assert_eq!(
            stages[0],
            Stage::Del(vec![
                vec!["sourceCode".to_string()],
                vec!["location".to_string(), "file".to_string()],
                vec!["advices".to_string()],
            ])
        );
    }

    #[test]
    fn test_parse_group_by() {
        let stages = parse_pipeline("group_by(.category)").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("category".to_string()));
    }

    #[test]
    fn test_parse_top() {
        let stages = parse_pipeline("top(20)").unwrap();
        assert_eq!(stages[0], Stage::Top(20));
    }

    #[test]
    fn test_parse_label() {
        let stages = parse_pipeline("label(.name)").unwrap();
        assert_eq!(stages[0], Stage::Label("name".to_string()));
    }

    #[test]
    fn test_parse_path_select_all() {
        let stages = parse_pipeline(".diagnostics[]").unwrap();
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("diagnostics".to_string()),
                PathSegment::All,
            ])
        );
    }

    #[test]
    fn test_parse_path_select_star() {
        let stages = parse_pipeline(".diagnostics[*]").unwrap();
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("diagnostics".to_string()),
                PathSegment::All,
            ])
        );
    }

    #[test]
    fn test_parse_path_select_index() {
        let stages = parse_pipeline(".items[0]").unwrap();
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("items".to_string()),
                PathSegment::Index(0),
            ])
        );
    }

    #[test]
    fn test_parse_multi_stage_pipeline() {
        let stages = parse_pipeline(".diagnostics[] | del(.sourceCode) | group_by(.category)").unwrap();
        assert_eq!(stages.len(), 3);
        assert_eq!(
            stages[0],
            Stage::PathSelect(vec![
                PathSegment::Field("diagnostics".to_string()),
                PathSegment::All,
            ])
        );
        assert_eq!(stages[1], Stage::Del(vec![vec!["sourceCode".to_string()]]));
        assert_eq!(stages[2], Stage::GroupBy("category".to_string()));
    }

    #[test]
    fn test_parse_similar_top_pipeline() {
        let stages = parse_pipeline("similar(0.8) | top(20)").unwrap();
        assert_eq!(stages[0], Stage::AlgorithmVerb(AlgorithmDirective::Similar(0.8)));
        assert_eq!(stages[1], Stage::Top(20));
    }

    #[test]
    fn test_parse_error_unknown_verb() {
        let err = parse_pipeline("frobnicate").unwrap_err();
        assert!(err.message.contains("frobnicate"));
    }

    #[test]
    fn test_parse_error_empty() {
        assert!(parse_pipeline("").is_err());
    }

    #[test]
    fn test_apply_pipeline_del() {
        let values = vec![serde_json::json!({"a": 1, "b": 2, "c": 3})];
        let stages = parse_pipeline("del(.b)").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert!(vals[0].get("a").is_some());
                assert!(vals[0].get("b").is_none());
                assert!(vals[0].get("c").is_some());
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_del_dotted_path() {
        let values = vec![serde_json::json!({
            "category": "error",
            "location": {"file": "main.rs", "line": 42}
        })];
        let stages = parse_pipeline("del(.location.file)").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals[0]["category"], "error");
                assert!(vals[0].get("location").is_some());
                assert!(vals[0]["location"].get("file").is_none());
                assert_eq!(vals[0]["location"]["line"], 42);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_del_missing_intermediate_key() {
        let values = vec![serde_json::json!({"x": 1})];
        let stages = parse_pipeline("del(.a.b)").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals[0]["x"], 1);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_path_select() {
        let values = vec![serde_json::json!({"items": [{"x": 1}, {"x": 2}]})];
        let stages = parse_pipeline(".items[]").unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 2);
                assert_eq!(vals[0]["x"], 1);
                assert_eq!(vals[1]["x"], 2);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_pipeline_extracts_algorithm() {
        let stages = parse_pipeline("del(.x) | schemas").unwrap();
        let values = vec![serde_json::json!({"a": 1, "x": 9})];
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        assert_eq!(result.algorithm, AlgorithmDirective::Schemas);
        assert_eq!(result.top, None);
    }

    #[test]
    fn test_apply_pipeline_extracts_top_and_label() {
        let stages = parse_pipeline("patterns | top(5) | label(.name)").unwrap();
        let entries: Vec<Entry> = vec![];
        let result = apply_pipeline(&stages, PipelineInput::Text(entries)).unwrap();
        assert_eq!(result.algorithm, AlgorithmDirective::Patterns);
        assert_eq!(result.top, Some(5));
        assert_eq!(result.label, Some("name".to_string()));
    }

    #[test]
    fn test_del_on_text_input_errors() {
        let stages = parse_pipeline("del(.x)").unwrap();
        let entries: Vec<Entry> = vec![];
        let err = apply_pipeline(&stages, PipelineInput::Text(entries)).unwrap_err();
        assert!(err.contains("JSON"));
    }

    #[test]
    fn test_partition_by_field() {
        let values = vec![
            serde_json::json!({"level": "error", "msg": "a"}),
            serde_json::json!({"level": "warn",  "msg": "b"}),
            serde_json::json!({"level": "error", "msg": "c"}),
            serde_json::json!({"msg": "no level"}),
        ];
        let (groups, ungrouped) = partition_by_field(&values, "level");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "error");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "warn");
        assert_eq!(ungrouped.len(), 1);
    }

    #[test]
    fn test_parse_group_by_slot() {
        let stages = parse_pipeline("group_by(slot[3])").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("slot[3]".to_string()));
    }

    #[test]
    fn test_parse_group_by_slot_zero() {
        let stages = parse_pipeline("group_by(slot[0])").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("slot[0]".to_string()));
    }

    #[test]
    fn test_parse_group_by_slot_in_pipeline() {
        let stages = parse_pipeline("group_by(slot[2]) | top(10)").unwrap();
        assert_eq!(stages[0], Stage::GroupBy("slot[2]".to_string()));
        assert_eq!(stages[1], Stage::Top(10));
    }

    #[test]
    fn test_parse_where_eq() {
        let stages = parse_pipeline(r#"where(.severity == "error")"#).unwrap();
        assert_eq!(
            stages[0],
            Stage::Where {
                field: vec!["severity".to_string()],
                op: WhereOp::Eq,
                value: WhereValue::String("error".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_where_ne() {
        let stages = parse_pipeline(r#"where(.category != "lint")"#).unwrap();
        assert_eq!(
            stages[0],
            Stage::Where {
                field: vec!["category".to_string()],
                op: WhereOp::Ne,
                value: WhereValue::String("lint".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_where_contains() {
        let stages = parse_pipeline(r#"where(.file contains "src/")"#).unwrap();
        assert_eq!(
            stages[0],
            Stage::Where {
                field: vec!["file".to_string()],
                op: WhereOp::Contains,
                value: WhereValue::String("src/".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_where_dotted_path() {
        let stages = parse_pipeline(r#"where(.location.file starts_with "src")"#).unwrap();
        assert_eq!(
            stages[0],
            Stage::Where {
                field: vec!["location".to_string(), "file".to_string()],
                op: WhereOp::StartsWith,
                value: WhereValue::String("src".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_where_in_pipeline() {
        let stages =
            parse_pipeline(r#".diagnostics[] | where(.severity == "error") | group_by(.category)"#)
                .unwrap();
        assert_eq!(stages.len(), 3);
        assert!(matches!(stages[1], Stage::Where { .. }));
    }

    #[test]
    fn test_apply_where_eq() {
        let values = vec![
            serde_json::json!({"level": "error", "msg": "a"}),
            serde_json::json!({"level": "warn",  "msg": "b"}),
            serde_json::json!({"level": "error", "msg": "c"}),
        ];
        let stages = parse_pipeline(r#"where(.level == "error")"#).unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 2);
                assert!(vals.iter().all(|v| v["level"] == "error"));
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_where_ne() {
        let values = vec![
            serde_json::json!({"level": "error"}),
            serde_json::json!({"level": "warn"}),
            serde_json::json!({"level": "info"}),
        ];
        let stages = parse_pipeline(r#"where(.level != "error")"#).unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 2);
                assert!(vals.iter().all(|v| v["level"] != "error"));
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_where_contains() {
        let values = vec![
            serde_json::json!({"path": "src/app/main.ts"}),
            serde_json::json!({"path": "e2e/results/trace.html"}),
            serde_json::json!({"path": "src/lib/utils.ts"}),
        ];
        let stages = parse_pipeline(r#"where(.path contains "src/")"#).unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 2);
                assert!(vals.iter().all(|v| v["path"].as_str().unwrap().contains("src/")));
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_where_starts_with() {
        let values = vec![
            serde_json::json!({"file": "src/app.ts"}),
            serde_json::json!({"file": "prisma/seed.ts"}),
        ];
        let stages = parse_pipeline(r#"where(.file starts_with "src")"#).unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 1);
                assert_eq!(vals[0]["file"], "src/app.ts");
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_where_ends_with() {
        let values = vec![
            serde_json::json!({"file": "src/app.ts"}),
            serde_json::json!({"file": "src/comp.tsx"}),
        ];
        let stages = parse_pipeline(r#"where(.file ends_with ".tsx")"#).unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 1);
                assert_eq!(vals[0]["file"], "src/comp.tsx");
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_apply_where_dotted_path() {
        let values = vec![
            serde_json::json!({"location": {"file": "src/main.ts"}}),
            serde_json::json!({"location": {"file": "e2e/trace.html"}}),
        ];
        let stages = parse_pipeline(r#"where(.location.file starts_with "src")"#).unwrap();
        let result = apply_pipeline(&stages, PipelineInput::Json(values)).unwrap();
        match result.input {
            PipelineInput::Json(vals) => {
                assert_eq!(vals.len(), 1);
            }
            _ => panic!("expected JSON output"),
        }
    }

    #[test]
    fn test_where_on_text_input_errors() {
        let stages = parse_pipeline(r#"where(.x == "y")"#).unwrap();
        let entries: Vec<Entry> = vec![];
        let err = apply_pipeline(&stages, PipelineInput::Text(entries)).unwrap_err();
        assert!(err.contains("JSON"));
    }

    #[test]
    fn test_is_verb_name() {
        assert!(is_verb_name("summarize"));
        assert!(is_verb_name("outliers"));
        assert!(is_verb_name("group_by"));
        assert!(!is_verb_name("foo"));
        assert!(!is_verb_name("app.log"));
    }
}
