export type { AnalysisOutput, AnalysisMetadata, AnalysisSummary, AlgorithmResults, GroupedResults, OutlierFocusedResults, SchemaGroupedResults, PathGroupedResults, GroupOutput, SampleEntry, OutlierOutput, BaselineOutput, ThresholdInfo, ScoreStatsOutput, SchemaGroupOutput, PathPatternOutput, ProcessOptions, } from "./types.js";
import type { AnalysisOutput, ProcessOptions } from "./types.js";
/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function process(input: string, options?: ProcessOptions): AnalysisOutput;
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function processMarkdown(input: string, options?: ProcessOptions): string;
