import init, { process_text, discover_text, cost_preview_text } from "../wasm-web/txtfold.js";
let initPromise = null;
function ensureInit() {
    if (!initPromise) {
        initPromise = init().then(() => undefined);
    }
    return initPromise;
}
function callCore(input, options, format) {
    const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1, budgetLines = undefined, } = options;
    return process_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, budgetLines, format);
}
/**
 * Explicitly pre-initialise the WASM module. Optional — all other exports
 * call this lazily on first use.
 */
export async function load() {
    await ensureInit();
}
/**
 * Analyse text or JSON input and return structured results.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function process(input, options) {
    await ensureInit();
    return JSON.parse(callCore(input, options, "json"));
}
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function processMarkdown(input, options) {
    await ensureInit();
    return callCore(input, options, "markdown");
}
/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function processFormatted(input, options, format = "markdown") {
    await ensureInit();
    return callCore(input, options, format);
}
/**
 * Run structural discovery and return a typed DiscoverOutput.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function discover(input, options) {
    await ensureInit();
    return JSON.parse(discover_text(input, options.inputFormat, "json"));
}
/**
 * Run structural discovery and return a markdown-formatted schema table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function discoverMarkdown(input, options) {
    await ensureInit();
    return discover_text(input, options.inputFormat, "markdown");
}
/**
 * Run full analysis and return a field-level token cost breakdown.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function costPreview(input, options) {
    await ensureInit();
    const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1 } = options;
    return JSON.parse(cost_preview_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, "json"));
}
/**
 * Run full analysis and return a markdown cost breakdown table.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function costPreviewMarkdown(input, options) {
    await ensureInit();
    const { inputFormat, pipeline = "", ngramSize = 2, outlierThreshold = 0.0, depth = 1 } = options;
    return cost_preview_text(input, inputFormat, pipeline, ngramSize, outlierThreshold, depth, "markdown");
}
