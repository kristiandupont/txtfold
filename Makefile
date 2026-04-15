.PHONY: all build install schema types wasm npm python web dev test clean

# --- Rust -------------------------------------------------------------------

# Build the release binaries (CLI + tools)
build:
	cargo build --release

# Install the CLI into the local Cargo bin path
install:
	cargo install --path cli

# --- Schema & generated types -----------------------------------------------

# Regenerate schema.json / output-schema.json from the compiled core metadata
schema: build
	./target/release/dump-schema web/schema.json output-schema.json

# Generate TypeScript and Python type stubs from output-schema.json
types: schema
	bun tools/gen-types.ts

# --- Language bindings ------------------------------------------------------

# Rebuild only the WASM binaries and regenerate the JS/TS glue files.
# Run this after any change to the core library's #[wasm_bindgen] API.
wasm:
	cd bindings/npm && bun run build:wasm && bun run build:wasm-web

# Build the npm package (both WASM targets + TypeScript)
npm: types
	cd bindings/npm && bun run build

# Build and install the Python extension into the active environment
python: types
	cd bindings/python && maturin develop --release

# --- Web --------------------------------------------------------------------

# Production build of the web UI (requires npm package to be built first)
web: npm
	cd web && bun run build

# Start the Vite dev server (uses whatever npm package is already built)
dev:
	cd web && bun run dev

# Rebuild core → schema → types → npm package, then start the Vite dev server
dev-web: npm
	cd web && bun run dev

# --- Tests ------------------------------------------------------------------

test:
	cargo test
	cd bindings/npm && bun test
	cd bindings/python && python -m pytest

# --- Meta -------------------------------------------------------------------

# Build everything
all: web python

clean:
	cargo clean
	rm -rf bindings/npm/dist bindings/npm/wasm bindings/npm/wasm-web
	rm -rf web/dist
