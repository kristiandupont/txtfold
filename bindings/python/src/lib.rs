use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Run an analysis on text or JSON input.
///
/// Returns a JSON string conforming to output-schema.json.
///
/// Args:
///     input: Text or JSON content to analyze.
///     algorithm: One of "auto", "template", "clustering", "ngram", "schema", "subtree".
///     threshold: Similarity threshold for clustering/schema algorithms (0.0–1.0).
///     ngram_size: N-gram size for the ngram algorithm.
///     outlier_threshold: Outlier threshold for ngram (0.0 = auto-detect).
///     format: Output format — "json" or "markdown".
#[pyfunction]
#[pyo3(signature = (input, algorithm="auto", threshold=0.8, ngram_size=2, outlier_threshold=0.0, format="json"))]
fn process(
    input: &str,
    algorithm: &str,
    threshold: f64,
    ngram_size: usize,
    outlier_threshold: f64,
    format: &str,
) -> PyResult<String> {
    txtfold::process(input, algorithm, threshold, ngram_size, outlier_threshold, format)
        .map_err(PyValueError::new_err)
}

#[pymodule]
fn _txtfold(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(process, m)?)?;
    Ok(())
}
