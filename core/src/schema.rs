//! JSON schema extraction and analysis
//!
//! Extracts structural signatures from JSON objects to identify
//! schema patterns and variations.

use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// Represents the schema signature of a JSON object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaSignature {
    /// Fields present in the object (sorted for consistency)
    pub fields: Vec<String>,
    /// Field name -> type mapping
    pub field_types: BTreeMap<String, JsonType>,
    /// Nested schemas for Object/Array fields (populated when depth > 0).
    /// For an Object field, stores its schema. For an Array field, stores the
    /// representative element schema (first element's schema, if objects).
    pub nested: BTreeMap<String, SchemaSignature>,
}

/// Simplified JSON type representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JsonType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl SchemaSignature {
    /// Extract schema signature from a JSON value (flat, depth=0).
    pub fn from_value(value: &Value) -> Option<Self> {
        Self::from_value_with_depth(value, 0)
    }

    /// Extract schema signature, recursing into nested objects up to `depth` levels.
    /// depth=0 gives the same flat result as `from_value`.
    pub fn from_value_with_depth(value: &Value, depth: usize) -> Option<Self> {
        match value {
            Value::Object(map) => {
                let mut fields: Vec<String> = map.keys().cloned().collect();
                fields.sort();

                let field_types: BTreeMap<String, JsonType> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), JsonType::from_value(v)))
                    .collect();

                let mut nested = BTreeMap::new();
                if depth > 0 {
                    for (k, v) in map.iter() {
                        match v {
                            Value::Object(_) => {
                                if let Some(sub) = Self::from_value_with_depth(v, depth - 1) {
                                    nested.insert(k.clone(), sub);
                                }
                            }
                            Value::Array(arr) if !arr.is_empty() => {
                                if let Some(elem) = detect_array_element_schema(arr, depth - 1) {
                                    nested.insert(k.clone(), elem);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                Some(SchemaSignature { fields, field_types, nested })
            }
            _ => None,
        }
    }

    /// Calculate similarity to another schema (0.0 = completely different, 1.0 = identical).
    ///
    /// When both schemas have nested information for an Object/Array field, the
    /// field's contribution is weighted by the recursive similarity of those nested
    /// schemas rather than being a hard 0-or-1 match.
    pub fn similarity(&self, other: &SchemaSignature) -> f64 {
        let all_fields: std::collections::HashSet<_> =
            self.fields.iter().chain(other.fields.iter()).collect();

        if all_fields.is_empty() {
            return 1.0;
        }

        let mut score = 0.0f64;
        let total = all_fields.len();

        for field in &all_fields {
            let self_type = self.field_types.get(*field);
            let other_type = other.field_types.get(*field);

            if let (Some(t1), Some(t2)) = (self_type, other_type) {
                if t1 == t2 {
                    // For Object/Array fields, blend in recursive similarity when available.
                    let field_score = match t1 {
                        JsonType::Object | JsonType::Array => {
                            match (self.nested.get(*field), other.nested.get(*field)) {
                                (Some(s1), Some(s2)) => s1.similarity(s2),
                                // Only one side has nested info: fall back to full credit.
                                _ => 1.0,
                            }
                        }
                        _ => 1.0,
                    };
                    score += field_score;
                }
                // type mismatch: 0 contribution
            }
            // field missing in one schema: 0 contribution
        }

        score / total as f64
    }

    /// Get fields that are in this schema but not in another
    pub fn extra_fields(&self, other: &SchemaSignature) -> Vec<String> {
        self.fields
            .iter()
            .filter(|f| !other.fields.contains(f))
            .cloned()
            .collect()
    }

    /// Get fields that are in another schema but not in this one
    pub fn missing_fields(&self, other: &SchemaSignature) -> Vec<String> {
        other.extra_fields(self)
    }

    /// Get a human-readable description of the schema.
    /// Nested schemas are shown inline: `{ user: { id: number, name: string } }`.
    pub fn description(&self) -> String {
        let field_list: Vec<String> = self
            .field_types
            .iter()
            .map(|(name, typ)| {
                if let Some(nested) = self.nested.get(name) {
                    format!("{}: {}", name, nested.description())
                } else {
                    format!("{}: {}", name, typ.as_str())
                }
            })
            .collect();

        format!("{{ {} }}", field_list.join(", "))
    }
}

/// Pick a representative element schema for a JSON array.
/// Returns the schema of the first Object element (sufficient for homogeneous arrays).
fn detect_array_element_schema(arr: &[Value], depth: usize) -> Option<SchemaSignature> {
    arr.iter()
        .find_map(|v| SchemaSignature::from_value_with_depth(v, depth))
}

impl JsonType {
    /// Determine type from a JSON value
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => JsonType::Null,
            Value::Bool(_) => JsonType::Bool,
            Value::Number(_) => JsonType::Number,
            Value::String(_) => JsonType::String,
            Value::Array(_) => JsonType::Array,
            Value::Object(_) => JsonType::Object,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            JsonType::Null => "null",
            JsonType::Bool => "bool",
            JsonType::Number => "number",
            JsonType::String => "string",
            JsonType::Array => "array",
            JsonType::Object => "object",
        }
    }
}

/// Extract sample values from a JSON object for specific fields
pub fn extract_sample_values(value: &Value, fields: &[String]) -> HashMap<String, Vec<String>> {
    let mut samples: HashMap<String, Vec<String>> = HashMap::new();

    if let Value::Object(map) = value {
        for field in fields {
            if let Some(val) = map.get(field) {
                let sample = match val {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    Value::Array(_) => "[...]".to_string(),
                    Value::Object(_) => "{...}".to_string(),
                };

                samples.entry(field.clone()).or_default().push(sample);
            }
        }
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_extraction_simple() {
        let obj = json!({
            "name": "Alice",
            "age": 30,
            "active": true
        });

        let schema = SchemaSignature::from_value(&obj).unwrap();

        assert_eq!(schema.fields, vec!["active", "age", "name"]);
        assert_eq!(schema.field_types.get("name"), Some(&JsonType::String));
        assert_eq!(schema.field_types.get("age"), Some(&JsonType::Number));
        assert_eq!(schema.field_types.get("active"), Some(&JsonType::Bool));
    }

    #[test]
    fn test_schema_extraction_non_object() {
        let value = json!("string");
        assert!(SchemaSignature::from_value(&value).is_none());

        let value = json!(42);
        assert!(SchemaSignature::from_value(&value).is_none());
    }

    #[test]
    fn test_schema_similarity_identical() {
        let schema1 = SchemaSignature::from_value(&json!({
            "name": "Alice",
            "age": 30
        }))
        .unwrap();

        let schema2 = SchemaSignature::from_value(&json!({
            "name": "Bob",
            "age": 25
        }))
        .unwrap();

        assert_eq!(schema1.similarity(&schema2), 1.0);
    }

    #[test]
    fn test_schema_similarity_partial() {
        let schema1 = SchemaSignature::from_value(&json!({
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        }))
        .unwrap();

        let schema2 = SchemaSignature::from_value(&json!({
            "name": "Bob",
            "age": 25
        }))
        .unwrap();

        // 2 matching fields out of 3 total fields = 0.666...
        let sim = schema1.similarity(&schema2);
        assert!((sim - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_schema_similarity_type_mismatch() {
        let schema1 = SchemaSignature::from_value(&json!({
            "id": 123
        }))
        .unwrap();

        let schema2 = SchemaSignature::from_value(&json!({
            "id": "abc"
        }))
        .unwrap();

        // Same field name but different type = 0.0
        assert_eq!(schema1.similarity(&schema2), 0.0);
    }

    #[test]
    fn test_extra_and_missing_fields() {
        let schema1 = SchemaSignature::from_value(&json!({
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        }))
        .unwrap();

        let schema2 = SchemaSignature::from_value(&json!({
            "name": "Bob",
            "city": "NYC"
        }))
        .unwrap();

        let extra = schema1.extra_fields(&schema2);
        assert!(extra.contains(&"age".to_string()));
        assert!(extra.contains(&"email".to_string()));

        let missing = schema1.missing_fields(&schema2);
        assert!(missing.contains(&"city".to_string()));
    }

    #[test]
    fn test_schema_description() {
        let schema = SchemaSignature::from_value(&json!({
            "name": "Alice",
            "age": 30,
            "active": true
        }))
        .unwrap();

        let desc = schema.description();
        assert!(desc.contains("name: string"));
        assert!(desc.contains("age: number"));
        assert!(desc.contains("active: bool"));
    }

    #[test]
    fn test_nested_schema_extraction() {
        let obj = json!({
            "type": "user_event",
            "data": { "id": 1, "name": "alice", "role": "member" },
            "meta": { "ts": "2024-01-01", "region": "us-east" }
        });

        let flat = SchemaSignature::from_value(&obj).unwrap();
        assert!(flat.nested.is_empty(), "depth=0 should produce no nested schemas");
        assert_eq!(flat.field_types.get("data"), Some(&JsonType::Object));

        let deep = SchemaSignature::from_value_with_depth(&obj, 1).unwrap();
        assert!(deep.nested.contains_key("data"), "depth=1 should extract nested schema for 'data'");
        assert!(deep.nested.contains_key("meta"), "depth=1 should extract nested schema for 'meta'");

        let data_schema = deep.nested.get("data").unwrap();
        assert_eq!(data_schema.field_types.get("id"), Some(&JsonType::Number));
        assert_eq!(data_schema.field_types.get("name"), Some(&JsonType::String));
        assert_eq!(data_schema.field_types.get("role"), Some(&JsonType::String));
    }

    #[test]
    fn test_nested_similarity_splits_envelope_pattern() {
        // All records share {type, data, meta} at the top level.
        // At depth=0 they all look identical (similarity=1.0).
        // At depth=1 the different data sub-schemas reduce similarity below 0.8.
        let user_event = json!({
            "type": "user_event",
            "data": { "id": 1, "name": "alice", "role": "member" },
            "meta": { "ts": "2024-01-01", "region": "us-east" }
        });
        let order_event = json!({
            "type": "order_event",
            "data": { "id": 1001, "amount": 49.99, "status": "complete" },
            "meta": { "ts": "2024-01-01", "region": "us-east" }
        });

        // Flat: identical top-level shape → similarity 1.0
        let u_flat = SchemaSignature::from_value(&user_event).unwrap();
        let o_flat = SchemaSignature::from_value(&order_event).unwrap();
        assert_eq!(u_flat.similarity(&o_flat), 1.0,
            "flat schemas should be identical for envelope pattern");

        // Depth=1: data sub-schema differs (name+role vs amount+status) →
        // data field contributes <1, dragging overall similarity below 0.8
        let u_deep = SchemaSignature::from_value_with_depth(&user_event, 1).unwrap();
        let o_deep = SchemaSignature::from_value_with_depth(&order_event, 1).unwrap();
        let sim = u_deep.similarity(&o_deep);
        assert!(
            sim < 0.8,
            "depth-1 similarity for user vs order event should be <0.8, got {sim:.3}"
        );
    }

    #[test]
    fn test_nested_description() {
        let obj = json!({
            "user": { "id": 1, "name": "alice" }
        });
        let deep = SchemaSignature::from_value_with_depth(&obj, 1).unwrap();
        let desc = deep.description();
        // Should show inline nested form, not "user: object"
        assert!(desc.contains("user: {"), "description should show nested schema inline, got: {desc}");
        assert!(desc.contains("id: number"), "nested fields should be visible");
        assert!(desc.contains("name: string"), "nested fields should be visible");
    }

    #[test]
    fn test_array_element_schema_detected() {
        let obj = json!({
            "items": [
                { "id": 1, "price": 9.99 },
                { "id": 2, "price": 19.99 }
            ]
        });
        let deep = SchemaSignature::from_value_with_depth(&obj, 1).unwrap();
        assert!(deep.nested.contains_key("items"),
            "depth=1 should detect element schema for array field 'items'");
        let elem = deep.nested.get("items").unwrap();
        assert_eq!(elem.field_types.get("id"), Some(&JsonType::Number));
        assert_eq!(elem.field_types.get("price"), Some(&JsonType::Number));
    }

    #[test]
    fn test_sample_value_extraction() {
        let obj = json!({
            "name": "Alice",
            "age": 30,
            "tags": ["user", "premium"],
            "metadata": {"role": "admin"}
        });

        let fields = vec!["name".to_string(), "age".to_string(), "tags".to_string()];
        let samples = extract_sample_values(&obj, &fields);

        assert_eq!(samples.get("name"), Some(&vec!["Alice".to_string()]));
        assert_eq!(samples.get("age"), Some(&vec!["30".to_string()]));
        assert_eq!(samples.get("tags"), Some(&vec!["[...]".to_string()]));
    }
}
