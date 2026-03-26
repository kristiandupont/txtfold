# txtfold (npm)

Identifies patterns and outliers in large log files and structured data. No ML, no fuzzy logic — same input always produces the same output.

## Installation

```
npm install txtfold
```

## Quick start

```ts
import { process, processMarkdown } from "txtfold";

// Returns a typed object
const result = process(text);

// Or get a markdown summary directly
const md = processMarkdown(text);
```

By default, the algorithm and input format are auto-detected. Options can be passed as a second argument:

```ts
const result = process(text, { algorithm: "clustering", threshold: 0.9 });
```

## Documentation

Full documentation — algorithms, parameters, and output schema — is at **https://txtfold.dev/docs**.
