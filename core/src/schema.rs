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
    /// Extract schema signature from a JSON value
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Object(map) => {
                let mut fields: Vec<String> = map.keys().cloned().collect();
                fields.sort();

                let field_types: BTreeMap<String, JsonType> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), JsonType::from_value(v)))
                    .collect();

                Some(SchemaSignature {
                    fields,
                    field_types,
                })
            }
            _ => None, // Only extract schema from objects
        }
    }

    /// Calculate similarity to another schema (0.0 = completely different, 1.0 = identical)
    pub fn similarity(&self, other: &SchemaSignature) -> f64 {
        let all_fields: std::collections::HashSet<_> =
            self.fields.iter().chain(other.fields.iter()).collect();

        if all_fields.is_empty() {
            return 1.0;
        }

        let mut matching = 0;
        let total = all_fields.len();

        for field in all_fields {
            let self_type = self.field_types.get(field);
            let other_type = other.field_types.get(field);

            match (self_type, other_type) {
                (Some(t1), Some(t2)) if t1 == t2 => matching += 1,
                (Some(_), Some(_)) => {}, // Field exists but different type
                _ => {},                   // Field missing in one schema
            }
        }

        matching as f64 / total as f64
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

    /// Get a human-readable description of the schema
    pub fn description(&self) -> String {
        let field_list: Vec<String> = self
            .field_types
            .iter()
            .map(|(name, typ)| format!("{}: {}", name, typ.as_str()))
            .collect();

        format!("{{ {} }}", field_list.join(", "))
    }
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
