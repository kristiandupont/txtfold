use serde::{Deserialize, Serialize};
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
    /// Path-based pattern grouping (subtree algorithm)
    PathGrouped {
        patterns: Vec<PathPatternOutput>,
        singletons: Vec<OutlierOutput>,
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
    /// Reduction ratio (output size / input size)
    pub reduction_ratio: f64,
    /// Budget in output lines requested by the caller (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_lines: Option<usize>,
    /// Whether the budget was reached and output was trimmed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_applied: Option<bool>,
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

/// A structural pattern found at one or more paths in a JSON document (subtree algorithm)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PathPatternOutput {
    /// Pattern identifier
    pub id: String,
    /// Human-readable schema description
    pub schema_description: String,
    /// Fields present in this schema
    pub fields: Vec<String>,
    /// Total number of objects that matched this pattern
    pub count: usize,
    /// Percentage of total objects found in the document
    pub percentage: f64,
    /// Normalized paths where this pattern appears (e.g. `$.users[*]`)
    pub paths: Vec<String>,
    /// Sample field values
    pub sample_values: HashMap<String, Vec<String>>,
}
