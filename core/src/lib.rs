//! txtfold: Deterministic text compression for log analysis
//!
//! This library provides algorithms for identifying patterns in large text files,
//! extracting templates, and detecting outliers.

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod clustering;
pub mod entry;
pub mod formatter;
pub mod ngram;
pub mod output;
pub mod parser;
pub mod patterns;
pub mod template;
pub mod tokenizer;

/// Core library functionality
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn wasm_version() -> String {
    version().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
