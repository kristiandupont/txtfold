use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Run an analysis on text or JSON input.
///
/// Returns a JSON string conforming to output-schema.json.
///
/// Args:
///     input: Text or JSON content to analyze.
///     input_format: Input format — "json", "line", or "block". Required.
///     pipeline: Optional pipeline expression (e.g. "del(.x) | schemas").
///               The terminal verb selects the algorithm.
///     ngram_size: N-gram size for the 'outliers' verb.
///     outlier_threshold: Outlier threshold for 'outliers' (0.0 = auto-detect).
///     depth: Nesting depth for the 'subtree' verb.
///     budget: Maximum output lines. Most important groups shown first.
///     format: Output format — "json" or "markdown".
#[pyfunction]
#[pyo3(signature = (input, input_format, pipeline=None, ngram_size=2, outlier_threshold=0.0, depth=1, budget=None, format="json"))]
fn process(
    input: &str,
    input_format: &str,
    pipeline: Option<&str>,
    ngram_size: usize,
    outlier_threshold: f64,
    depth: usize,
    budget: Option<usize>,
    format: &str,
) -> PyResult<String> {
    let fmt = txtfold::input_format_from_str(input_format)
        .map_err(PyValueError::new_err)?;
    let options = txtfold::ProcessOptions {
        input_format: fmt,
        pipeline_expr: pipeline.map(|s| s.to_string()),
        budget,
        ngram_size,
        outlier_threshold,
        depth,
    };
    txtfold::process(input, &options, format)
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

/// Run full analysis then return a field-level token cost breakdown.
///
/// Returns either a JSON string (CostPreviewOutput) or a markdown table.
///
/// Args:
///     input: Text or JSON content to analyze.
///     input_format: Input format — "json", "line", or "block". Required.
///     pipeline: Optional pipeline expression (e.g. "del(.x) | schemas").
///     ngram_size: N-gram size for the 'outliers' verb.
///     outlier_threshold: Outlier threshold for 'outliers' (0.0 = auto-detect).
///     depth: Nesting depth for the 'subtree' verb.
///     format: Output format — "json" or "markdown".
#[pyfunction]
#[pyo3(signature = (input, input_format, pipeline=None, ngram_size=2, outlier_threshold=0.0, depth=1, format="json"))]
fn cost_preview(
    input: &str,
    input_format: &str,
    pipeline: Option<&str>,
    ngram_size: usize,
    outlier_threshold: f64,
    depth: usize,
    format: &str,
) -> PyResult<String> {
    let fmt = txtfold::input_format_from_str(input_format)
        .map_err(PyValueError::new_err)?;
    let options = txtfold::ProcessOptions {
        input_format: fmt,
        pipeline_expr: pipeline.map(|s| s.to_string()),
        budget: None,
        ngram_size,
        outlier_threshold,
        depth,
    };
    txtfold::cost_preview_formatted(input, &options, format)
        .map_err(PyValueError::new_err)
}

#[pymodule]
fn _txtfold(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(process, m)?)?;
    m.add_function(wrap_pyfunction!(discover, m)?)?;
    m.add_function(wrap_pyfunction!(cost_preview, m)?)?;
    Ok(())
}
