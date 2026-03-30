# THIS FILE IS GENERATED — do not edit by hand.
# Source: output-schema.json
# Regenerate: bun tools/gen-types.ts

from __future__ import annotations

from typing import Any, Literal, Union
from typing import TypedDict


# Complete analysis output
class AnalysisOutput(TypedDict):
    metadata: AnalysisMetadata
    results: AlgorithmResults
    summary: AnalysisSummary


# Pattern grouping with optional outliers (template extraction, clustering)
class GroupedResults(TypedDict):
    groups: list[GroupOutput]
    outliers: list[OutlierOutput]
    type: Literal["grouped"]


# Outlier-focused with baseline information (n-gram analysis)
class OutlierFocusedResults(TypedDict):
    baseline: BaselineOutput
    outliers: list[OutlierOutput]
    type: Literal["outlier_focused"]


# Schema-based grouping (JSON/structured data)
class SchemaGroupedResults(TypedDict):
    outliers: list[OutlierOutput]
    schemas: list[SchemaGroupOutput]
    type: Literal["schema_grouped"]


# Path-based pattern grouping (subtree algorithm)
class PathGroupedResults(TypedDict):
    patterns: list[PathPatternOutput]
    singletons: list[OutlierOutput]
    type: Literal["path_grouped"]


# Algorithm-specific output formats
AlgorithmResults = Union[GroupedResults, OutlierFocusedResults, SchemaGroupedResults, PathGroupedResults]


# Metadata about the analysis run
class _AnalysisMetadataRequired(TypedDict):
    algorithm: str
    reduction_ratio: float
    total_entries: int

class AnalysisMetadata(_AnalysisMetadataRequired, total=False):
    budget_applied: bool | None
    budget_lines: int | None
    input_file: str | None


# Summary statistics
class AnalysisSummary(TypedDict):
    largest_cluster: int
    outliers: int
    unique_patterns: int


# Baseline information for outlier-focused algorithms
class _BaselineOutputRequired(TypedDict):
    common_features: list[str]
    description: str
    normal_count: int
    normal_percentage: float

class BaselineOutput(_BaselineOutputRequired, total=False):
    threshold: ThresholdInfo | None


# A single pattern group in the output
class GroupOutput(TypedDict):
    count: int
    id: str
    line_ranges: list[tuple[int, int]]
    name: str
    pattern: str
    percentage: float
    samples: list[SampleEntry]


# An outlier entry
class OutlierOutput(TypedDict):
    content: str
    id: str
    line_number: int
    reason: str
    score: float


# A structural pattern found at one or more paths in a JSON document (subtree algorithm)
class PathPatternOutput(TypedDict):
    count: int
    fields: list[str]
    id: str
    paths: list[str]
    percentage: float
    sample_values: dict[str, list[str]]
    schema_description: str


# A sample entry from a group
class SampleEntry(TypedDict):
    content: str
    line_numbers: list[int]
    variable_values: dict[str, list[str]]


# A schema group (for JSON/structured data)
class SchemaGroupOutput(TypedDict):
    count: int
    entry_indices: list[int]
    fields: list[str]
    id: str
    name: str
    percentage: float
    sample_values: dict[str, list[str]]
    schema_description: str


# Score statistics for n-gram analysis
class ScoreStatsOutput(TypedDict):
    max: float
    mean: float
    median: float
    min: float


# Information about threshold used for outlier detection
class _ThresholdInfoRequired(TypedDict):
    auto_detected: bool
    value: float

class ThresholdInfo(_ThresholdInfoRequired, total=False):
    score_stats: ScoreStatsOutput | None

