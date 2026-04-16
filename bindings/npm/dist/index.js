// wasm-pack --target nodejs self-initialises at require() time —
// no manual initSync or WASM loading needed.
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { process_text, discover_text, cost_preview_text } = require("../wasm/txtfold.js");
function callCore(input, options, format) {
    const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1, budgetLines = undefined, } = options;
    return process_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, budgetLines, format);
}
/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function process(input, options) {
    return JSON.parse(callCore(input, options, "json"));
}
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function processMarkdown(input, options) {
    return callCore(input, options, "markdown");
}
/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function processFormatted(input, options, format = "markdown") {
    return callCore(input, options, format);
}
/**
 * Run structural discovery and return a typed DiscoverOutput.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function discover(input, options) {
    return JSON.parse(discover_text(input, options.inputFormat, "json"));
}
/**
 * Run structural discovery and return a markdown-formatted schema table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function discoverMarkdown(input, options) {
    return discover_text(input, options.inputFormat, "markdown");
}
/**
 * Run full analysis and return a field-level token cost breakdown.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function costPreview(input, options) {
    const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1, } = options;
    return JSON.parse(cost_preview_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, "json"));
}
/**
 * Run full analysis and return a markdown cost breakdown table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function costPreviewMarkdown(input, options) {
    const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1, } = options;
    return cost_preview_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, "markdown");
}
