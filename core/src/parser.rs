//! Entry parser for detecting and grouping log entries

use crate::entry::Entry;
use crate::metadata::{InputFormatMetadata, SubOption};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

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
    /// Optional regex for detecting entry boundaries in block mode.
    entry_pattern: Option<Regex>,
}

impl EntryParser {
    // ── Public format-family metadata (json / line / block) ──────────────────

    /// Metadata for JSON input format (array or map — internal heuristic picks between them).
    pub const JSON_FORMAT: InputFormatMetadata = InputFormatMetadata {
        name: "json",
        aliases: &["json-array", "json-map"],
        description: "JSON input — array of objects or a map/object. Path selection applies.",
        sub_options: &[],
    };

    /// Metadata for line-delimited input format (one entry per line).
    pub const LINE_FORMAT: InputFormatMetadata = InputFormatMetadata {
        name: "line",
        aliases: &["log", "logs", "text"],
        description: "Line-delimited input — one entry per line (logs, CSV)",
        sub_options: &[],
    };

    /// Metadata for block input format (multi-line entries).
    pub const BLOCK_FORMAT: InputFormatMetadata = InputFormatMetadata {
        name: "block",
        aliases: &["multiline", "multi-line"],
        description:
            "Multi-line entries — boundaries declared via --entry-pattern <regex> or \
             detected by the multiline heuristic (timestamp + indentation) as a fallback",
        sub_options: &[SubOption {
            name: "entry-pattern",
            values: &[],
            default: "",
            description: "Regex that matches the start of each new entry",
        }],
    };

    // ── Legacy format metadata (kept for internal use) ────────────────────────

    #[doc(hidden)]
    pub const TEXT_FORMAT: InputFormatMetadata = InputFormatMetadata {
        name: "text",
        aliases: &[],
        description: "Plain text log files (legacy name — use 'line')",
        sub_options: &[],
    };

    #[doc(hidden)]
    pub const JSON_ARRAY_FORMAT: InputFormatMetadata = InputFormatMetadata {
        name: "json-array",
        aliases: &[],
        description: "JSON array of objects (legacy name — use 'json')",
        sub_options: &[],
    };

    #[doc(hidden)]
    pub const JSON_MAP_FORMAT: InputFormatMetadata = InputFormatMetadata {
        name: "json-map",
        aliases: &[],
        description: "JSON object/map where each value is analyzed (legacy name — use 'json')",
        sub_options: &[],
    };

    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a new parser with the specified mode.
    pub fn new(mode: EntryMode) -> Self {
        EntryParser { mode, entry_pattern: None }
    }

    /// Set a custom entry-boundary pattern for block mode.
    ///
    /// When set, lines matching this regex are treated as the start of a new
    /// entry instead of using the default timestamp heuristic.
    ///
    /// Returns an error if `pattern` is not a valid regex.
    pub fn with_entry_pattern(mut self, pattern: &str) -> Result<Self, String> {
        self.entry_pattern = Some(
            Regex::new(pattern)
                .map_err(|e| format!("Invalid entry pattern '{}': {}", pattern, e))?,
        );
        Ok(self)
    }

    /// Parse content into entries
    pub fn parse(&self, content: &str) -> Vec<Entry> {
        let mode = match self.mode {
            EntryMode::Auto => detect_entry_mode(content),
            mode => mode,
        };

        match mode {
            EntryMode::SingleLine => parse_single_line(content),
            EntryMode::MultiLine => {
                if let Some(pattern) = &self.entry_pattern {
                    parse_multi_line_with_pattern(content, pattern)
                } else {
                    parse_multi_line(content)
                }
            }
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
    static TIMESTAMP_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[\[\(]?\d{4}[-/]\d{2}[-/]\d{2}[T\s]\d{2}:\d{2}:\d{2}").unwrap()
    });

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

/// Parse content as multi-line entries using a custom entry-boundary pattern.
///
/// Lines that match `pattern` start a new entry; all subsequent lines until
/// the next match are continuation lines of that entry.
fn parse_multi_line_with_pattern(content: &str, pattern: &Regex) -> Vec<Entry> {
    let lines: Vec<&str> = content.lines().collect();

    let mut entries = Vec::new();
    let mut current_entry_lines: Vec<String> = Vec::new();
    let mut current_line_numbers: Vec<usize> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;

        if pattern.is_match(line) && !current_entry_lines.is_empty() {
            entries.push(Entry::from_lines(
                current_entry_lines.clone(),
                current_line_numbers.clone(),
            ));
            current_entry_lines.clear();
            current_line_numbers.clear();
        }

        current_entry_lines.push(line.to_string());
        current_line_numbers.push(line_number);
    }

    if !current_entry_lines.is_empty() {
        entries.push(Entry::from_lines(current_entry_lines, current_line_numbers));
    }

    // If the pattern never matched, fall back to single-line mode
    if entries.len() == 1 && entries[0].content.len() == lines.len() {
        return parse_single_line(content);
    }

    entries
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

/// Parse JSON array into entries (each array element becomes an entry)
pub fn parse_json_array(content: &str) -> Result<Vec<Value>, String> {
    let parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    match parsed {
        Value::Array(arr) => Ok(arr),
        Value::Object(_) => {
            // Single object, wrap in array
            Ok(vec![parsed])
        }
        _ => {
            // Unexpected JSON type
            Err("Expected JSON array or object, got other type".to_string())
        }
    }
}

/// Parse JSON map/object into entries (each value becomes an entry)
/// Returns (values, keys) tuple
pub fn parse_json_map(content: &str) -> Result<(Vec<Value>, Vec<String>), String> {
    let parsed: Value = serde_json::from_str(content)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    match parsed {
        Value::Object(map) => {
            let mut keys = Vec::new();
            let mut values = Vec::new();

            for (key, value) in map {
                keys.push(key);
                values.push(value);
            }

            Ok((values, keys))
        }
        _ => {
            Err("Expected JSON object/map, got other type".to_string())
        }
    }
}

/// Detect if content is JSON (array or object)
pub fn is_json(content: &str) -> bool {
    let trimmed = content.trim();
    (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
}

/// Detect if JSON content is a map (object with values that are objects)
/// vs an array or a single object entry
pub fn is_json_map(content: &str) -> bool {
    if let Ok(parsed) = serde_json::from_str::<Value>(content) {
        if let Value::Object(map) = parsed {
            // Check if at least one value is an object (suggests map structure)
            // and we have multiple keys
            if map.len() > 1 {
                let object_values = map.values()
                    .filter(|v| matches!(v, Value::Object(_)))
                    .count();
                // If majority of values are objects, treat as map
                return object_values as f64 / map.len() as f64 > 0.5;
            }
        }
    }
    false
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

    #[test]
    fn test_json_array_parsing() {
        let content = r#"[
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]"#;

        let values = parse_json_array(content).unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["name"], "Alice");
        assert_eq!(values[1]["name"], "Bob");
    }

    #[test]
    fn test_json_single_object() {
        let content = r#"{"name": "Alice", "age": 30}"#;

        let values = parse_json_array(content).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["name"], "Alice");
    }

    #[test]
    fn test_is_json_detection() {
        assert!(is_json(r#"[{"key": "value"}]"#));
        assert!(is_json(r#"{"key": "value"}"#));
        assert!(is_json("  [1, 2, 3]  "));
        assert!(!is_json("plain text"));
        assert!(!is_json("[incomplete"));
    }

    #[test]
    fn test_json_map_parsing() {
        let content = r#"{
            "user_1": {"name": "Alice", "age": 30},
            "user_2": {"name": "Bob", "age": 25}
        }"#;

        let (values, keys) = parse_json_map(content).unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"user_1".to_string()));
        assert!(keys.contains(&"user_2".to_string()));
    }

    #[test]
    fn test_is_json_map_detection() {
        // Map with object values
        let map_content = r#"{
            "user_1": {"name": "Alice"},
            "user_2": {"name": "Bob"}
        }"#;
        assert!(is_json_map(map_content));

        // Array (not a map)
        let array_content = r#"[{"name": "Alice"}, {"name": "Bob"}]"#;
        assert!(!is_json_map(array_content));

        // Single object (not a map)
        let single_content = r#"{"name": "Alice", "age": 30}"#;
        assert!(!is_json_map(single_content));

        // Map with mixed values (mostly objects)
        let mixed_content = r#"{
            "user_1": {"name": "Alice"},
            "user_2": {"name": "Bob"},
            "count": 2
        }"#;
        assert!(is_json_map(mixed_content));
    }
}
