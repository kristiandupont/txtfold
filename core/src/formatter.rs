//! Formatters for converting analysis output to human-readable formats

use crate::output::AnalysisOutput;

/// Markdown formatter for analysis output
pub struct MarkdownFormatter;

impl MarkdownFormatter {
    /// Format analysis output as Markdown
    pub fn format(output: &AnalysisOutput) -> String {
        let mut md = String::new();

        // Title
        md.push_str("# txtfold Analysis Report\n\n");

        // Metadata section
        md.push_str("## Metadata\n\n");
        if let Some(ref filename) = output.metadata.input_file {
            md.push_str(&format!("- **Input file**: `{}`\n", filename));
        }
        md.push_str(&format!(
            "- **Total entries**: {}\n",
            output.metadata.total_entries
        ));
        md.push_str(&format!(
            "- **Algorithm**: {}\n",
            output.metadata.algorithm
        ));
        md.push_str(&format!(
            "- **Compression ratio**: {:.2}%\n",
            output.metadata.compression_ratio * 100.0
        ));
        md.push_str("\n");

        // Summary section
        md.push_str("## Summary\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!(
            "| Unique patterns | {} |\n",
            output.summary.unique_patterns
        ));
        md.push_str(&format!("| Outliers | {} |\n", output.summary.outliers));
        md.push_str(&format!(
            "| Largest cluster | {} |\n",
            output.summary.largest_cluster
        ));
        md.push_str("\n");

        // Pattern groups section
        md.push_str("## Pattern Groups\n\n");
        for group in &output.groups {
            // Header with derived name, count and percentage
            md.push_str(&format!(
                "### {} ({} entries, {:.1}%)\n\n",
                group.name, group.count, group.percentage
            ));

            // Pattern in code block
            md.push_str("**Pattern**:\n```\n");
            md.push_str(&group.pattern);
            md.push_str("\n```\n\n");

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

            // Sample entries
            if !group.samples.is_empty() {
                md.push_str("**Sample entry**:\n```\n");
                md.push_str(&group.samples[0].content);
                md.push_str("\n```\n");

                // Variable values
                if !group.samples[0].variable_values.is_empty() {
                    md.push_str("\n**Variable values**:\n");
                    for (var_name, values) in &group.samples[0].variable_values {
                        md.push_str(&format!("- `{}`: ", var_name));
                        let value_str: Vec<String> =
                            values.iter().map(|v| format!("`{}`", v)).collect();
                        md.push_str(&value_str.join(", "));
                        md.push_str("\n");
                    }
                }
            }

            md.push_str("\n");
        }

        // Outliers section
        if !output.outliers.is_empty() {
            md.push_str("## Outliers\n\n");
            md.push_str("Rare patterns that appear only once:\n\n");

            for outlier in &output.outliers {
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

        md
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

        // Check that key sections are present
        assert!(markdown.contains("# txtfold Analysis Report"));
        assert!(markdown.contains("## Metadata"));
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("## Pattern Groups"));
        assert!(markdown.contains("## Outliers"));

        // Check metadata
        assert!(markdown.contains("test.log"));
        assert!(markdown.contains("Total entries**: 3"));
        assert!(markdown.contains("template_extraction"));

        // Check summary table
        assert!(markdown.contains("Unique patterns"));
        assert!(markdown.contains("Outliers"));
        assert!(markdown.contains("Largest cluster"));
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

        // Should show count and percentage in header
        assert!(markdown.contains("3 entries"));
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
        // But should still have the other sections
        assert!(markdown.contains("## Summary"));
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
        assert!(markdown.contains("# txtfold Analysis Report"));
        assert!(markdown.contains("Total entries"));
    }
}
