# cli/src

Single-file CLI binary (`txtfold`). Everything lives in `main.rs`.

Builds the `clap` argument parser dynamically from `txtfold_core::registry` so the help text stays in sync with the core. Reads stdin or a file, calls `txtfold_core::process` / `discover` / `cost_preview`, and prints the result.
