# txtfold repo

This repository contains a tool called txtfold which will surface outliers and repetitive data in large files. It's built in Rust. There is a CLI, also built in Rust, a Python binding, a JS/TS binding and a web UI that uses the JS/TS binding. Refer to the [ARCHITECTURE](./ARCHITECTURE.md) document for details.

We're using `make` as the unifying build tool, refer to `Makefile` for details.

## Build dependencies

The project contains a number of unusual build dependencies. The core defines a generic API "schema" which the bindings and the web ui use to generate their exposed API's and UI. Also, documentation is extracted from the README and built into the core as well as the web UI, in an attempt to keep a single source of truth. This should work more or less automatically, but it's good to keep in mind when making fundamental changes.

## Multi-language repo

We have projects in multiple programming languages. Strive to follow the norms of each language (like naming conventions, directory structuring, etc.) when working with it, and seek uniformity across projects secondly.

## General file/folder structure

Keep files focused and maintainable by breaking them up once they grow too large or take on multiple responsibilities—at that point, convert the file into a folder and split its logic into smaller, well-named modules. Each folder should represent a single feature or component and contain all related files (such as logic, styles, tests, and utilities) rather than separating them by file type across the project. Avoid organizing by technical categories like “css” or “js” directories; instead, co-locate everything that belongs together so structure reflects functionality, improves readability, and makes navigation more intuitive.

### Sub-folder AGENTS.md files

Each folder in any /src folder should contain an AGENTS.md file that outlines the contents of the folder. The purpose is "caching" of the information. This means that token economics is important here. It should be succinct, and anything which is obvious (for instance, from the name), shouldn't be stated. It should help the agent understand the folder without or before scanning the code inside. This file should be read whenever working with files inside the folder and updated whenever necessary. When a new folder is created, the parent folder’s AGENTS.md should also be reviewed and updated if needed (for example, to document how the new folder relates to the parent or to describe any new cross‑folder responsibilities).
