//! Structured output format for analysis results

use crate::entry::Entry;
use crate::template::{TemplateExtractor, TemplateGroup};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete analysis output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOutput {
    /// Metadata about the analysis
    pub metadata: AnalysisMetadata,
    /// Summary statistics
    pub summary: AnalysisSummary,
    /// Template groups (patterns found)
    pub groups: Vec<GroupOutput>,
    /// Detected outliers
    pub outliers: Vec<OutlierOutput>,
}

/// Metadata about the analysis run
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Build the output from a template extractor
    pub fn build(self, extractor: &TemplateExtractor) -> AnalysisOutput {
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
            groups: group_outputs,
            outliers,
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
        assert_eq!(output.groups.len(), 2);
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
        assert_eq!(output.outliers.len(), 1);
        assert_eq!(output.outliers[0].content, "ERROR Fatal exception");
        assert_eq!(output.outliers[0].line_number, 4);
        assert_eq!(output.outliers[0].reason, "rare_pattern");
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
        assert_eq!(output.groups.len(), 2);
        assert_eq!(output.groups[0].count, 3);
        assert_eq!(output.groups[0].percentage, 75.0);
        assert_eq!(output.groups[1].count, 1);
        assert_eq!(output.groups[1].percentage, 25.0);
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
        assert_eq!(output.groups.len(), 1);
        let ranges = &output.groups[0].line_ranges;
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], (1, 3));
        assert_eq!(ranges[1], (10, 11));
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

        assert_eq!(output.groups.len(), 1);
        let samples = &output.groups[0].samples;
        assert!(!samples.is_empty());

        // Should have captured the user IDs
        let var_values = &samples[0].variable_values;
        assert!(!var_values.is_empty());
    }
}
