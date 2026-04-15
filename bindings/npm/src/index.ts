// wasm-pack --target nodejs self-initialises at require() time —
// no manual initSync or WASM loading needed.
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { process_text, discover_text, cost_preview_text } = require("../wasm/txtfold.js");

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
  DiscoverOutput,
  FieldSummary,
  DiscoverOptions,
  CostPreviewOutput,
  FieldCost,
  CostPreviewOptions,
} from "./types.js";

import type { AnalysisOutput, ProcessOptions, DiscoverOutput, DiscoverOptions, CostPreviewOutput, CostPreviewOptions } from "./types.js";

function callCore(input: string, options: ProcessOptions, format: string): string {
  const {
    inputFormat,
    algorithm = "auto",
    threshold = 0.8,
    ngramSize = 2,
    outlierThreshold = 0.0,
    budgetLines = undefined,
  } = options;
  return process_text(input, inputFormat, algorithm, threshold, ngramSize, outlierThreshold, budgetLines, format) as string;
}

/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function process(input: string, options: ProcessOptions): AnalysisOutput {
  return JSON.parse(callCore(input, options, "json")) as AnalysisOutput;
}

/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function processMarkdown(input: string, options: ProcessOptions): string {
  return callCore(input, options, "markdown");
}

/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function processFormatted(input: string, options: ProcessOptions, format: string = "markdown"): string {
  return callCore(input, options, format);
}

/**
 * Run structural discovery and return a typed DiscoverOutput.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function discover(input: string, options: DiscoverOptions): DiscoverOutput {
  return JSON.parse(discover_text(input, options.inputFormat, "json") as string) as DiscoverOutput;
}

/**
 * Run structural discovery and return a markdown-formatted schema table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function discoverMarkdown(input: string, options: DiscoverOptions): string {
  return discover_text(input, options.inputFormat, "markdown") as string;
}

/**
 * Run full analysis and return a field-level token cost breakdown.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function costPreview(input: string, options: CostPreviewOptions): CostPreviewOutput {
  const {
    inputFormat,
    algorithm = "auto",
    threshold = 0.8,
    ngramSize = 2,
    outlierThreshold = 0.0,
  } = options;
  return JSON.parse(
    cost_preview_text(input, inputFormat, algorithm, threshold, ngramSize, outlierThreshold, "json") as string
  ) as CostPreviewOutput;
}

/**
 * Run full analysis and return a markdown cost breakdown table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function costPreviewMarkdown(input: string, options: CostPreviewOptions): string {
  const {
    inputFormat,
    algorithm = "auto",
    threshold = 0.8,
    ngramSize = 2,
    outlierThreshold = 0.0,
  } = options;
  return cost_preview_text(input, inputFormat, algorithm, threshold, ngramSize, outlierThreshold, "markdown") as string;
}
