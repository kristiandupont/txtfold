# bindings/npm/src

TypeScript Node.js binding. Wraps the WASM core (`../wasm/txtfold.js`) with a typed, ergonomic API.

- `index.ts` — public API: `process`, `processMarkdown`, `processFormatted`, `discover`, `discoverMarkdown`, `costPreview`, `costPreviewMarkdown`
- `browser.ts` — browser-specific variant (uses `../wasm-web/` instead)
- `types.ts` — TypeScript type definitions mirroring `output-schema.json`
