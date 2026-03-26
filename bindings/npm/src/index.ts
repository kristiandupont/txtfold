// wasm-pack --target nodejs self-initialises at require() time —
// no manual initSync or WASM loading needed.
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { process_text } = require("../wasm/txtfold.js");

export type {
  AnalysisOutput,
  AnalysisMetadata,
  AnalysisSummary,
  AlgorithmResults,
  GroupedResults,
  OutlierFocusedResults,
  SchemaGroupedResults,
  PathGroupedResults,
  GroupOutput,
  SampleEntry,
  OutlierOutput,
  BaselineOutput,
  ThresholdInfo,
  ScoreStatsOutput,
  SchemaGroupOutput,
  PathPatternOutput,
  ProcessOptions,
} from "./types.js";

import type { AnalysisOutput, ProcessOptions } from "./types.js";

function callCore(input: string, options: ProcessOptions, format: string): string {
  const {
    algorithm = "auto",
    threshold = 0.8,
    ngramSize = 2,
    outlierThreshold = 0.0,
  } = options;
  return process_text(input, algorithm, threshold, ngramSize, outlierThreshold, format) as string;
}

/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function process(input: string, options: ProcessOptions = {}): AnalysisOutput {
  return JSON.parse(callCore(input, options, "json")) as AnalysisOutput;
}

/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function processMarkdown(input: string, options: ProcessOptions = {}): string {
  return callCore(input, options, "markdown");
}
