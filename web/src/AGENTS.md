# web/src

Crank + TypeScript web UI. Bundles the WASM core and renders a single-page app.

## Files (root)

- `main.tsx` — entry point, mounts `<Page />`
- `App.tsx` — top-level layout, wires state to panels
- `Page.tsx` — page shell
- `State.ts` — shared UI state type
- `InputPanel.tsx`, `OptionsPanel.tsx`, `OutputPanel.tsx` — the three main panels
- `SectionHeader.tsx` — reusable section heading component
- `InstallGuide.tsx` — install instructions panel
- `loadCore.tsx` — lazy-loads the WASM module
- `style.css` — global styles

## Sub-folders

- `generated/` — auto-generated files (docs extracted from README); do not edit directly
- `markdown/` — Markdown renderer component + styles
- `wasm/` — compiled WASM artifacts (checked-in build output); do not edit
