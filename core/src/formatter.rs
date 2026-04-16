//! Formatters for converting analysis output to human-readable formats

use crate::metadata::FormatterMetadata;
use crate::output::AnalysisOutput;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format a number with thousands separators, e.g. `1190` → `"1,190"`.
fn format_count(n: usize) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut result = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Markdown formatter for analysis output
pub struct MarkdownFormatter;

impl MarkdownFormatter {
    /// Metadata describing this formatter
    pub const METADATA: FormatterMetadata = FormatterMetadata {
        name: "markdown",
        aliases: &["md"],
        description: "Human-readable markdown format with sections, tables, and code blocks",
        mime_type: "text/markdown",
        file_extension: "md",
        supports_streaming: false,
    };

    /// Format analysis output as Markdown
    pub fn format(output: &AnalysisOutput) -> String {
        let mut md = String::new();

        // Title
        md.push_str("# txtfold output\n\n");

        // Compact metadata line: "N entries → M groups  (algorithm)  [budget: N lines]"
        use crate::output::AlgorithmResults;
        let (group_count, group_label) = match &output.results {
            AlgorithmResults::Grouped { groups, .. } => {
                (groups.len(), if groups.len() == 1 { "group" } else { "groups" })
            }
            AlgorithmResults::SchemaGrouped { schemas, .. } => {
                (schemas.len(), if schemas.len() == 1 { "schema" } else { "schemas" })
            }
            AlgorithmResults::PathGrouped { patterns, .. } => {
                (patterns.len(), if patterns.len() == 1 { "pattern" } else { "patterns" })
            }
            AlgorithmResults::OutlierFocused { outliers, .. } => {
                (outliers.len(), if outliers.len() == 1 { "outlier" } else { "outliers" })
            }
        };

        let entry_word = if output.metadata.total_entries == 1 { "entry" } else { "entries" };

        let budget_suffix = if output.metadata.budget_applied == Some(true) {
            output.metadata.budget_lines
                .map(|b| format!("  [budget: {} lines]", b))
                .unwrap_or_default()
        } else {
            String::new()
        };

        md.push_str(&format!(
            "{} {} → {} {}  ({}){}\n\n",
            format_count(output.metadata.total_entries),
            entry_word,
            group_count,
            group_label,
            output.metadata.algorithm,
            budget_suffix,
        ));

        // Results section
        match &output.results {
            AlgorithmResults::Grouped { groups, outliers } => {
                Self::format_grouped_results(&mut md, groups, outliers, &output.metadata.algorithm);
            }
            AlgorithmResults::OutlierFocused { baseline, outliers } => {
                Self::format_outlier_focused_results(&mut md, baseline, outliers);
            }
            AlgorithmResults::SchemaGrouped { schemas, outliers } => {
                Self::format_schema_grouped_results(&mut md, schemas, outliers);
            }
            AlgorithmResults::PathGrouped { patterns, singletons } => {
                Self::format_path_grouped_results(&mut md, patterns, singletons);
            }
        }

        md
    }

    /// Render sample entries with deduplication.
    ///
    /// Rules:
    /// - All samples identical → show one + "(all N entries identical)"
    /// - Fewer distinct than limit → show only the distinct ones
    /// - More distinct than limit → show up to limit + "(+ M more distinct)"
    fn format_samples(
        md: &mut String,
        samples: &[crate::output::SampleEntry],
        group_count: usize,
    ) {
        if samples.is_empty() {
            return;
        }

        // Deduplicate by content string.
        let mut seen = std::collections::HashSet::new();
        let distinct: Vec<&crate::output::SampleEntry> = samples
            .iter()
            .filter(|s| seen.insert(s.content.clone()))
            .collect();

        let num_samples = samples.len();
        let num_distinct = distinct.len();

        const SAMPLE_LIMIT: usize = 3;
        let to_show = &distinct[..num_distinct.min(SAMPLE_LIMIT)];
        let more = num_distinct.saturating_sub(SAMPLE_LIMIT);

        if to_show.len() == 1 {
            md.push_str("**Sample entry**:\n```\n");
            md.push_str(&to_show[0].content);
            md.push_str("\n```\n");

            // Annotate when all samples were identical (implying all entries are).
            if num_distinct == 1 && num_samples > 1 {
                md.push_str(&format!("  (all {} entries identical)\n", group_count));
            }

            // Variable values (only populated by template extraction).
            if !to_show[0].variable_values.is_empty() {
                md.push_str("\n**Variable values**:\n");
                for (var_name, values) in &to_show[0].variable_values {
                    md.push_str(&format!("- `{}`: ", var_name));
                    let value_str: Vec<String> =
                        values.iter().map(|v| format!("`{}`", v)).collect();
                    md.push_str(&value_str.join(", "));
                    md.push_str("\n");
                }
            }
        } else {
            md.push_str("**Sample entries**:\n\n");
            for (idx, sample) in to_show.iter().enumerate() {
                md.push_str(&format!("*Sample {}*:\n```\n", idx + 1));
                md.push_str(&sample.content);
                md.push_str("\n```\n\n");
            }
            if more > 0 {
                md.push_str(&format!("  (+ {} more distinct)\n", more));
            }
        }
    }

    /// Format grouped results (template extraction, clustering)
    fn format_grouped_results(
        md: &mut String,
        groups: &[crate::output::GroupOutput],
        outliers: &[crate::output::OutlierOutput],
        algorithm: &str,
    ) {
        // Pattern groups section
        md.push_str("## Pattern Groups\n\n");
        for group in groups {
            // Header with derived name, count and percentage
            md.push_str(&format!(
                "### {} ({} entries, {:.1}%)\n\n",
                group.name, group.count, group.percentage
            ));

            // Pattern (only show for template extraction, skip for clustering)
            let is_clustering = algorithm == "edit_distance_clustering";
            if !is_clustering {
                md.push_str("**Pattern**:\n```\n");
                md.push_str(&group.pattern);
                md.push_str("\n```\n\n");
            }

            // Line ranges
            if !group.line_ranges.is_empty() {
                md.push_str("**Line ranges**: ");
                let ranges: Vec<String> = group
                    .line_ranges
                    .iter()
                    .map(|(start, end)| {
                        if start == end {
                            format!("{}", start)
                        } else {
                            format!("{}-{}", start, end)
                        }
                    })
                    .collect();
                md.push_str(&ranges.join(", "));
                md.push_str("\n\n");
            }

            // Sample entries with deduplication
            if !group.samples.is_empty() {
                Self::format_samples(md, &group.samples, group.count);
            }

            md.push_str("\n");
        }

        // Outliers section
        if !outliers.is_empty() {
            md.push_str("## Outliers\n\n");
            md.push_str("Rare patterns that appear only once:\n\n");

            for outlier in outliers {
                md.push_str(&format!(
                    "### {} (Line {})\n\n",
                    outlier.id, outlier.line_number
                ));
                md.push_str(&format!("- **Reason**: {}\n", outlier.reason));
                md.push_str(&format!("- **Score**: {:.6}\n", outlier.score));
                md.push_str("\n```\n");
                md.push_str(&outlier.content);
                md.push_str("\n```\n\n");
            }
        }
    }

    /// Format outlier-focused results (n-gram analysis)
    fn format_outlier_focused_results(
        md: &mut String,
        baseline: &crate::output::BaselineOutput,
        outliers: &[crate::output::OutlierOutput],
    ) {
        // Baseline section
        md.push_str("## Baseline\n\n");
        md.push_str(&format!("{}\n\n", baseline.description));
        md.push_str(&format!(
            "- **Normal entries**: {} ({:.1}%)\n",
            baseline.normal_count, baseline.normal_percentage
        ));

        if !baseline.common_features.is_empty() {
            md.push_str("\n**Common features**:\n");
            for feature in &baseline.common_features {
                md.push_str(&format!("- `{}`\n", feature));
            }
        }

        // Show threshold information if available
        if let Some(ref threshold_info) = baseline.threshold {
            md.push_str("\n**Outlier detection**:\n");
            if threshold_info.auto_detected {
                md.push_str(&format!(
                    "- Threshold: {:.6} (auto-detected for bottom ~5%)\n",
                    threshold_info.value
                ));
            } else {
                md.push_str(&format!(
                    "- Threshold: {:.6} (user-specified)\n",
                    threshold_info.value
                ));
            }

            if let Some(ref stats) = threshold_info.score_stats {
                md.push_str(&format!(
                    "- Score range: {:.6} to {:.6} (mean: {:.6}, median: {:.6})\n",
                    stats.min, stats.max, stats.mean, stats.median
                ));
            }
        }

        md.push_str("\n");

        // Outliers section
        md.push_str("## Outliers\n\n");
        if outliers.is_empty() {
            md.push_str("No outliers detected.\n\n");
        } else {
            md.push_str(&format!(
                "{} entries with unusual patterns:\n\n",
                outliers.len()
            ));

            for outlier in outliers {
                md.push_str(&format!(
                    "### {} (Line {})\n\n",
                    outlier.id, outlier.line_number
                ));
                md.push_str(&format!("- **Reason**: {}\n", outlier.reason));
                md.push_str(&format!("- **Score**: {:.6}\n", outlier.score));
                md.push_str("\n```\n");
                md.push_str(&outlier.content);
                md.push_str("\n```\n\n");
            }
        }
    }

    /// Format schema-grouped results (JSON/structured data)
    fn format_schema_grouped_results(
        md: &mut String,
        schemas: &[crate::output::SchemaGroupOutput],
        outliers: &[crate::output::OutlierOutput],
    ) {
        // Schema groups section
        md.push_str("## Schema Groups\n\n");
        for schema in schemas {
            // Header with name, count and percentage
            md.push_str(&format!(
                "### {} ({} entries, {:.1}%)\n\n",
                schema.name, schema.count, schema.percentage
            ));

            // Schema description
            md.push_str("**Schema**:\n```\n");
            md.push_str(&schema.schema_description);
            md.push_str("\n```\n\n");

            // Sample values
            if !schema.sample_values.is_empty() {
                md.push_str("**Sample values**:\n");
                for (field, values) in &schema.sample_values {
                    if !values.is_empty() {
                        md.push_str(&format!("- `{}`: ", field));
                        let value_str: Vec<String> =
                            values.iter().map(|v| format!("`{}`", v)).collect();
                        md.push_str(&value_str.join(", "));
                        md.push_str("\n");
                    }
                }
                md.push_str("\n");
            }
        }

        // Outliers section
        if !outliers.is_empty() {
            md.push_str("## Outliers\n\n");
            md.push_str(&format!(
                "{} entries with unique schemas:\n\n",
                outliers.len()
            ));

            for outlier in outliers {
                md.push_str(&format!(
                    "### {} (Entry {})\n\n",
                    outlier.id, outlier.line_number
                ));
                md.push_str(&format!("- **Reason**: {}\n", outlier.reason));
                md.push_str("\n```json\n");
                md.push_str(&outlier.content);
                md.push_str("\n```\n\n");
            }
        }
    }

    /// Format path-grouped results (subtree algorithm)
    fn format_path_grouped_results(
        md: &mut String,
        patterns: &[crate::output::PathPatternOutput],
        singletons: &[crate::output::OutlierOutput],
    ) {
        md.push_str("## Subtree Patterns\n\n");

        if patterns.is_empty() {
            md.push_str("No repeated structural patterns found.\n\n");
        }

        for pattern in patterns {
            md.push_str(&format!(
                "### Pattern {} ({} objects, {:.1}%)\n\n",
                pattern.id, pattern.count, pattern.percentage
            ));

            md.push_str("**Schema**:\n```\n");
            md.push_str(&pattern.schema_description);
            md.push_str("\n```\n\n");

            md.push_str("**Appears at**:\n");
            for path in &pattern.paths {
                md.push_str(&format!("- `{}`\n", path));
            }
            md.push_str("\n");

            if !pattern.sample_values.is_empty() {
                md.push_str("**Sample values**:\n");
                for (field, values) in &pattern.sample_values {
                    if !values.is_empty() {
                        md.push_str(&format!("- `{}`: ", field));
                        let value_str: Vec<String> =
                            values.iter().map(|v| format!("`{}`", v)).collect();
                        md.push_str(&value_str.join(", "));
                        md.push_str("\n");
                    }
                }
                md.push_str("\n");
            }
        }

        if !singletons.is_empty() {
            md.push_str("## Singletons\n\n");
            md.push_str("Objects with a unique schema (appeared only once):\n\n");
            for singleton in singletons {
                md.push_str(&format!("- **{}**: {}\n", singleton.id, singleton.reason));
            }
            md.push_str("\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Entry;
    use crate::output::OutputBuilder;
    use crate::template::TemplateExtractor;

    #[test]
    fn test_markdown_formatter_basic() {
        let entries = vec![
            Entry::from_line("INFO User login".to_string(), 1),
            Entry::from_line("INFO User login".to_string(), 2),
            Entry::from_line("ERROR Connection failed".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries)
            .with_input_file("test.log".to_string())
            .build(&extractor);

        let markdown = MarkdownFormatter::format(&output);

        // Title changed
        assert!(markdown.contains("# txtfold output"));
        // Compact header line present
        assert!(markdown.contains("entries →"));
        assert!(markdown.contains("template_extraction"));
        // Results sections still present
        assert!(markdown.contains("## Pattern Groups"));
        assert!(markdown.contains("## Outliers"));
        // Old metadata/summary sections gone
        assert!(!markdown.contains("# txtfold Analysis Report"));
        assert!(!markdown.contains("## Metadata"));
        assert!(!markdown.contains("## Summary"));
        assert!(!markdown.contains("Unique patterns"));
    }

    #[test]
    fn test_markdown_formatter_groups() {
        let entries = vec![
            Entry::from_line("Request took 42 ms".to_string(), 1),
            Entry::from_line("Request took 100 ms".to_string(), 2),
            Entry::from_line("Request took 15 ms".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        // Should show the pattern in code block
        assert!(markdown.contains("**Pattern**:"));
        assert!(markdown.contains("Request took <NUM> ms"));

        // Compact header
        assert!(markdown.contains("3 entries →"));
        assert!(markdown.contains("100.0%"));

        // Should show sample entry
        assert!(markdown.contains("Sample entry"));
        assert!(markdown.contains("Request took 42 ms"));

        // Should show variable values
        assert!(markdown.contains("Variable values"));
    }

    #[test]
    fn test_markdown_formatter_outliers() {
        let entries = vec![
            Entry::from_line("INFO Normal message".to_string(), 1),
            Entry::from_line("INFO Normal message".to_string(), 2),
            Entry::from_line("ERROR Rare exception at line 42".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        // Should have outliers section
        assert!(markdown.contains("## Outliers"));
        assert!(markdown.contains("Rare patterns"));

        // Should show the outlier
        assert!(markdown.contains("ERROR Rare exception"));
        assert!(markdown.contains("Line 3"));
        assert!(markdown.contains("rare_pattern"));
    }

    #[test]
    fn test_markdown_formatter_line_ranges() {
        let entries = vec![
            Entry::from_line("Message".to_string(), 1),
            Entry::from_line("Message".to_string(), 2),
            Entry::from_line("Message".to_string(), 3),
            Entry::from_line("Message".to_string(), 10),
            Entry::from_line("Message".to_string(), 11),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        // Should show line ranges
        assert!(markdown.contains("Line ranges"));
        assert!(markdown.contains("1-3"));
        assert!(markdown.contains("10-11"));
    }

    #[test]
    fn test_markdown_formatter_deterministic() {
        let entries = vec![
            Entry::from_line("Test message".to_string(), 1),
            Entry::from_line("Test message".to_string(), 2),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries.clone()).build(&extractor);

        let md1 = MarkdownFormatter::format(&output);
        let md2 = MarkdownFormatter::format(&output);

        assert_eq!(md1, md2, "Markdown output should be deterministic");
    }

    #[test]
    fn test_markdown_formatter_no_outliers() {
        let entries = vec![
            Entry::from_line("Message".to_string(), 1),
            Entry::from_line("Message".to_string(), 2),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        // Should not show outliers section if there are no outliers
        assert!(!markdown.contains("## Outliers"));
        // But should still have the results section
        assert!(markdown.contains("## Pattern Groups"));
    }

    #[test]
    fn test_markdown_formatter_no_input_file() {
        let entries = vec![Entry::from_line("Message".to_string(), 1)];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        // Should not crash without input file
        assert!(markdown.contains("# txtfold output"));
        assert!(markdown.contains("→")); // compact header line present
    }

    #[test]
    fn test_budget_applied_shown_in_markdown() {
        let entries = vec![
            Entry::from_line("Alpha message kind".to_string(), 1),
            Entry::from_line("Alpha message kind".to_string(), 2),
            Entry::from_line("Alpha message kind".to_string(), 3),
            Entry::from_line("Beta message kind".to_string(), 4),
            Entry::from_line("Beta message kind".to_string(), 5),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        // FIXED_OVERHEAD(4) + SECTION_HEADER(2) + 1 * LINES_PER_GROUP(15) = 21
        // budget=21 fits exactly 1 of 2 groups → budget reached
        let output = OutputBuilder::new(entries).with_budget(21).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        assert!(markdown.contains("[budget: 21 lines]"));
    }

    #[test]
    fn test_budget_within_limit_not_shown_in_markdown() {
        let entries = vec![
            Entry::from_line("Only one group here".to_string(), 1),
            Entry::from_line("Only one group here".to_string(), 2),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).with_budget(200).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        // Budget not applied → no budget annotation in compact header
        assert!(!markdown.contains("[budget:"));
    }

    #[test]
    fn test_no_budget_annotation_without_budget() {
        let entries = vec![Entry::from_line("Message".to_string(), 1)];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        let output = OutputBuilder::new(entries).build(&extractor);
        let markdown = MarkdownFormatter::format(&output);

        assert!(!markdown.contains("[budget:"));
    }

    #[test]
    fn test_sample_dedup_all_identical() {
        use crate::output::{GroupOutput, SampleEntry, AlgorithmResults, AnalysisMetadata};
        use std::collections::HashMap;

        // Build a GroupOutput with 3 identical samples
        let identical_sample = SampleEntry {
            content: r#"{"category":"error"}"#.to_string(),
            line_numbers: vec![1],
            variable_values: HashMap::new(),
        };

        let group = GroupOutput {
            id: "group_0".to_string(),
            name: "error".to_string(),
            pattern: ".category = error".to_string(),
            count: 100,
            percentage: 100.0,
            samples: vec![identical_sample.clone(), identical_sample.clone(), identical_sample],
            line_ranges: vec![],
        };

        let output = AnalysisOutput {
            metadata: AnalysisMetadata {
                input_file: None,
                total_entries: 100,
                algorithm: "group_by(.category)".to_string(),
                reduction_ratio: 0.1,
                budget_lines: None,
                budget_applied: None,
            },
            summary: crate::output::AnalysisSummary {
                unique_patterns: 1,
                outliers: 0,
                largest_cluster: 100,
            },
            results: AlgorithmResults::Grouped {
                groups: vec![group],
                outliers: vec![],
            },
        };

        let markdown = MarkdownFormatter::format(&output);

        // Should show one sample and the "all identical" annotation
        assert!(markdown.contains("**Sample entry**:"));
        assert!(markdown.contains("(all 100 entries identical)"));
        // Should NOT show "Sample entries" (plural)
        assert!(!markdown.contains("**Sample entries**:"));
        // Should NOT show duplicate samples
        let count = markdown.matches(r#"{"category":"error"}"#).count();
        assert_eq!(count, 1, "identical content should appear exactly once");
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1000), "1,000");
        assert_eq!(format_count(1190), "1,190");
        assert_eq!(format_count(10000), "10,000");
        assert_eq!(format_count(1000000), "1,000,000");
    }
}

/// JSON formatter for analysis output
pub struct JsonFormatter;

impl JsonFormatter {
    /// Metadata describing this formatter
    pub const METADATA: FormatterMetadata = FormatterMetadata {
        name: "json",
        aliases: &[],
        description: "Machine-readable JSON format with full structured output",
        mime_type: "application/json",
        file_extension: "json",
        supports_streaming: false,
    };

    /// Format analysis output as JSON
    pub fn format(output: &AnalysisOutput) -> Result<String, String> {
        serde_json::to_string_pretty(output)
            .map_err(|e| format!("Failed to serialize to JSON: {}", e))
    }
}
