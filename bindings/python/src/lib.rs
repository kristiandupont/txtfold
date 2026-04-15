use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Run an analysis on text or JSON input.
///
/// Returns a JSON string conforming to output-schema.json.
///
/// Args:
///     input: Text or JSON content to analyze.
///     input_format: Input format — "json", "line", or "block". Required.
///     algorithm: One of "auto", "template", "clustering", "ngram", "schema", "subtree".
///     threshold: Similarity threshold for clustering/schema algorithms (0.0–1.0).
///     ngram_size: N-gram size for the ngram algorithm.
///     outlier_threshold: Outlier threshold for ngram (0.0 = auto-detect).
///     budget: Maximum output lines. Most important groups shown first; output trimmed at limit.
///     format: Output format — "json" or "markdown".
#[pyfunction]
#[pyo3(signature = (input, input_format, algorithm="auto", threshold=0.8, ngram_size=2, outlier_threshold=0.0, budget=None, format="json"))]
fn process(
    input: &str,
    input_format: &str,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    budget: Option<usize>,
    format: &str,
) -> PyResult<String> {
    let fmt = txtfold::input_format_from_str(input_format)
        .map_err(PyValueError::new_err)?;
    txtfold::process(input, fmt, algorithm, threshold, ngram_size, outlier_threshold, budget, format)
        .map_err(PyValueError::new_err)
}

/// Run structural discovery on text or JSON input.
///
/// Returns either a JSON string (DiscoverOutput) or a markdown table.
///
/// Args:
///     input: Text or JSON content to discover.
///     input_format: Input format — "json", "line", or "block". Required.
///     format: Output format — "json" or "markdown".
#[pyfunction]
#[pyo3(signature = (input, input_format, format="json"))]
fn discover(input: &str, input_format: &str, format: &str) -> PyResult<String> {
    let fmt = txtfold::input_format_from_str(input_format)
        .map_err(PyValueError::new_err)?;
    txtfold::discover_formatted(input, fmt, format)
        .map_err(PyValueError::new_err)
}

#[pymodule]
fn _txtfold(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(process, m)?)?;
    m.add_function(wrap_pyfunction!(discover, m)?)?;
    Ok(())
}
