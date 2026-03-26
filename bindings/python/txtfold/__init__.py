"""
txtfold — deterministic pattern summarization for log files and structured data.

Quick start::

    import txtfold

    # Returns a dict matching output-schema.json
    result = txtfold.process(text)

    # Or get markdown directly
    md = txtfold.process_markdown(text)
"""

from __future__ import annotations

import json
from ._txtfold import process as _process_raw
from ._types import (
    AnalysisOutput,
    AnalysisMetadata,
    AnalysisSummary,
    AlgorithmResults,
    GroupedResults,
    OutlierFocusedResults,
    SchemaGroupedResults,
    PathGroupedResults,
    GroupOutput,
    SampleEntry,
    OutlierOutput,
    BaselineOutput,
    ThresholdInfo,
    ScoreStatsOutput,
    SchemaGroupOutput,
    PathPatternOutput,
)

__all__ = [
    "process",
    "process_markdown",
    "AnalysisOutput",
    "AnalysisMetadata",
    "AnalysisSummary",
    "AlgorithmResults",
    "GroupedResults",
    "OutlierFocusedResults",
    "SchemaGroupedResults",
    "PathGroupedResults",
    "GroupOutput",
    "SampleEntry",
    "OutlierOutput",
    "BaselineOutput",
    "ThresholdInfo",
    "ScoreStatsOutput",
    "SchemaGroupOutput",
    "PathPatternOutput",
]


def process(
    input: str,
    *,
    algorithm: str = "auto",
    threshold: float = 0.8,
    ngram_size: int = 2,
    outlier_threshold: float = 0.0,
) -> AnalysisOutput:
    """Analyze text or JSON input and return structured results.

    The returned dict matches the schema in ``output-schema.json``.

    Args:
        input: Text or JSON content to analyze.
        algorithm: ``"auto"`` (default), ``"template"``, ``"clustering"``,
            ``"ngram"``, ``"schema"``, or ``"subtree"``.
        threshold: Similarity threshold for clustering/schema algorithms (0.0–1.0).
        ngram_size: N-gram size for the ``ngram`` algorithm.
        outlier_threshold: Outlier threshold for ``ngram`` (0.0 = auto-detect).

    Returns:
        Parsed analysis output.

    Raises:
        ValueError: If the input cannot be processed.
    """
    raw = _process_raw(input, algorithm, threshold, ngram_size, outlier_threshold, "json")
    return json.loads(raw)


def process_markdown(
    input: str,
    *,
    algorithm: str = "auto",
    threshold: float = 0.8,
    ngram_size: int = 2,
    outlier_threshold: float = 0.0,
) -> str:
    """Analyze text or JSON input and return a markdown-formatted summary.

    Args:
        input: Text or JSON content to analyze.
        algorithm: ``"auto"`` (default), ``"template"``, ``"clustering"``,
            ``"ngram"``, ``"schema"``, or ``"subtree"``.
        threshold: Similarity threshold for clustering/schema algorithms (0.0–1.0).
        ngram_size: N-gram size for the ``ngram`` algorithm.
        outlier_threshold: Outlier threshold for ``ngram`` (0.0 = auto-detect).

    Returns:
        Markdown-formatted analysis summary.

    Raises:
        ValueError: If the input cannot be processed.
    """
    return _process_raw(input, algorithm, threshold, ngram_size, outlier_threshold, "markdown")
