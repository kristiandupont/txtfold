//! Structured output format for analysis results

use crate::clustering::{Cluster, EditDistanceClusterer};
use crate::entry::Entry;
use crate::ngram::NgramOutlierDetector;
use crate::schema_clustering::SchemaClusterer;
use crate::template::{TemplateExtractor, TemplateGroup};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Complete analysis output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AnalysisOutput {
    /// Metadata about the analysis
    pub metadata: AnalysisMetadata,
    /// Summary statistics
    pub summary: AnalysisSummary,
    /// Algorithm-specific results
    pub results: AlgorithmResults,
}

/// Algorithm-specific output formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlgorithmResults {
    /// Pattern grouping with optional outliers (template extraction, clustering)
    Grouped {
        groups: Vec<GroupOutput>,
        outliers: Vec<OutlierOutput>,
    },
    /// Outlier-focused with baseline information (n-gram analysis)
    OutlierFocused {
        baseline: BaselineOutput,
        outliers: Vec<OutlierOutput>,
    },
    /// Schema-based grouping (JSON/structured data)
    SchemaGrouped {
        schemas: Vec<SchemaGroupOutput>,
        outliers: Vec<OutlierOutput>,
    },
}

/// Baseline information for outlier-focused algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct BaselineOutput {
    /// Description of the baseline/common patterns
    pub description: String,
    /// Number of entries considered "normal"
    pub normal_count: usize,
    /// Percentage of entries considered "normal"
    pub normal_percentage: f64,
    /// Top common features (e.g., n-grams, tokens)
    pub common_features: Vec<String>,
    /// Threshold used for outlier detection (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<ThresholdInfo>,
}

/// Information about threshold used for outlier detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ThresholdInfo {
    /// The threshold value used
    pub value: f64,
    /// Whether this was auto-detected
    pub auto_detected: bool,
    /// Score statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_stats: Option<ScoreStatsOutput>,
}

/// Score statistics for n-gram analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ScoreStatsOutput {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
}

/// Metadata about the analysis run
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AnalysisMetadata {
    /// Input file name (optional)
    pub input_file: Option<String>,
    /// Total number of entries processed
    pub total_entries: usize,
    /// Algorithm used
    pub algorithm: String,
    /// Compression ratio (output size / input size)
    pub compression_ratio: f64,
}

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AnalysisSummary {
    /// Number of unique patterns found
    pub unique_patterns: usize,
    /// Number of outliers detected
    pub outliers: usize,
    /// Size of largest cluster
    pub largest_cluster: usize,
}

/// A single pattern group in the output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct GroupOutput {
    /// Group identifier
    pub id: String,
    /// Derived human-readable name from pattern
    pub name: String,
    /// Human-readable pattern
    pub pattern: String,
    /// Number of entries matching this pattern
    pub count: usize,
    /// Percentage of total entries
    pub percentage: f64,
    /// Sample entries from this group
    pub samples: Vec<SampleEntry>,
    /// Line number ranges where this pattern appears
    pub line_ranges: Vec<(usize, usize)>,
}

/// A sample entry from a group
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SampleEntry {
    /// The actual content
    pub content: String,
    /// Line numbers of sample instances
    pub line_numbers: Vec<usize>,
    /// Variable values found in samples
    pub variable_values: HashMap<String, Vec<String>>,
}

/// An outlier entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OutlierOutput {
    /// Outlier identifier
    pub id: String,
    /// The content
    pub content: String,
    /// Line number
    pub line_number: usize,
    /// Reason for being flagged as outlier
    pub reason: String,
    /// Outlier score (lower = more unusual)
    pub score: f64,
}

/// A schema group (for JSON/structured data)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SchemaGroupOutput {
    /// Group identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Schema description (field names and types)
    pub schema_description: String,
    /// Fields present in this schema
    pub fields: Vec<String>,
    /// Number of entries with this schema
    pub count: usize,
    /// Percentage of total entries
    pub percentage: f64,
    /// Sample field values
    pub sample_values: HashMap<String, Vec<String>>,
    /// Entry indices (which entries have this schema)
    pub entry_indices: Vec<usize>,
}

/// Builder for creating AnalysisOutput from extraction results
pub struct OutputBuilder {
    entries: Vec<Entry>,
    input_file: Option<String>,
}

impl OutputBuilder {
    /// Create a new output builder
    pub fn new(entries: Vec<Entry>) -> Self {
        OutputBuilder {
            entries,
            input_file: None,
        }
    }

    /// Set the input filename
    pub fn with_input_file(mut self, filename: String) -> Self {
        self.input_file = Some(filename);
        self
    }

    /// Build the output from a template extractor (backwards compatible alias)
    pub fn build(self, extractor: &TemplateExtractor) -> AnalysisOutput {
        self.build_from_templates(extractor)
    }

    /// Build the output from a template extractor
    pub fn build_from_templates(self, extractor: &TemplateExtractor) -> AnalysisOutput {
        let total_entries = self.entries.len();
        let groups = extractor.get_groups();
        let unique_patterns = groups.len();

        // Convert template groups to output format
        let mut group_outputs = Vec::new();
        let mut largest_cluster = 0;

        for (idx, group) in groups.iter().enumerate() {
            let count = group.count();
            if count > largest_cluster {
                largest_cluster = count;
            }

            let percentage = if total_entries > 0 {
                (count as f64 / total_entries as f64) * 100.0
            } else {
                0.0
            };

            let samples = self.build_samples(group);
            let line_ranges = self.build_line_ranges(group);

            group_outputs.push(GroupOutput {
                id: format!("group_{}", idx),
                name: group.derive_name(),
                pattern: group.template.pattern.clone(),
                count,
                percentage,
                samples,
                line_ranges,
            });
        }

        // Detect outliers (groups with count = 1)
        let outliers = self.detect_outliers(&groups);

        // Calculate compression ratio
        let compression_ratio = self.calculate_compression_ratio(&group_outputs);

        AnalysisOutput {
            metadata: AnalysisMetadata {
                input_file: self.input_file,
                total_entries,
                algorithm: "template_extraction".to_string(),
                compression_ratio,
            },
            summary: AnalysisSummary {
                unique_patterns,
                outliers: outliers.len(),
                largest_cluster,
            },
            results: AlgorithmResults::Grouped {
                groups: group_outputs,
                outliers,
            },
        }
    }

    /// Build the output from an n-gram outlier detector
    pub fn build_from_ngrams(self, detector: &NgramOutlierDetector) -> AnalysisOutput {
        let total_entries = self.entries.len();
        let outlier_indices = detector.get_outliers();

        // Build outlier outputs
        let mut outliers = Vec::new();
        for (outlier_idx, &entry_idx) in outlier_indices.iter().enumerate() {
            if let Some(entry) = self.entries.get(entry_idx) {
                let line_number = entry
                    .metadata
                    .as_ref()
                    .and_then(|m| m.line_numbers.first().copied())
                    .unwrap_or(0);

                let score = detector.get_score(entry_idx).unwrap_or(0.0);

                outliers.push(OutlierOutput {
                    id: format!("outlier_{}", outlier_idx),
                    content: entry.as_single_string(),
                    line_number,
                    reason: "rare_ngrams".to_string(),
                    score,
                });
            }
        }

        // Build baseline info
        let normal_count = detector.get_normal_count(total_entries);
        let normal_percentage = detector.get_normal_percentage(total_entries);
        let top_ngrams = detector.get_top_ngrams(10);
        let common_features: Vec<String> = top_ngrams
            .iter()
            .map(|(ng, count)| format!("'{}' ({}x)", ng, count))
            .collect();

        // Build threshold info
        let stats = detector.get_score_stats();
        let threshold_info = ThresholdInfo {
            value: detector.get_effective_threshold(),
            auto_detected: detector.is_auto_threshold(),
            score_stats: Some(ScoreStatsOutput {
                min: stats.min,
                max: stats.max,
                mean: stats.mean,
                median: stats.median,
            }),
        };

        let baseline = BaselineOutput {
            description: format!(
                "Most entries share common patterns. {} entries analyzed.",
                total_entries
            ),
            normal_count,
            normal_percentage,
            common_features: common_features.clone(),
            threshold: Some(threshold_info),
        };

        // Calculate compression ratio
        let compressed_size = baseline.description.len()
            + common_features.iter().map(|s| s.len()).sum::<usize>()
            + outliers.iter().map(|o| o.content.len()).sum::<usize>();

        let original_size: usize = self
            .entries
            .iter()
            .map(|e| e.as_single_string().len())
            .sum();

        let compression_ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            0.0
        };

        AnalysisOutput {
            metadata: AnalysisMetadata {
                input_file: self.input_file,
                total_entries,
                algorithm: "ngram_outlier_detection".to_string(),
                compression_ratio,
            },
            summary: AnalysisSummary {
                unique_patterns: 0, // N-gram doesn't produce discrete patterns
                outliers: outliers.len(),
                largest_cluster: normal_count,
            },
            results: AlgorithmResults::OutlierFocused { baseline, outliers },
        }
    }

    /// Build the output from an edit distance clusterer
    pub fn build_from_clusters(self, clusterer: &EditDistanceClusterer) -> AnalysisOutput {
        let total_entries = self.entries.len();
        let clusters = clusterer.get_clusters();
        let unique_patterns = clusters.len();

        // Convert clusters to output format
        let mut group_outputs = Vec::new();
        let mut largest_cluster = 0;

        for (idx, cluster) in clusters.iter().enumerate() {
            let count = cluster.entry_indices.len();
            if count > largest_cluster {
                largest_cluster = count;
            }

            let percentage = if total_entries > 0 {
                (count as f64 / total_entries as f64) * 100.0
            } else {
                0.0
            };

            let samples = self.build_cluster_samples(cluster);
            let line_ranges = self.build_cluster_line_ranges(cluster);

            // Derive name from exemplar - extract meaningful part
            let first_line = cluster
                .exemplar
                .lines()
                .next()
                .unwrap_or(&cluster.exemplar);

            // Try to extract content after timestamp and log level
            // Example: "[2024-01-15 10:00:00] ERROR Something happened" -> "Something happened"
            let name = if let Some(after_bracket) = first_line.split(']').nth(1) {
                // Remove log level (ERROR, INFO, etc.)
                let words: Vec<&str> = after_bracket
                    .trim()
                    .split_whitespace()
                    .skip_while(|w| {
                        matches!(
                            w.to_uppercase().as_str(),
                            "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE"
                        )
                    })
                    .collect();

                if words.is_empty() {
                    first_line.chars().take(60).collect()
                } else {
                    words.join(" ").chars().take(60).collect()
                }
            } else {
                first_line.chars().take(60).collect()
            };

            group_outputs.push(GroupOutput {
                id: format!("cluster_{}", idx),
                name,
                pattern: cluster.exemplar.clone(),
                count,
                percentage,
                samples,
                line_ranges,
            });
        }

        // Detect outliers (clusters with count = 1)
        let outliers = self.detect_cluster_outliers(clusters);

        // Calculate compression ratio
        let compression_ratio = self.calculate_compression_ratio(&group_outputs);

        AnalysisOutput {
            metadata: AnalysisMetadata {
                input_file: self.input_file,
                total_entries,
                algorithm: "edit_distance_clustering".to_string(),
                compression_ratio,
            },
            summary: AnalysisSummary {
                unique_patterns,
                outliers: outliers.len(),
                largest_cluster,
            },
            results: AlgorithmResults::Grouped {
                groups: group_outputs,
                outliers,
            },
        }
    }

    /// Build sample entries for a group
    fn build_samples(&self, group: &TemplateGroup) -> Vec<SampleEntry> {
        // For now, just take up to 3 samples
        let sample_indices: Vec<usize> = group.entry_indices.iter().take(3).copied().collect();

        let mut samples = Vec::new();

        if !sample_indices.is_empty() {
            // Get first sample's content
            if let Some(entry) = self.entries.get(sample_indices[0]) {
                let content = entry.as_single_string();

                // Collect line numbers from samples
                let line_numbers: Vec<usize> = sample_indices
                    .iter()
                    .filter_map(|&idx| {
                        self.entries.get(idx).and_then(|e| {
                            e.metadata
                                .as_ref()
                                .and_then(|m| m.line_numbers.first().copied())
                        })
                    })
                    .collect();

                // Build variable values map
                let mut variable_values = HashMap::new();
                for (var_idx, values) in &group.variable_samples {
                    let key = format!("var_{}", var_idx);
                    // Take up to 5 sample values
                    let sample_values: Vec<String> = values.iter().take(5).cloned().collect();
                    variable_values.insert(key, sample_values);
                }

                samples.push(SampleEntry {
                    content,
                    line_numbers,
                    variable_values,
                });
            }
        }

        samples
    }

    /// Build line ranges for a group
    fn build_line_ranges(&self, group: &TemplateGroup) -> Vec<(usize, usize)> {
        let mut line_numbers: Vec<usize> = group
            .entry_indices
            .iter()
            .filter_map(|&idx| {
                self.entries.get(idx).and_then(|e| {
                    e.metadata
                        .as_ref()
                        .and_then(|m| m.line_numbers.first().copied())
                })
            })
            .collect();

        if line_numbers.is_empty() {
            return Vec::new();
        }

        line_numbers.sort_unstable();

        // Build ranges from consecutive line numbers
        let mut ranges = Vec::new();
        let mut range_start = line_numbers[0];
        let mut range_end = line_numbers[0];

        for &line_num in &line_numbers[1..] {
            if line_num == range_end + 1 {
                range_end = line_num;
            } else {
                ranges.push((range_start, range_end));
                range_start = line_num;
                range_end = line_num;
            }
        }
        ranges.push((range_start, range_end));

        ranges
    }

    /// Detect outliers (entries that appear rarely)
    /// Build sample entries for a cluster
    fn build_cluster_samples(&self, cluster: &Cluster) -> Vec<SampleEntry> {
        // For clustering, show multiple actual entries to demonstrate variation
        // Take up to 3 different samples from the cluster
        let sample_indices: Vec<usize> = cluster.entry_indices.iter().take(3).copied().collect();

        let mut samples = Vec::new();

        for &idx in &sample_indices {
            if let Some(entry) = self.entries.get(idx) {
                let content = entry.as_single_string();
                let line_numbers = entry
                    .metadata
                    .as_ref()
                    .map(|m| vec![m.line_numbers.first().copied().unwrap_or(0)])
                    .unwrap_or_default();

                // No variable values for clustering (would need to compute diffs)
                let variable_values = HashMap::new();

                samples.push(SampleEntry {
                    content,
                    line_numbers,
                    variable_values,
                });
            }
        }

        samples
    }

    /// Build line ranges for a cluster
    fn build_cluster_line_ranges(&self, cluster: &Cluster) -> Vec<(usize, usize)> {
        let mut line_numbers = cluster.line_numbers.clone();

        if line_numbers.is_empty() {
            return Vec::new();
        }

        line_numbers.sort_unstable();

        // Build ranges from consecutive line numbers
        let mut ranges = Vec::new();
        let mut range_start = line_numbers[0];
        let mut range_end = line_numbers[0];

        for &line_num in &line_numbers[1..] {
            if line_num == range_end + 1 {
                range_end = line_num;
            } else {
                ranges.push((range_start, range_end));
                range_start = line_num;
                range_end = line_num;
            }
        }
        ranges.push((range_start, range_end));

        ranges
    }

    /// Detect outliers from clusters
    fn detect_cluster_outliers(&self, clusters: &[Cluster]) -> Vec<OutlierOutput> {
        let mut outliers = Vec::new();
        let mut outlier_count = 0;

        for cluster in clusters {
            if cluster.entry_indices.len() == 1 {
                // Single occurrence = outlier
                if let Some(&entry_idx) = cluster.entry_indices.first() {
                    if let Some(entry) = self.entries.get(entry_idx) {
                        let line_number = entry
                            .metadata
                            .as_ref()
                            .and_then(|m| m.line_numbers.first().copied())
                            .unwrap_or(0);

                        outliers.push(OutlierOutput {
                            id: format!("outlier_{}", outlier_count),
                            content: entry.as_single_string(),
                            line_number,
                            reason: "rare_pattern".to_string(),
                            score: 1.0 / self.entries.len() as f64,
                        });

                        outlier_count += 1;
                    }
                }
            }
        }

        outliers
    }

    fn detect_outliers(&self, groups: &[&TemplateGroup]) -> Vec<OutlierOutput> {
        let mut outliers = Vec::new();
        let mut outlier_count = 0;

        for group in groups {
            if group.count() == 1 {
                // Single occurrence = outlier
                if let Some(&entry_idx) = group.entry_indices.first() {
                    if let Some(entry) = self.entries.get(entry_idx) {
                        let line_number = entry
                            .metadata
                            .as_ref()
                            .and_then(|m| m.line_numbers.first().copied())
                            .unwrap_or(0);

                        outliers.push(OutlierOutput {
                            id: format!("outlier_{}", outlier_count),
                            content: entry.as_single_string(),
                            line_number,
                            reason: "rare_pattern".to_string(),
                            score: 1.0 / self.entries.len() as f64,
                        });

                        outlier_count += 1;
                    }
                }
            }
        }

        outliers
    }

    /// Calculate compression ratio
    fn calculate_compression_ratio(&self, groups: &[GroupOutput]) -> f64 {
        // Original size: sum of all entry contents
        let original_size: usize = self
            .entries
            .iter()
            .map(|e| e.as_single_string().len())
            .sum();

        if original_size == 0 {
            return 0.0;
        }

        // Compressed size: sum of pattern strings + sample data
        let compressed_size: usize = groups
            .iter()
            .map(|g| {
                // Pattern + count + samples
                g.pattern.len() + 8 + g.samples.iter().map(|s| s.content.len()).sum::<usize>()
            })
            .sum();

        compressed_size as f64 / original_size as f64
    }

    /// Build the output from a schema clusterer (for JSON data)
    pub fn build_from_schemas(
        self,
        clusterer: &SchemaClusterer,
        values: &[Value],
    ) -> AnalysisOutput {
        let total_entries = values.len();
        let clusters = clusterer.get_clusters();

        // Build schema group outputs
        let mut schema_outputs = Vec::new();
        for (idx, cluster) in clusters.iter().enumerate() {
            let count = cluster.entry_indices.len();
            let percentage = (count as f64 / total_entries as f64) * 100.0;

            // Generate a name from the schema (use first few fields or common pattern)
            let name = if cluster.schema.fields.is_empty() {
                format!("Schema {}", idx)
            } else if cluster.schema.fields.len() <= 3 {
                cluster.schema.fields.join(", ")
            } else {
                format!(
                    "{}, ... ({} fields)",
                    cluster.schema.fields[..2].join(", "),
                    cluster.schema.fields.len()
                )
            };

            // Limit sample values to first 5 unique per field
            let mut limited_samples: HashMap<String, Vec<String>> = HashMap::new();
            for (field, vals) in &cluster.sample_values {
                let mut unique: Vec<String> = vals.iter().cloned().collect();
                unique.sort();
                unique.dedup();
                unique.truncate(5);
                limited_samples.insert(field.clone(), unique);
            }

            schema_outputs.push(SchemaGroupOutput {
                id: format!("schema_{}", idx),
                name,
                schema_description: cluster.schema.description(),
                fields: cluster.schema.fields.clone(),
                count,
                percentage,
                sample_values: limited_samples,
                entry_indices: cluster.entry_indices.clone(),
            });
        }

        // Detect outliers (singleton clusters)
        let singletons = clusterer.get_singleton_clusters();
        let mut outliers = Vec::new();
        for (outlier_idx, cluster) in singletons.iter().enumerate() {
            if let Some(&entry_idx) = cluster.entry_indices.first() {
                if let Some(value) = values.get(entry_idx) {
                    let content = serde_json::to_string_pretty(value).unwrap_or_default();

                    outliers.push(OutlierOutput {
                        id: format!("outlier_{}", outlier_idx),
                        content,
                        line_number: entry_idx + 1,
                        reason: "unique_schema".to_string(),
                        score: 0.0,
                    });
                }
            }
        }

        // Calculate compression ratio
        let original_size: usize = values
            .iter()
            .map(|v| serde_json::to_string(v).unwrap_or_default().len())
            .sum();

        let compressed_size: usize = schema_outputs
            .iter()
            .map(|s| {
                s.schema_description.len()
                    + s.sample_values
                        .values()
                        .map(|vals| vals.iter().map(|v| v.len()).sum::<usize>())
                        .sum::<usize>()
            })
            .sum::<usize>()
            + outliers.iter().map(|o| o.content.len()).sum::<usize>();

        let compression_ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            0.0
        };

        AnalysisOutput {
            metadata: AnalysisMetadata {
                input_file: self.input_file,
                total_entries,
                algorithm: "schema_clustering".to_string(),
                compression_ratio,
            },
            summary: AnalysisSummary {
                unique_patterns: schema_outputs.len(),
                outliers: outliers.len(),
                largest_cluster: schema_outputs
                    .iter()
                    .map(|s| s.count)
                    .max()
                    .unwrap_or(0),
            },
            results: AlgorithmResults::SchemaGrouped {
                schemas: schema_outputs,
                outliers,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::TemplateExtractor;

    #[test]
    fn test_output_builder_basic() {
        let entries = vec![
            Entry::from_line("[2024-01-15] INFO User login".to_string(), 1),
            Entry::from_line("[2024-01-16] INFO User login".to_string(), 2),
            Entry::from_line("[2024-01-17] ERROR Connection failed".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries.clone()).build(&extractor);

        assert_eq!(output.metadata.total_entries, 3);
        assert_eq!(output.metadata.algorithm, "template_extraction");
        assert_eq!(output.summary.unique_patterns, 2);
        assert_eq!(output.summary.largest_cluster, 2);

        // Check that we have grouped results
        if let AlgorithmResults::Grouped { groups, .. } = &output.results {
            assert_eq!(groups.len(), 2);
        } else {
            panic!("Expected Grouped results");
        }
    }

    #[test]
    fn test_output_serialization() {
        let entries = vec![
            Entry::from_line("Request took 42 ms".to_string(), 1),
            Entry::from_line("Request took 100 ms".to_string(), 2),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries)
            .with_input_file("test.log".to_string())
            .build(&extractor);

        // Should serialize to JSON
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("test.log"));
        assert!(json.contains("template_extraction"));
        assert!(json.contains("Request took <NUM> ms"));

        // Should deserialize back
        let deserialized: AnalysisOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.input_file, Some("test.log".to_string()));
        assert_eq!(deserialized.metadata.total_entries, 2);
    }

    #[test]
    fn test_outlier_detection() {
        let entries = vec![
            Entry::from_line("INFO User login".to_string(), 1),
            Entry::from_line("INFO User login".to_string(), 2),
            Entry::from_line("INFO User login".to_string(), 3),
            Entry::from_line("ERROR Fatal exception".to_string(), 4),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);

        // Should detect the single ERROR as an outlier
        assert_eq!(output.summary.outliers, 1);

        if let AlgorithmResults::Grouped { outliers, .. } = &output.results {
            assert_eq!(outliers.len(), 1);
            assert_eq!(outliers[0].content, "ERROR Fatal exception");
            assert_eq!(outliers[0].line_number, 4);
            assert_eq!(outliers[0].reason, "rare_pattern");
        } else {
            panic!("Expected Grouped results");
        }
    }

    #[test]
    fn test_compression_ratio() {
        let entries = vec![
            Entry::from_line("Same message".to_string(), 1),
            Entry::from_line("Same message".to_string(), 2),
            Entry::from_line("Same message".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);

        // Compression ratio should be less than 1.0 (we've compressed)
        assert!(output.metadata.compression_ratio < 1.0);
        assert!(output.metadata.compression_ratio > 0.0);
    }

    #[test]
    fn test_group_percentages() {
        let entries = vec![
            Entry::from_line("Type A".to_string(), 1),
            Entry::from_line("Type A".to_string(), 2),
            Entry::from_line("Type A".to_string(), 3),
            Entry::from_line("Type B".to_string(), 4),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);

        // Type A should be 75%, Type B should be 25%
        if let AlgorithmResults::Grouped { groups, .. } = &output.results {
            assert_eq!(groups.len(), 2);
            assert_eq!(groups[0].count, 3);
            assert_eq!(groups[0].percentage, 75.0);
            assert_eq!(groups[1].count, 1);
            assert_eq!(groups[1].percentage, 25.0);
        } else {
            panic!("Expected Grouped results");
        }
    }

    #[test]
    fn test_line_ranges() {
        let entries = vec![
            Entry::from_line("Message".to_string(), 1),
            Entry::from_line("Message".to_string(), 2),
            Entry::from_line("Message".to_string(), 3),
            Entry::from_line("Message".to_string(), 10),
            Entry::from_line("Message".to_string(), 11),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);

        // Should have two ranges: [1-3] and [10-11]
        if let AlgorithmResults::Grouped { groups, .. } = &output.results {
            assert_eq!(groups.len(), 1);
            let ranges = &groups[0].line_ranges;
            assert_eq!(ranges.len(), 2);
            assert_eq!(ranges[0], (1, 3));
            assert_eq!(ranges[1], (10, 11));
        } else {
            panic!("Expected Grouped results");
        }
    }

    #[test]
    fn test_variable_samples() {
        let entries = vec![
            Entry::from_line("User 123 logged in".to_string(), 1),
            Entry::from_line("User 456 logged in".to_string(), 2),
            Entry::from_line("User 789 logged in".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);

        if let AlgorithmResults::Grouped { groups, .. } = &output.results {
            assert_eq!(groups.len(), 1);
            let samples = &groups[0].samples;
            assert!(!samples.is_empty());

            // Should have captured the user IDs
            let var_values = &samples[0].variable_values;
            assert!(!var_values.is_empty());
        } else {
            panic!("Expected Grouped results");
        }
    }
}
