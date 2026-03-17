//! Entry representation for log lines and text records

use serde::{Deserialize, Serialize};

/// Metadata associated with an entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Original line number(s) in the input
    pub line_numbers: Vec<usize>,
    /// Detected fields (optional, for structured formats)
    pub fields: Option<std::collections::HashMap<String, String>>,
}

/// A single entry (log line or text record)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The content of the entry (one or more lines)
    pub content: Vec<String>,
    /// Optional metadata about the entry
    pub metadata: Option<Metadata>,
}

impl Entry {
    /// Create a new entry from a single line
    pub fn from_line(line: String, line_number: usize) -> Self {
        Entry {
            content: vec![line],
            metadata: Some(Metadata {
                line_numbers: vec![line_number],
                fields: None,
            }),
        }
    }

    /// Create a new entry from multiple lines
    pub fn from_lines(lines: Vec<String>, line_numbers: Vec<usize>) -> Self {
        Entry {
            content: lines,
            metadata: Some(Metadata {
                line_numbers,
                fields: None,
            }),
        }
    }

    /// Get the full content as a single string
    pub fn as_single_string(&self) -> String {
        self.content.join("\n")
    }

    /// Get the first line of content
    pub fn first_line(&self) -> Option<&str> {
        self.content.first().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_from_line() {
        let entry = Entry::from_line("[2024-01-15] INFO User login".to_string(), 1);

        assert_eq!(entry.content.len(), 1);
        assert_eq!(entry.content[0], "[2024-01-15] INFO User login");
        assert_eq!(entry.first_line(), Some("[2024-01-15] INFO User login"));

        let metadata = entry.metadata.unwrap();
        assert_eq!(metadata.line_numbers, vec![1]);
        assert_eq!(metadata.fields, None);
    }

    #[test]
    fn test_entry_from_lines() {
        let lines = vec![
            "[2024-01-15] ERROR Exception".to_string(),
            "  at line 42".to_string(),
            "  in module foo".to_string(),
        ];
        let entry = Entry::from_lines(lines.clone(), vec![10, 11, 12]);

        assert_eq!(entry.content, lines);
        assert_eq!(entry.first_line(), Some("[2024-01-15] ERROR Exception"));

        let metadata = entry.metadata.unwrap();
        assert_eq!(metadata.line_numbers, vec![10, 11, 12]);
    }

    #[test]
    fn test_entry_as_single_string() {
        let lines = vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
        ];
        let entry = Entry::from_lines(lines, vec![1, 2, 3]);

        assert_eq!(entry.as_single_string(), "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_entry_serialization() {
        let entry = Entry::from_line("Test log line".to_string(), 42);

        // Should serialize to JSON
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Test log line"));
        assert!(json.contains("42"));

        // Should deserialize back
        let deserialized: Entry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, entry);
    }
}
