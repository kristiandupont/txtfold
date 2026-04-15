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
  budget_applied?: boolean | null;
  budget_lines?: number | null;
  input_file?: string | null;
  reduction_ratio: number;
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
  /** Input format: "json", "line", or "block". Required. */
  inputFormat: string;
  /** Algorithm to use. Default: "auto" (auto-detect). */
  algorithm?: string;
  /** Similarity threshold for clustering/schema algorithms (0.0–1.0). Default: 0.8. */
  threshold?: number;
  /** N-gram size for the ngram algorithm. Default: 2. */
  ngramSize?: number;
  /** Outlier threshold for ngram (0.0 = auto-detect). Default: 0.0. */
  outlierThreshold?: number;
  /** Maximum output lines. Most important groups shown first; output trimmed at limit. */
  budgetLines?: number;
}

// ── Discover types ────────────────────────────────────────────────────────────
// Note: not yet part of output-schema.json — will be added once the type stabilizes.

/** Summary of a single field or slot discovered in the input. */
export interface FieldSummary {
  /** Normalized path, e.g. "$.diagnostics[*].category" or "slot[0]" */
  path: string;
  /** Value types seen at this path, e.g. ["string", "null"] */
  types: string[];
  /** Number of distinct values seen (capped at 10 000) */
  cardinality: number;
  /** Up to 5 representative values */
  samples: string[];
  /** Fraction of entries that contain this field (0.0–1.0) */
  present_in_pct: number;
}

/** Output of the discover operation — a compact structural schema map. */
export interface DiscoverOutput {
  /** Input format: "json", "line", or "block" */
  format: string;
  /** Total number of top-level entries processed */
  entry_count: number;
  /** Per-field summaries */
  fields: FieldSummary[];
}

/** Options for discover() and discoverMarkdown(). */
export interface DiscoverOptions {
  /** Input format: "json", "line", or "block". Required. */
  inputFormat: string;
}
