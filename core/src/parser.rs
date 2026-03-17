//! Entry parser for detecting and grouping log entries

use crate::entry::Entry;
use regex::Regex;

/// Entry parsing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryMode {
    /// Each line is a separate entry
    SingleLine,
    /// Entries can span multiple lines (detected by timestamps + indentation)
    MultiLine,
    /// Auto-detect based on file content
    Auto,
}

/// Parser for grouping lines into entries
pub struct EntryParser {
    mode: EntryMode,
}

impl EntryParser {
    /// Create a new parser with the specified mode
    pub fn new(mode: EntryMode) -> Self {
        EntryParser { mode }
    }

    /// Parse content into entries
    pub fn parse(&self, content: &str) -> Vec<Entry> {
        let mode = match self.mode {
            EntryMode::Auto => detect_entry_mode(content),
            mode => mode,
        };

        match mode {
            EntryMode::SingleLine => parse_single_line(content),
            EntryMode::MultiLine => parse_multi_line(content),
            EntryMode::Auto => unreachable!("Auto mode should be resolved by now"),
        }
    }
}

/// Detect whether content should be parsed as single-line or multi-line
fn detect_entry_mode(content: &str) -> EntryMode {
    let sample_size = 100;
    let lines: Vec<&str> = content.lines().take(sample_size).collect();

    if lines.len() < 2 {
        return EntryMode::SingleLine;
    }

    let mut timestamp_lines = 0;
    let mut indented_lines = 0;

    for line in &lines {
        if has_timestamp_prefix(line) {
            timestamp_lines += 1;
        }
        if is_continuation_line(line) {
            indented_lines += 1;
        }
    }

    // If we have timestamps and significant indented lines, use multi-line mode
    // Heuristic: >10% timestamp lines and >20% indented lines suggests multi-line entries
    // (Each multi-line entry has 1 timestamp + multiple indented lines)
    let timestamp_ratio = timestamp_lines as f64 / lines.len() as f64;
    let indented_ratio = indented_lines as f64 / lines.len() as f64;

    if timestamp_ratio > 0.10 && indented_ratio > 0.20 {
        EntryMode::MultiLine
    } else {
        EntryMode::SingleLine
    }
}

/// Check if a line starts with a timestamp pattern
fn has_timestamp_prefix(line: &str) -> bool {
    lazy_static::lazy_static! {
        static ref TIMESTAMP_RE: Regex = Regex::new(
            r"^[\[\(]?\d{4}[-/]\d{2}[-/]\d{2}[T\s]\d{2}:\d{2}:\d{2}"
        ).unwrap();
    }

    TIMESTAMP_RE.is_match(line)
}

/// Check if a line is a continuation line (indented or special markers)
fn is_continuation_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }

    // Check for leading whitespace
    if line.starts_with(' ') || line.starts_with('\t') {
        return true;
    }

    // Check for common stack trace markers
    let trimmed = line.trim_start();
    if trimmed.starts_with("at ")
        || trimmed.starts_with("File \"")
        || trimmed.starts_with("Caused by:")
        || trimmed.starts_with("...")
    {
        return true;
    }

    false
}

/// Parse content as single-line entries
fn parse_single_line(content: &str) -> Vec<Entry> {
    content
        .lines()
        .enumerate()
        .map(|(idx, line)| Entry::from_line(line.to_string(), idx + 1))
        .collect()
}

/// Parse content as multi-line entries
fn parse_multi_line(content: &str) -> Vec<Entry> {
    let lines: Vec<&str> = content.lines().collect();

    // Check if file has any timestamps - if not, fall back to single-line mode
    let has_any_timestamps = lines.iter().any(|line| has_timestamp_prefix(line));
    if !has_any_timestamps {
        return parse_single_line(content);
    }

    let mut entries = Vec::new();
    let mut current_entry_lines = Vec::new();
    let mut current_line_numbers = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;

        // Check if this starts a new entry
        if has_timestamp_prefix(line) && !current_entry_lines.is_empty() {
            // Save the previous entry
            entries.push(Entry::from_lines(
                current_entry_lines.clone(),
                current_line_numbers.clone(),
            ));
            current_entry_lines.clear();
            current_line_numbers.clear();
        }

        // Add line to current entry
        current_entry_lines.push(line.to_string());
        current_line_numbers.push(line_number);
    }

    // Don't forget the last entry
    if !current_entry_lines.is_empty() {
        entries.push(Entry::from_lines(current_entry_lines, current_line_numbers));
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_line_mode() {
        let content = "Line 1\nLine 2\nLine 3\n";
        let parser = EntryParser::new(EntryMode::SingleLine);
        let entries = parser.parse(content);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].first_line(), Some("Line 1"));
        assert_eq!(entries[1].first_line(), Some("Line 2"));
        assert_eq!(entries[2].first_line(), Some("Line 3"));
    }

    #[test]
    fn test_multi_line_mode() {
        let content = "\
[2024-01-15 10:00:00] ERROR Exception
  at line 42
  in module foo
[2024-01-15 10:00:01] INFO Success
";
        let parser = EntryParser::new(EntryMode::MultiLine);
        let entries = parser.parse(content);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content.len(), 3);
        assert_eq!(
            entries[0].first_line(),
            Some("[2024-01-15 10:00:00] ERROR Exception")
        );
        assert_eq!(entries[1].content.len(), 1);
        assert_eq!(
            entries[1].first_line(),
            Some("[2024-01-15 10:00:01] INFO Success")
        );
    }

    #[test]
    fn test_timestamp_detection() {
        assert!(has_timestamp_prefix("[2024-01-15 10:00:00] INFO"));
        assert!(has_timestamp_prefix("2024-01-15 10:00:00 INFO"));
        assert!(has_timestamp_prefix("2024/01/15 10:00:00 INFO"));
        assert!(has_timestamp_prefix("[2024-01-15T10:00:00] INFO"));
        assert!(!has_timestamp_prefix("INFO no timestamp"));
        assert!(!has_timestamp_prefix("  [2024-01-15 10:00:00] indented"));
    }

    #[test]
    fn test_continuation_line_detection() {
        assert!(is_continuation_line("  at line 42"));
        assert!(is_continuation_line("\tat line 42"));
        assert!(is_continuation_line("  File \"foo.py\", line 10"));
        assert!(is_continuation_line("Caused by: something"));
        assert!(!is_continuation_line("[2024-01-15] INFO"));
        assert!(!is_continuation_line("Regular line"));
    }

    #[test]
    fn test_auto_detection_single_line() {
        let content = "\
Line 1
Line 2
Line 3
";
        let parser = EntryParser::new(EntryMode::Auto);
        let entries = parser.parse(content);

        // Should detect as single-line (no timestamps, no indentation)
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_auto_detection_multi_line() {
        let content = "\
[2024-01-15 10:00:00] ERROR Exception
  at line 42
  in module foo
[2024-01-15 10:00:01] ERROR Another
  at line 100
[2024-01-15 10:00:02] INFO Success
  details here
[2024-01-15 10:00:03] ERROR Third
  stack trace
  more lines
";
        let parser = EntryParser::new(EntryMode::Auto);
        let entries = parser.parse(content);

        // Should detect as multi-line (timestamps + indentation)
        assert_eq!(entries.len(), 4);
        assert!(entries[0].content.len() > 1);
    }

    #[test]
    fn test_java_stack_trace() {
        let content = "\
[2024-01-15 10:00:00] ERROR Exception in thread \"main\"
java.lang.NullPointerException: Cannot invoke method
\tat com.example.Main.process(Main.java:42)
\tat com.example.Main.main(Main.java:10)
[2024-01-15 10:00:01] INFO Application started
";
        let parser = EntryParser::new(EntryMode::MultiLine);
        let entries = parser.parse(content);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content.len(), 4);
        assert_eq!(entries[1].content.len(), 1);
    }

    #[test]
    fn test_python_traceback() {
        let content = "\
[2024-01-15 10:00:00] ERROR Traceback (most recent call last):
  File \"script.py\", line 42, in main
    result = process()
  File \"script.py\", line 20, in process
    return data[key]
KeyError: 'missing_key'
[2024-01-15 10:00:01] INFO Recovered
";
        let parser = EntryParser::new(EntryMode::MultiLine);
        let entries = parser.parse(content);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content.len(), 6);
        assert_eq!(entries[1].content.len(), 1);
    }

    #[test]
    fn test_multiline_without_timestamps_falls_back() {
        let content = "\
User alice logged in
Processing request for user bob
Database query completed
User charlie logged out
";
        let parser = EntryParser::new(EntryMode::MultiLine);
        let entries = parser.parse(content);

        // Should fall back to single-line mode (one entry per line)
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].first_line(), Some("User alice logged in"));
        assert_eq!(entries[1].first_line(), Some("Processing request for user bob"));
        assert_eq!(entries[2].first_line(), Some("Database query completed"));
        assert_eq!(entries[3].first_line(), Some("User charlie logged out"));
    }
}
