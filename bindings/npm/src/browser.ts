import init, { process_text, discover_text, cost_preview_text } from "../wasm-web/txtfold.js";

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

let initPromise: Promise<void> | null = null;

function ensureInit(): Promise<void> {
  if (!initPromise) {
    initPromise = init().then(() => undefined);
  }
  return initPromise!;
}

function callCore(input: string, options: ProcessOptions, format: string): string {
  const {
    inputFormat,
    pipeline = "",
    ngramSize = 2,
    outlierThreshold = 0.0,
    depth = 1,
    budgetLines = undefined,
  } = options;
  return process_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, budgetLines, format) as string;
}

/**
 * Explicitly pre-initialise the WASM module. Optional — all other exports
 * call this lazily on first use.
 */
export async function load(): Promise<void> {
  await ensureInit();
}

/**
 * Analyse text or JSON input and return structured results.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function process(input: string, options: ProcessOptions): Promise<AnalysisOutput> {
  await ensureInit();
  return JSON.parse(callCore(input, options, "json")) as AnalysisOutput;
}

/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function processMarkdown(input: string, options: ProcessOptions): Promise<string> {
  await ensureInit();
  return callCore(input, options, "markdown");
}

/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function processFormatted(input: string, options: ProcessOptions, format: string = "markdown"): Promise<string> {
  await ensureInit();
  return callCore(input, options, format);
}

/**
 * Run structural discovery and return a typed DiscoverOutput.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function discover(input: string, options: DiscoverOptions): Promise<DiscoverOutput> {
  await ensureInit();
  return JSON.parse(discover_text(input, options.inputFormat, "json") as string) as DiscoverOutput;
}

/**
 * Run structural discovery and return a markdown-formatted schema table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function discoverMarkdown(input: string, options: DiscoverOptions): Promise<string> {
  await ensureInit();
  return discover_text(input, options.inputFormat, "markdown") as string;
}

/**
 * Run full analysis and return a field-level token cost breakdown.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function costPreview(input: string, options: CostPreviewOptions): Promise<CostPreviewOutput> {
  await ensureInit();
  const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1 } = options;
  return JSON.parse(
    cost_preview_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, "json") as string
  ) as CostPreviewOutput;
}

/**
 * Run full analysis and return a markdown cost breakdown table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function costPreviewMarkdown(input: string, options: CostPreviewOptions): Promise<string> {
  await ensureInit();
  const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1 } = options;
  return cost_preview_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, "markdown") as string;
}
