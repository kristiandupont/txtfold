# txtfold-core

Identifies patterns and outliers in large log files and structured data. No ML, no fuzzy logic — same input always produces the same output.

## Installation

```toml
[dependencies]
txtfold-core = "0.1"
```

## Quick start

```rust
use txtfold_core::{process, ProcessOptions, InputFormat};

// Auto-detect format and algorithm
let output = process(text, &ProcessOptions::default())?;

// Or specify explicitly
let options = ProcessOptions {
    input_format: Some(InputFormat::Line),
    ..Default::default()
};
let output = process(text, &options)?;
```

## Documentation

Full documentation — algorithms, parameters, and output schema — is at **https://kristiandupont.github.io/txtfold/**.
