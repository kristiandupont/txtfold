// wasm-pack --target nodejs self-initialises at require() time —
// no manual initSync or WASM loading needed.
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { process_text } = require("../wasm/txtfold.js");
function callCore(input, options, format) {
    const { algorithm = "auto", threshold = 0.8, ngramSize = 2, outlierThreshold = 0.0, } = options;
    return process_text(input, algorithm, threshold, ngramSize, outlierThreshold, format);
}
/**
 * Analyse text or JSON input and return structured results.
 *
 * The returned object matches the schema in `output-schema.json`.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function process(input, options = {}) {
    return JSON.parse(callCore(input, options, "json"));
}
/**
 * Analyse text or JSON input and return a markdown-formatted summary.
 *
 * @throws {Error} if the input cannot be processed.
 */
export function processMarkdown(input, options = {}) {
    return callCore(input, options, "markdown");
}
