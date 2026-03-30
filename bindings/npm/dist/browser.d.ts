export type { AnalysisOutput, AnalysisMetadata, AnalysisSummary, AlgorithmResults, GroupedResults, OutlierFocusedResults, SchemaGroupedResults, PathGroupedResults, GroupOutput, SampleEntry, OutlierOutput, BaselineOutput, ThresholdInfo, ScoreStatsOutput, SchemaGroupOutput, PathPatternOutput, ProcessOptions, } from "./types.js";
import type { AnalysisOutput, ProcessOptions } from "./types.js";
/**
 * Explicitly pre-initialise the WASM module. Optional — all other exports
 * call this lazily on first use.
 */
export declare function load(): Promise<void>;
/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function process(input: string, options?: ProcessOptions): Promise<AnalysisOutput>;
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function processMarkdown(input: string, options?: ProcessOptions): Promise<string>;
/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export declare function processFormatted(input: string, options?: ProcessOptions, format?: string): Promise<string>;
