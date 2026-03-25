//! Subtree pattern detection for arbitrary JSON documents
//!
//! Walks the entire JSON document tree, collects every Object node with its
//! normalized path, clusters them by schema similarity, and reports which paths
//! each structural pattern appears at.
//!
//! Unlike the `schema` algorithm (which processes a flat collection of records),
//! `subtree` accepts a single document of any shape and finds patterns wherever
//! they occur — e.g., the same `{id, name, email}` shape appearing at
//! `$.users[*]`, `$.team.members[*]`, and `$.config.owner`.

use crate::metadata::{AlgorithmMetadata, InputType, ParamDefault, ParamRange, ParamType, Parameter};
use crate::schema::SchemaSignature;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

/// A structural pattern found at one or more paths in the document.
#[derive(Debug, Clone)]
pub struct SubtreePattern {
    /// Representative schema for this pattern
    pub schema: SchemaSignature,
    /// Total number of individual objects that matched this pattern
    pub count: usize,
    /// Deduplicated, sorted normalized paths where this pattern appears
    /// (e.g. `["$.config.owner", "$.team.members[*]", "$.users[*]"]`)
    pub paths: Vec<String>,
    /// Up to 5 sample values per field
    pub sample_values: HashMap<String, Vec<String>>,
}

/// Algorithm that finds recurring structural patterns in an arbitrary JSON document.
pub struct SubtreeFinder {
    threshold: f64,
    patterns: Vec<SubtreePattern>,
    /// Singleton objects that did not match any repeated pattern
    pub singletons: Vec<(String, Value)>,
}

impl SubtreeFinder {
    pub const METADATA: AlgorithmMetadata = AlgorithmMetadata {
        name: "subtree",
        aliases: &["json-tree", "tree"],
        description: "Finds recurring structural patterns anywhere in a JSON document, reporting the paths where each pattern appears",
        best_for: "Single JSON documents (API responses, configs, exports) where the same object shape appears at multiple locations",
        parameters: &[Parameter {
            name: "threshold",
            type_info: ParamType::Float,
            default: ParamDefault::Float(0.8),
            range: Some(ParamRange::Float { min: 0.0, max: 1.0 }),
            description: "Fraction of fields that must match for two objects to be grouped as the same pattern",
            special_values: &[
                (1.0, "exact match only"),
                (0.8, "allow 20% field difference"),
            ],
        }],
        input_types: &[InputType::JsonNested, InputType::JsonArray, InputType::JsonMap],
    };

    pub fn new(threshold: f64) -> Self {
        SubtreeFinder {
            threshold,
            patterns: Vec::new(),
            singletons: Vec::new(),
        }
    }

    /// Walk `root` and cluster every Object node by schema similarity.
    pub fn process(&mut self, root: &Value) {
        // Step 1: collect all Object nodes with their normalized paths.
        let mut occurrences: Vec<(String, Value)> = Vec::new();
        collect_objects(root, "$".to_string(), &mut occurrences);

        if occurrences.is_empty() {
            return;
        }

        // Step 2: extract flat schemas (depth=0; we cluster by top-level shape here —
        // the path information already disambiguates location).
        let schemas: Vec<Option<SchemaSignature>> = occurrences
            .iter()
            .map(|(_, v)| SchemaSignature::from_value(v))
            .collect();

        // Step 3: greedy clustering, same algorithm as SchemaClusterer.
        // Each cluster: (representative schema, list of occurrence indices).
        let mut clusters: Vec<(SchemaSignature, Vec<usize>)> = Vec::new();

        for (idx, schema_opt) in schemas.iter().enumerate() {
            let Some(schema) = schema_opt else { continue };

            let mut found = false;
            for (rep, indices) in &mut clusters {
                if rep.similarity(schema) >= self.threshold {
                    indices.push(idx);
                    found = true;
                    break;
                }
            }
            if !found {
                clusters.push((schema.clone(), vec![idx]));
            }
        }

        // Step 4: sort by count descending, then build output structures.
        clusters.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (rep_schema, indices) in clusters {
            let count = indices.len();

            // Collect normalized paths (deduplicated via BTreeSet).
            let mut path_set: BTreeSet<String> = BTreeSet::new();
            let mut sample_values: HashMap<String, Vec<String>> = HashMap::new();

            for &idx in &indices {
                let (path, value) = &occurrences[idx];
                path_set.insert(path.clone());
                collect_samples(value, &rep_schema.fields, &mut sample_values);
            }

            let paths: Vec<String> = path_set.into_iter().collect();

            if count == 1 {
                let (path, value) = occurrences[indices[0]].clone();
                self.singletons.push((path, value));
            } else {
                self.patterns.push(SubtreePattern {
                    schema: rep_schema,
                    count,
                    paths,
                    sample_values,
                });
            }
        }
    }

    pub fn get_patterns(&self) -> &[SubtreePattern] {
        &self.patterns
    }

    pub fn get_singletons(&self) -> &[(String, Value)] {
        &self.singletons
    }
}

/// Recursively collect every Object node with its normalized path.
/// Array indices are replaced with `[*]` so that all elements of an array
/// share a single representative path.
fn collect_objects(value: &Value, path: String, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(map) => {
            out.push((path.clone(), value.clone()));
            for (key, child) in map {
                collect_objects(child, format!("{}.{}", path, key), out);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_objects(item, format!("{}[*]", path), out);
            }
        }
        _ => {}
    }
}

/// Populate `samples` with up to 5 values per field from a single object.
fn collect_samples(
    value: &Value,
    fields: &[String],
    samples: &mut HashMap<String, Vec<String>>,
) {
    let Value::Object(map) = value else { return };
    for field in fields {
        if let Some(v) = map.get(field) {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                Value::Array(_) => "[...]".to_string(),
                Value::Object(_) => "{...}".to_string(),
            };
            let entry = samples.entry(field.clone()).or_default();
            if entry.len() < 5 {
                entry.push(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_path_normalization() {
        let mut out = Vec::new();
        let doc = json!({
            "users": [{"id": 1}, {"id": 2}],
            "config": {"owner": {"id": 99}}
        });
        collect_objects(&doc, "$".to_string(), &mut out);

        let paths: Vec<&str> = out.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"$"), "root should be collected");
        assert!(paths.contains(&"$.users[*]"), "array elements should use [*]");
        assert!(paths.contains(&"$.config.owner"), "nested object path should be correct");
        // Array indices should NOT appear
        assert!(!paths.iter().any(|p| p.contains("[0]") || p.contains("[1]")));
    }

    #[test]
    fn test_finds_pattern_at_multiple_paths() {
        // The {id, name} shape appears at both $.users[*] and $.admins[*].
        let doc = json!({
            "users":  [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}],
            "admins": [{"id": 10, "name": "carol"}, {"id": 11, "name": "dave"}]
        });

        let mut finder = SubtreeFinder::new(0.8);
        finder.process(&doc);

        // There should be a pattern for {id, name} covering both paths.
        let user_pattern = finder.get_patterns().iter().find(|p| {
            p.paths.iter().any(|path| path.contains("users"))
                && p.paths.iter().any(|path| path.contains("admins"))
        });
        assert!(user_pattern.is_some(), "expected {{id,name}} pattern spanning users and admins");
        let p = user_pattern.unwrap();
        assert_eq!(p.count, 4, "four objects total");
        assert!(p.paths.len() >= 2, "pattern should appear at ≥2 paths");
    }

    #[test]
    fn test_distinct_schemas_stay_separate() {
        let doc = json!({
            "users":  [{"id": 1, "name": "alice"}],
            "orders": [{"order_id": 1001, "amount": 49.99}]
        });

        let mut finder = SubtreeFinder::new(0.8);
        finder.process(&doc);

        // The two shapes are completely different → should not be merged.
        let patterns = finder.get_patterns();
        // Both have count=1, so they'll be in singletons, not patterns.
        // Either way, they should not be in the same group.
        for p in patterns {
            let has_users  = p.paths.iter().any(|path| path.contains("users"));
            let has_orders = p.paths.iter().any(|path| path.contains("orders"));
            assert!(
                !(has_users && has_orders),
                "user and order schemas should not be clustered together"
            );
        }
    }

    #[test]
    fn test_sample_values_collected() {
        let doc = json!({
            "users": [
                {"id": 1, "name": "alice"},
                {"id": 2, "name": "bob"},
                {"id": 3, "name": "charlie"}
            ]
        });

        let mut finder = SubtreeFinder::new(0.8);
        finder.process(&doc);

        let pattern = finder.get_patterns().iter()
            .find(|p| p.paths.iter().any(|path| path.contains("users")));
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert!(p.sample_values.contains_key("name"), "sample values should include 'name'");
        assert!(p.sample_values["name"].contains(&"alice".to_string()));
    }

    #[test]
    fn test_singleton_objects_not_in_patterns() {
        let doc = json!({
            "users": [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}],
            "unique": {"x": 1, "y": 2, "z": 3}  // unique shape, count=1
        });

        let mut finder = SubtreeFinder::new(1.0); // strict threshold
        finder.process(&doc);

        // The unique object should not appear in patterns
        for p in finder.get_patterns() {
            assert!(
                !p.paths.iter().any(|path| path.contains("unique")),
                "singleton should not appear in patterns"
            );
        }
    }

    #[test]
    fn test_empty_document() {
        let mut finder = SubtreeFinder::new(0.8);
        finder.process(&json!({}));
        // Root object is collected but is a singleton → no repeated patterns
        assert!(finder.get_patterns().is_empty());
    }
}
