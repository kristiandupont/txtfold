# txtfold

Identifies patterns and outliers in large log files and structured data. No ML, no fuzzy logic — same input always produces the same output.

## Installation

```
cargo install txtfold
```

## Quick start

```sh
txtfold server.log
txtfold --output-format json server.log
cat server.log | txtfold --format line
```

## Documentation

Full documentation — algorithms, parameters, and output schema — is at **https://kristiandupont.github.io/txtfold/**.
