//! Metadata definitions for algorithms, input formats, and output formatters
//!
//! This module provides const-friendly metadata structures that allow each
//! component to declare its configuration needs alongside its implementation.

use serde::Serialize;

/// Metadata describing an algorithm
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AlgorithmMetadata {
    /// Primary name of the algorithm
    pub name: &'static str,
    /// Alternative names that can be used to refer to this algorithm
    pub aliases: &'static [&'static str],
    /// Human-readable description of what the algorithm does
    pub description: &'static str,
    /// What type of data this algorithm works best with
    pub best_for: &'static str,
    /// Configuration parameters this algorithm accepts
    pub parameters: &'static [Parameter],
    /// Input types this algorithm can process
    pub input_types: &'static [InputType],
}

/// Metadata describing an output formatter
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FormatterMetadata {
    /// Primary name of the formatter
    pub name: &'static str,
    /// Alternative names that can be used
    pub aliases: &'static [&'static str],
    /// Human-readable description
    pub description: &'static str,
    /// MIME type of the output
    pub mime_type: &'static str,
    /// File extension (without the dot)
    pub file_extension: &'static str,
    /// Whether this formatter can stream output incrementally
    pub supports_streaming: bool,
}

/// Metadata describing an input format
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct InputFormatMetadata {
    /// Primary name of the format
    pub name: &'static str,
    /// Alternative names
    pub aliases: &'static [&'static str],
    /// Human-readable description
    pub description: &'static str,
    /// Sub-options specific to this input format
    pub sub_options: &'static [SubOption],
}

/// A sub-option for an input format (e.g., entry-mode for text)
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SubOption {
    /// Name of the sub-option
    pub name: &'static str,
    /// Possible values
    pub values: &'static [&'static str],
    /// Default value
    pub default: &'static str,
    /// Description
    pub description: &'static str,
}

/// A configurable parameter for an algorithm
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Parameter {
    /// Parameter name (as it appears in code/CLI)
    pub name: &'static str,
    /// Type of the parameter
    pub type_info: ParamType,
    /// Default value
    pub default: ParamDefault,
    /// Valid range (if applicable)
    pub range: Option<ParamRange>,
    /// Human-readable description
    pub description: &'static str,
    /// Special values with semantic meaning (e.g., 0.0 = "auto-detect")
    pub special_values: &'static [(f64, &'static str)],
}

/// Type of a parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ParamType {
    /// Floating point number
    Float,
    /// Unsigned integer
    USize,
    /// Boolean flag
    Bool,
    /// String value
    String,
    /// Enum with specific choices
    Enum(&'static [&'static str]),
}

/// Default value for a parameter
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ParamDefault {
    Float(f64),
    USize(usize),
    Bool(bool),
    Str(&'static str),
}

/// Valid range for a parameter
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ParamRange {
    Float { min: f64, max: f64 },
    USize { min: usize, max: usize },
}

/// Input type that an algorithm can process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InputType {
    /// Plain text / log files
    Text,
    /// JSON arrays
    JsonArray,
    /// JSON maps/objects
    JsonMap,
    /// Arbitrary nested JSON
    JsonNested,
}
