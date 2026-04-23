use super::parser::Parser;
use super::tokenizer::Tokenizer;
use super::{AlgorithmDirective, ParseError, PathSegment, PipelineInput, PipelineResult, Stage, WhereOp, WhereValue};
use serde_json::Value;

/// Parse a pipeline expression string into a list of stages.
///
/// Returns a [`ParseError`] with the byte offset and a hint on failure.
pub fn parse_pipeline(expr: &str) -> Result<Vec<Stage>, ParseError> {
    let tokens = Tokenizer::new(expr).tokenize()?;
    if tokens.is_empty() {
        return Err(ParseError {
            position: 0,
            message: "empty pipeline expression".to_string(),
        });
    }
    Parser::new(tokens).parse_pipeline()
}

/// Execute the pre-processing stages of a pipeline and extract the algorithm
/// directive and post-processing modifiers.
///
/// # Stage execution order
/// 1. Pre-processing stages (`PathSelect`, `Del`) are applied sequentially to
///    the input, transforming it before the algorithm sees it.
/// 2. The algorithm directive comes from the last `AlgorithmVerb` or `GroupBy`
///    stage (or `Summarize` if none is present).
/// 3. Post-processing modifiers (`Top`, `Label`) are collected and returned for
///    the caller to apply after the algorithm runs.
///
/// Returns `Err` if a JSON-only stage is used with text input.
pub fn apply_pipeline(
    stages: &[Stage],
    input: PipelineInput,
) -> Result<PipelineResult, String> {
    let mut input = input;
    let mut algorithm = AlgorithmDirective::Summarize;
    let mut group_by_field: Option<String> = None;
    let mut top: Option<usize> = None;
    let mut label: Option<String> = None;

    for stage in stages {
        match stage {
            Stage::PathSelect(segments) => {
                input = apply_path_select(input, segments)?;
            }
            Stage::Del(fields) => {
                input = apply_del(input, fields)?;
            }
            Stage::Where { field, op, value } => {
                input = apply_where(input, field, op, value)?;
            }
            Stage::GroupBy(field) => {
                group_by_field = Some(field.clone());
                // GroupBy also drives algorithm selection.
                algorithm = AlgorithmDirective::Summarize; // placeholder; group_by_field signals the real path
            }
            Stage::AlgorithmVerb(dir) => {
                algorithm = dir.clone();
            }
            Stage::Top(n) => {
                top = Some(*n);
            }
            Stage::Label(field) => {
                label = Some(field.clone());
            }
            Stage::Jaq(_) => {
                return Err("jaq integration is not yet implemented".to_string());
            }
        }
    }

    Ok(PipelineResult {
        input,
        algorithm,
        group_by_field,
        top,
        label,
    })
}

// ── Pre-processing helpers ────────────────────────────────────────────────────

fn apply_path_select(input: PipelineInput, segments: &[PathSegment]) -> Result<PipelineInput, String> {
    match input {
        PipelineInput::Json(values) => {
            let mut current: Vec<Value> = values;

            for seg in segments {
                current = match seg {
                    PathSegment::Field(name) => {
                        current
                            .into_iter()
                            .filter_map(|v| {
                                if let Value::Object(map) = v {
                                    map.get(name).cloned()
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                    PathSegment::All => {
                        current
                            .into_iter()
                            .flat_map(|v| match v {
                                Value::Array(arr) => arr,
                                _ => vec![],
                            })
                            .collect()
                    }
                    PathSegment::Index(n) => {
                        current
                            .into_iter()
                            .filter_map(|v| {
                                if let Value::Array(arr) = v {
                                    arr.into_iter().nth(*n)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                };
            }

            Ok(PipelineInput::Json(current))
        }
        PipelineInput::Text(_) => Err(
            "path selection (e.g. '.foo[]') is only valid for JSON input; \
             use --format json or omit path stages for line/block input".to_string(),
        ),
    }
}

fn apply_del(input: PipelineInput, paths: &[Vec<String>]) -> Result<PipelineInput, String> {
    match input {
        PipelineInput::Json(values) => {
            let result = values
                .into_iter()
                .map(|v| {
                    let mut v = v;
                    for path in paths {
                        v = remove_at_path(v, path);
                    }
                    v
                })
                .collect();
            Ok(PipelineInput::Json(result))
        }
        PipelineInput::Text(_) => Err("del() is only valid for JSON input".to_string()),
    }
}

fn apply_where(
    input: PipelineInput,
    field: &[String],
    op: &WhereOp,
    value: &WhereValue,
) -> Result<PipelineInput, String> {
    match input {
        PipelineInput::Json(values) => {
            let result = values
                .into_iter()
                .filter(|v| matches_where(v, field, op, value))
                .collect();
            Ok(PipelineInput::Json(result))
        }
        PipelineInput::Text(_) => Err("where() is only valid for JSON input".to_string()),
    }
}

/// Evaluate a `where` predicate against a single JSON value.
fn matches_where(v: &Value, field: &[String], op: &WhereOp, rhs: &WhereValue) -> bool {
    let field_val = get_at_path(v, field);
    let field_str = match field_val {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b))   => b.to_string(),
        Some(Value::Null)      => "null".to_string(),
        _ => return false,
    };
    match (op, rhs) {
        (WhereOp::Eq, WhereValue::String(s))  => &field_str == s,
        (WhereOp::Ne, WhereValue::String(s))  => &field_str != s,
        (WhereOp::Eq, WhereValue::Number(n))  => field_str.parse::<f64>().ok().as_ref() == Some(n),
        (WhereOp::Ne, WhereValue::Number(n))  => field_str.parse::<f64>().ok().as_ref() != Some(n),
        (WhereOp::Contains,   WhereValue::String(s)) => field_str.contains(s.as_str()),
        (WhereOp::StartsWith, WhereValue::String(s)) => field_str.starts_with(s.as_str()),
        (WhereOp::EndsWith,   WhereValue::String(s)) => field_str.ends_with(s.as_str()),
        // Substring ops on numbers: coerce number to string and compare.
        (WhereOp::Contains,   WhereValue::Number(n)) => field_str.contains(&n.to_string().as_str()),
        (WhereOp::StartsWith, WhereValue::Number(n)) => field_str.starts_with(&n.to_string().as_str()),
        (WhereOp::EndsWith,   WhereValue::Number(n)) => field_str.ends_with(&n.to_string().as_str()),
    }
}

/// Walk a dotted path and return a reference to the value at that path, or
/// `None` if any segment is missing or the value is not an object.
fn get_at_path<'a>(mut v: &'a Value, path: &[String]) -> Option<&'a Value> {
    for seg in path {
        v = v.as_object()?.get(seg)?;
    }
    Some(v)
}

/// Recursively remove the field at `path` from `value`.
///
/// - Single-segment path: removes the key directly from the object.
/// - Multi-segment path: traverses nested objects and removes the terminal key.
/// - If any intermediate key is missing or the value is not an object: silently skip
///   (same behaviour as jq `del`).
fn remove_at_path(value: Value, path: &[String]) -> Value {
    if path.is_empty() {
        return value;
    }
    match value {
        Value::Object(mut map) => {
            if path.len() == 1 {
                map.remove(&path[0]);
            } else if let Some(nested) = map.remove(&path[0]) {
                let updated = remove_at_path(nested, &path[1..]);
                map.insert(path[0].clone(), updated);
            }
            // If the key doesn't exist, silently skip.
            Value::Object(map)
        }
        // Not an object at this level — silently skip.
        other => other,
    }
}

// ── Value-based group_by implementation ──────────────────────────────────────

/// Partition a list of JSON values by the string value of `field`.
///
/// Returns `(groups, ungrouped)` where `groups` is a list of `(field_value,
/// entries)` pairs sorted by descending count, and `ungrouped` contains values
/// that did not have the field (or had a non-string/non-scalar value).
pub fn partition_by_field(
    values: &[Value],
    field: &str,
) -> (Vec<(String, Vec<usize>)>, Vec<usize>) {
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut ungrouped: Vec<usize> = Vec::new();

    for (idx, value) in values.iter().enumerate() {
        let key = match value {
            Value::Object(map) => map.get(field).and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                Value::Null => Some("null".to_string()),
                _ => None,
            }),
            _ => None,
        };

        match key {
            Some(k) => groups.entry(k).or_default().push(idx),
            None => ungrouped.push(idx),
        }
    }

    let mut sorted: Vec<(String, Vec<usize>)> = groups.into_iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    (sorted, ungrouped)
}

// ── Known verb names (for CLI disambiguation) ─────────────────────────────────

/// Returns true if `s` is a pipeline verb name (used for CLI disambiguation).
pub fn is_verb_name(s: &str) -> bool {
    matches!(
        s,
        "summarize" | "similar" | "patterns" | "outliers"
            | "schemas" | "subtree" | "del" | "where"
            | "group_by" | "label" | "top"
    )
}
