.PHONY: all build schema test clean

# Build the release binary
build:
	cargo build --release

# Regenerate schema.json from the core library metadata
schema: build
	./target/release/dump-schema > schema.json

# Run all tests
test:
	cargo test

clean:
	cargo clean
