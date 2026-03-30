import init, { process_text } from "../wasm-web/txtfold.js";
let initPromise = null;
function ensureInit() {
    if (!initPromise) {
        initPromise = init().then(() => undefined);
    }
    return initPromise;
}
function callCore(input, options, format) {
    const { algorithm = "auto", threshold = 0.8, ngramSize = 2, outlierThreshold = 0.0, budgetLines = undefined, } = options;
    return process_text(input, algorithm, threshold, ngramSize, outlierThreshold, budgetLines, format);
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
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function process(input, options = {}) {
    await ensureInit();
    return JSON.parse(callCore(input, options, "json"));
}
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function processMarkdown(input, options = {}) {
    await ensureInit();
    return callCore(input, options, "markdown");
}
/**
 * Analyse text or JSON input and return a string in the requested format.
 *
 * @throws {Error} if the input cannot be processed.
 */
export async function processFormatted(input, options = {}, format = "markdown") {
    await ensureInit();
    return callCore(input, options, format);
}
