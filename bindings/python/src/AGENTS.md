# bindings/python/src

PyO3 Rust shim that exposes `txtfold_core` to Python. Single file: `lib.rs`.

Exports three `#[pyfunction]`s — `process`, `discover`, `cost_preview` — which map Python keyword arguments to `txtfold_core::ProcessOptions` and return JSON or Markdown strings. The Python package wraps these in `bindings/python/txtfold/`.
