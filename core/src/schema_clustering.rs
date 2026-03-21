//! Schema-based clustering for JSON data
//!
//! Groups JSON objects by their structural similarity.

use crate::metadata::{AlgorithmMetadata, InputType, ParamDefault, ParamRange, ParamType, Parameter};
use crate::schema::{SchemaSignature, extract_sample_values};
use serde_json::Value;
use std::collections::HashMap;

/// Schema cluster - a group of entries with similar structure
#[derive(Debug, Clone)]
pub struct SchemaCluster {
    /// Representative schema for this cluster
    pub schema: SchemaSignature,
    /// Indices of entries belonging to this cluster
    pub entry_indices: Vec<usize>,
    /// Sample values from entries in this cluster
    pub sample_values: HashMap<String, Vec<String>>,
}

/// Schema-based clustering algorithm
pub struct SchemaClusterer {
    /// Similarity threshold (0.0-1.0) for grouping schemas
    threshold: f64,
    /// Clusters found
    clusters: Vec<SchemaCluster>,
}

impl SchemaClusterer {
    /// Metadata describing this algorithm
    pub const METADATA: AlgorithmMetadata = AlgorithmMetadata {
        name: "schema",
        aliases: &["json"],
        description: "Groups JSON objects by structural similarity (matching field names and types)",
        best_for: "JSON data with varying schemas, API responses, configuration files",
        parameters: &[Parameter {
            name: "threshold",
            type_info: ParamType::Float,
            default: ParamDefault::Float(0.8),
            range: Some(ParamRange::Float { min: 0.0, max: 1.0 }),
            description: "Fraction of fields that must match (1.0 = exact schema match, 0.8 = 80% field overlap)",
            special_values: &[
                (1.0, "exact match only"),
                (0.8, "allow 20% field difference"),
            ],
        }],
        input_types: &[InputType::JsonArray, InputType::JsonMap, InputType::JsonNested],
    };

    /// Create a new schema clusterer with given similarity threshold
    pub fn new(threshold: f64) -> Self {
        SchemaClusterer {
            threshold,
            clusters: Vec::new(),
        }
    }

    /// Process JSON values and cluster them by schema
    pub fn process(&mut self, values: &[Value]) {
        let mut schemas: Vec<(usize, SchemaSignature)> = Vec::new();

        // Extract schemas from all values
        for (idx, value) in values.iter().enumerate() {
            if let Some(schema) = SchemaSignature::from_value(value) {
                schemas.push((idx, schema));
            }
        }

        // Greedy clustering: first schema becomes exemplar
        for (idx, schema) in schemas {
            let mut found_cluster = false;

            // Try to find a matching cluster
            for cluster in &mut self.clusters {
                let similarity = cluster.schema.similarity(&schema);
                if similarity >= self.threshold {
                    cluster.entry_indices.push(idx);

                    // Extract sample values from this entry
                    if let Some(value) = values.get(idx) {
                        let samples = extract_sample_values(value, &schema.fields);
                        for (field, vals) in samples {
                            cluster
                                .sample_values
                                .entry(field)
                                .or_default()
                                .extend(vals);
                        }
                    }

                    found_cluster = true;
                    break;
                }
            }

            // Create new cluster if no match found
            if !found_cluster {
                let mut sample_values = HashMap::new();

                if let Some(value) = values.get(idx) {
                    let samples = extract_sample_values(value, &schema.fields);
                    sample_values = samples;
                }

                self.clusters.push(SchemaCluster {
                    schema,
                    entry_indices: vec![idx],
                    sample_values,
                });
            }
        }

        // Sort clusters by size (descending)
        self.clusters
            .sort_by(|a, b| b.entry_indices.len().cmp(&a.entry_indices.len()));
    }

    /// Get the clusters
    pub fn get_clusters(&self) -> &[SchemaCluster] {
        &self.clusters
    }

    /// Get clusters that appear only once (potential outliers)
    pub fn get_singleton_clusters(&self) -> Vec<&SchemaCluster> {
        self.clusters
            .iter()
            .filter(|c| c.entry_indices.len() == 1)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_identical_schemas() {
        let values = vec![
            json!({"name": "Alice", "age": 30}),
            json!({"name": "Bob", "age": 25}),
            json!({"name": "Charlie", "age": 35}),
        ];

        let mut clusterer = SchemaClusterer::new(1.0);
        clusterer.process(&values);

        let clusters = clusterer.get_clusters();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entry_indices.len(), 3);
    }

    #[test]
    fn test_different_schemas() {
        let values = vec![
            json!({"name": "Alice", "age": 30}),
            json!({"email": "bob@example.com", "active": true}),
            json!({"id": 123, "role": "admin"}),
        ];

        let mut clusterer = SchemaClusterer::new(1.0);
        clusterer.process(&values);

        let clusters = clusterer.get_clusters();
        // Each should be in its own cluster (completely different schemas)
        assert_eq!(clusters.len(), 3);
        for cluster in clusters {
            assert_eq!(cluster.entry_indices.len(), 1);
        }
    }

    #[test]
    fn test_partial_match_with_threshold() {
        let values = vec![
            json!({"name": "Alice", "age": 30, "email": "alice@example.com"}),
            json!({"name": "Bob", "age": 25}), // Missing email field
            json!({"name": "Charlie", "age": 35, "email": "charlie@example.com"}),
        ];

        // Threshold 0.6 means at least 60% fields must match
        // "name" and "age" match = 2/3 = 0.666, so should cluster with first entry
        let mut clusterer = SchemaClusterer::new(0.6);
        clusterer.process(&values);

        let clusters = clusterer.get_clusters();

        // Should have 1 cluster (all entries have name+age, email is optional)
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entry_indices.len(), 3);
    }

    #[test]
    fn test_sample_value_collection() {
        let values = vec![
            json!({"event": "login", "user_id": 1}),
            json!({"event": "logout", "user_id": 2}),
            json!({"event": "login", "user_id": 3}),
        ];

        let mut clusterer = SchemaClusterer::new(1.0);
        clusterer.process(&values);

        let clusters = clusterer.get_clusters();
        assert_eq!(clusters.len(), 1);

        let samples = &clusters[0].sample_values;
        assert!(samples.contains_key("event"));
        assert!(samples.contains_key("user_id"));

        let event_values = &samples["event"];
        assert_eq!(event_values.len(), 3);
        assert!(event_values.contains(&"login".to_string()));
        assert!(event_values.contains(&"logout".to_string()));
    }

    #[test]
    fn test_singleton_detection() {
        let values = vec![
            json!({"name": "Alice"}),
            json!({"name": "Bob"}),
            json!({"email": "unique@example.com"}), // Outlier
        ];

        let mut clusterer = SchemaClusterer::new(1.0);
        clusterer.process(&values);

        let singletons = clusterer.get_singleton_clusters();
        assert_eq!(singletons.len(), 1);
        assert_eq!(singletons[0].entry_indices[0], 2);
    }

    #[test]
    fn test_sorting_by_size() {
        let values = vec![
            json!({"type": "A", "id": 1}),
            json!({"type": "B"}),
            json!({"type": "B"}),
            json!({"type": "B"}),
            json!({"name": "C"}),
            json!({"name": "C"}),
        ];

        let mut clusterer = SchemaClusterer::new(1.0);
        clusterer.process(&values);

        let clusters = clusterer.get_clusters();

        // Should have 3 clusters with different schemas, sorted by size
        assert_eq!(clusters.len(), 3);
        // Should be sorted by size descending
        assert!(clusters[0].entry_indices.len() >= clusters[1].entry_indices.len());
        assert!(clusters[1].entry_indices.len() >= clusters[2].entry_indices.len());
        // Largest cluster should have 3 entries (the "type: B" ones)
        assert_eq!(clusters[0].entry_indices.len(), 3);
    }

    #[test]
    fn test_non_object_values_ignored() {
        let values = vec![
            json!("string"),
            json!(42),
            json!({"name": "Alice"}),
            json!(null),
            json!({"name": "Bob"}),
        ];

        let mut clusterer = SchemaClusterer::new(1.0);
        clusterer.process(&values);

        let clusters = clusterer.get_clusters();

        // Only the two objects should be clustered
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entry_indices.len(), 2);
    }
}
