.PHONY: all build schema web test clean

# Build the release binary
build:
	cargo build --release

# Regenerate schemas from the core library metadata
schema: build
	./target/release/dump-schema web/schema.json output-schema.json

# Build the web UI (schema must be up to date first)
web: schema
	cd web && npm run build

# Run all tests
test:
	cargo test

clean:
	cargo clean
