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
export type AlgorithmResults = GroupedResults | OutlierFocusedResults | SchemaGroupedResults | PathGroupedResults;
/** Metadata about the analysis run */
export interface AnalysisMetadata {
    algorithm: string;
    budget_applied?: boolean | null;
    budget_lines?: number | null;
    input_file?: string | null;
    reduction_ratio: number;
    total_entries: number;
}
/** Complete analysis output */
export interface AnalysisOutput {
    metadata: AnalysisMetadata;
    results: AlgorithmResults;
    summary: AnalysisSummary;
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
/** Field-level token breakdown of an analysis result. */
export interface CostPreviewOutput {
    estimated_tokens: number;
    fields: FieldCost[];
    suggestion?: string | null;
    warning?: string | null;
}
/** Output of the discover operation — a compact structural schema map. */
export interface DiscoverOutput {
    entry_count: number;
    fields: FieldSummary[];
    format: string;
}
/** Token cost attributed to a single field across all groups/patterns. */
export interface FieldCost {
    note?: string | null;
    path: string;
    pct: number;
    tokens: number;
}
/** Summary of a single field/slot discovered in the input. */
export interface FieldSummary {
    cardinality: number;
    path: string;
    present_in_pct: number;
    samples: string[];
    types: string[];
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
    /** Pipeline expression selecting the algorithm and pre-processing steps.
     *  Examples: "outliers", "similar(0.8) | top(20)",
     *  ".diagnostics[] | del(.sourceCode) | group_by(.category)".
     *  If omitted, defaults to summarize (json→subtree, line/block→template). */
    pipeline?: string;
    /** N-gram size for the 'outliers' verb. Default: 2. */
    ngramSize?: number;
    /** Outlier score threshold for the 'outliers' verb (0.0 = auto-detect). Default: 0.0. */
    outlierThreshold?: number;
    /** Nesting depth for the 'subtree' verb. Default: 1. */
    depth?: number;
    /** Maximum output lines. Most important groups shown first; output trimmed at limit. */
    budgetLines?: number;
}
/** Options for discover() and discoverMarkdown(). */
export interface DiscoverOptions {
    /** Input format: "json", "line", or "block". Required. */
    inputFormat: string;
}
/** Options for costPreview() and costPreviewMarkdown(). */
export interface CostPreviewOptions {
    /** Input format: "json", "line", or "block". Required. */
    inputFormat: string;
    /** Pipeline expression (same syntax as ProcessOptions.pipeline). */
    pipeline?: string;
    /** N-gram size for the 'outliers' verb. Default: 2. */
    ngramSize?: number;
    /** Outlier score threshold for the 'outliers' verb (0.0 = auto-detect). Default: 0.0. */
    outlierThreshold?: number;
    /** Nesting depth for the 'subtree' verb. Default: 1. */
    depth?: number;
}
