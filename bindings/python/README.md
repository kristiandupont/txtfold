# txtfold

Identifies patterns and outliers in large log files and structured data. No ML, no fuzzy logic — same input always produces the same output.

## Installation

```
pip install txtfold
```

## Quick start

```python
import txtfold

# Returns a structured dict
result = txtfold.process(text)

# Or get a markdown summary directly
md = txtfold.process_markdown(text)
```

By default, the algorithm and input format are auto-detected. Options can be passed as keyword arguments:

```python
result = txtfold.process(text, algorithm="clustering", threshold=0.9)
```

## Documentation

Full documentation — algorithms, parameters, and output schema — is at **https://kristiandupont.github.io/txtfold/**.
