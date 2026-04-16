export type { AnalysisOutput, AnalysisMetadata, AnalysisSummary, AlgorithmResults, GroupedResults, OutlierFocusedResults, SchemaGroupedResults, PathGroupedResults, GroupOutput, SampleEntry, OutlierOutput, BaselineOutput, ThresholdInfo, ScoreStatsOutput, SchemaGroupOutput, PathPatternOutput, ProcessOptions, DiscoverOutput, FieldSummary, DiscoverOptions, CostPreviewOutput, FieldCost, CostPreviewOptions, } from "./types.js";
import type { AnalysisOutput, ProcessOptions, DiscoverOutput, DiscoverOptions, CostPreviewOutput, CostPreviewOptions } from "./types.js";
/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function process(input: string, options: ProcessOptions): AnalysisOutput;
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function processMarkdown(input: string, options: ProcessOptions): string;
/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function processFormatted(input: string, options: ProcessOptions, format?: string): string;
/**
 * Run structural discovery and return a typed DiscoverOutput.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function discover(input: string, options: DiscoverOptions): DiscoverOutput;
/**
 * Run structural discovery and return a markdown-formatted schema table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function discoverMarkdown(input: string, options: DiscoverOptions): string;
/**
 * Run full analysis and return a field-level token cost breakdown.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function costPreview(input: string, options: CostPreviewOptions): CostPreviewOutput;
/**
 * Run full analysis and return a markdown cost breakdown table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function costPreviewMarkdown(input: string, options: CostPreviewOptions): string;
