// THIS FILE IS GENERATED — do not edit by hand.
// Source: output-schema.json
// Regenerate: bun tools/gen-types.ts

/** Complete analysis output */
export interface AnalysisOutput {
  metadata: AnalysisMetadata;
  results: AlgorithmResults;
  summary: AnalysisSummary;
}

/** Pattern grouping with optional outliers (template extraction, clustering) */
export interface GroupedResults {
  groups: GroupOutput[];
  outliers: OutlierOutput[];
  type: "grouped";
}

/** Outlier-focused with baseline information (n-gram analysis) */
export interface OutlierFocusedResults {
  baseline: BaselineOutput;
  outliers: OutlierOutput[];
  type: "outlier_focused";
}

/** Schema-based grouping (JSON/structured data) */
export interface SchemaGroupedResults {
  outliers: OutlierOutput[];
  schemas: SchemaGroupOutput[];
  type: "schema_grouped";
}

/** Path-based pattern grouping (subtree algorithm) */
export interface PathGroupedResults {
  patterns: PathPatternOutput[];
  singletons: OutlierOutput[];
  type: "path_grouped";
}

/** Algorithm-specific output formats */
export type AlgorithmResults =
  | GroupedResults
  | OutlierFocusedResults
  | SchemaGroupedResults
  | PathGroupedResults;

/** Metadata about the analysis run */
export interface AnalysisMetadata {
  algorithm: string;
  reduction_ratio: number;
  input_file?: string | null;
  total_entries: number;
}

/** Summary statistics */
export interface AnalysisSummary {
  largest_cluster: number;
  outliers: number;
  unique_patterns: number;
}

/** Baseline information for outlier-focused algorithms */
export interface BaselineOutput {
  common_features: string[];
  description: string;
  normal_count: number;
  normal_percentage: number;
  threshold?: ThresholdInfo | null;
}

/** A single pattern group in the output */
export interface GroupOutput {
  count: number;
  id: string;
  line_ranges: [number, number][];
  name: string;
  pattern: string;
  percentage: number;
  samples: SampleEntry[];
}

/** An outlier entry */
export interface OutlierOutput {
  content: string;
  id: string;
  line_number: number;
  reason: string;
  score: number;
}

/** A structural pattern found at one or more paths in a JSON document (subtree algorithm) */
export interface PathPatternOutput {
  count: number;
  fields: string[];
  id: string;
  paths: string[];
  percentage: number;
  sample_values: Record<string, string[]>;
  schema_description: string;
}

/** A sample entry from a group */
export interface SampleEntry {
  content: string;
  line_numbers: number[];
  variable_values: Record<string, string[]>;
}

/** A schema group (for JSON/structured data) */
export interface SchemaGroupOutput {
  count: number;
  entry_indices: number[];
  fields: string[];
  id: string;
  name: string;
  percentage: number;
  sample_values: Record<string, string[]>;
  schema_description: string;
}

/** Score statistics for n-gram analysis */
export interface ScoreStatsOutput {
  max: number;
  mean: number;
  median: number;
  min: number;
}

/** Information about threshold used for outlier detection */
export interface ThresholdInfo {
  auto_detected: boolean;
  score_stats?: ScoreStatsOutput | null;
  value: number;
}

/** Options for process() and processMarkdown(). */
export interface ProcessOptions {
  /** Algorithm to use. Default: "auto" (auto-detect). */
  algorithm?: string;
  /** Similarity threshold for clustering/schema algorithms (0.0–1.0). Default: 0.8. */
  threshold?: number;
  /** N-gram size for the ngram algorithm. Default: 2. */
  ngramSize?: number;
  /** Outlier threshold for ngram (0.0 = auto-detect). Default: 0.0. */
  outlierThreshold?: number;
}
