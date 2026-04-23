//! Tokenization for log lines and text entries

use crate::patterns::{PatternMatcher, PatternType};
use serde::{Deserialize, Serialize};

/// A token extracted from text
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Token {
    /// A literal word or symbol
    Literal(String),
    /// A detected number
    Number(String),
    /// A detected timestamp/date
    Timestamp(String),
    /// A detected date (YYYY-MM-DD)
    Date(String),
    /// A detected identifier (UUID, hash, etc.)
    Identifier(String),
    /// A detected duration (e.g., "42ms", "3.5s")
    Duration(String),
    /// A detected IP address
    IpAddress(String),
    /// Whitespace
    Whitespace,
    /// Punctuation
    Punctuation(char),
}

impl Token {
    /// Get the string representation of the token
    pub fn as_str(&self) -> &str {
        match self {
            Token::Literal(s)
            | Token::Number(s)
            | Token::Timestamp(s)
            | Token::Date(s)
            | Token::Identifier(s)
            | Token::Duration(s)
            | Token::IpAddress(s) => s,
            Token::Whitespace => " ",
            Token::Punctuation(c) => {
                // We need a static reference, so we'll handle common punctuation
                match c {
                    '[' => "[",
                    ']' => "]",
                    '(' => "(",
                    ')' => ")",
                    '{' => "{",
                    '}' => "}",
                    ':' => ":",
                    ',' => ",",
                    '.' => ".",
                    ';' => ";",
                    _ => "",
                }
            }
        }
    }

    /// Check if this token should be considered a variable slot in template extraction
    pub fn is_variable(&self) -> bool {
        matches!(
            self,
            Token::Number(_)
                | Token::Timestamp(_)
                | Token::Date(_)
                | Token::Identifier(_)
                | Token::Duration(_)
                | Token::IpAddress(_)
        )
    }
}

/// Tokenizer for breaking text into tokens
pub struct Tokenizer;

impl Tokenizer {
    /// Tokenize a single line into tokens
    /// Uses a two-pass approach: first extract timestamps, then tokenize rest
    pub fn tokenize(line: &str) -> Vec<Token> {
        // First pass: extract full timestamps and mark their positions
        let preprocessed = Self::preprocess_timestamps(line);

        // Second pass: normal tokenization on preprocessed string
        let mut tokens = Vec::new();
        let mut current_word = String::new();

        for ch in preprocessed.chars() {
            match ch {
                // Special marker for timestamp boundaries
                '\x00' => {
                    if !current_word.is_empty() {
                        tokens.push(Self::classify_word(&current_word));
                        current_word.clear();
                    }
                }
                // Punctuation gets its own token
                '[' | ']' | '(' | ')' | '{' | '}' | ',' | ';' => {
                    if !current_word.is_empty() {
                        tokens.push(Self::classify_word(&current_word));
                        current_word.clear();
                    }
                    tokens.push(Token::Punctuation(ch));
                }
                // Whitespace
                ' ' | '\t' => {
                    if !current_word.is_empty() {
                        tokens.push(Self::classify_word(&current_word));
                        current_word.clear();
                    }
                    tokens.push(Token::Whitespace);
                }
                // Regular characters (including : and . which are part of timestamps)
                _ => {
                    current_word.push(ch);
                }
            }
        }

        // Don't forget the last word
        if !current_word.is_empty() {
            tokens.push(Self::classify_word(&current_word));
        }

        tokens
    }

    /// Preprocess line to protect timestamps from being split
    /// Replaces timestamp patterns with marked versions
    fn preprocess_timestamps(line: &str) -> String {
        use regex::Regex;
        use std::sync::LazyLock;

        // Match common timestamp patterns
        static TIMESTAMP_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\d{4}-\d{2}-\d{2}[\sT]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})?|\d{2}:\d{2}:\d{2}(\.\d+)?").unwrap()
        });

        // Add boundary markers around timestamps
        let result = TIMESTAMP_PATTERN.replace_all(line, |caps: &regex::Captures| {
            format!("\x00{}\x00", &caps[0])
        });

        result.to_string()
    }

    /// Classify a word into the appropriate token type using pattern matching
    fn classify_word(word: &str) -> Token {
        match PatternMatcher::classify(word) {
            PatternType::Timestamp => Token::Timestamp(word.to_string()),
            PatternType::Date => Token::Date(word.to_string()),
            PatternType::Number => Token::Number(word.to_string()),
            PatternType::Identifier => Token::Identifier(word.to_string()),
            PatternType::Duration => Token::Duration(word.to_string()),
            PatternType::IpAddress => Token::IpAddress(word.to_string()),
            PatternType::None => Token::Literal(word.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_log() {
        let line = "[2024-01-15] INFO User login";
        let tokens = Tokenizer::tokenize(line);

        assert_eq!(tokens[0], Token::Punctuation('['));
        assert_eq!(tokens[1], Token::Date("2024-01-15".to_string()));
        assert_eq!(tokens[2], Token::Punctuation(']'));
        assert_eq!(tokens[3], Token::Whitespace);
        assert_eq!(tokens[4], Token::Literal("INFO".to_string()));
        assert_eq!(tokens[5], Token::Whitespace);
        assert_eq!(tokens[6], Token::Literal("User".to_string()));
        assert_eq!(tokens[7], Token::Whitespace);
        assert_eq!(tokens[8], Token::Literal("login".to_string()));
    }

    #[test]
    fn test_tokenize_with_numbers() {
        let line = "Request took 42 ms";
        let tokens = Tokenizer::tokenize(line);

        assert_eq!(tokens[0], Token::Literal("Request".to_string()));
        assert_eq!(tokens[1], Token::Whitespace);
        assert_eq!(tokens[2], Token::Literal("took".to_string()));
        assert_eq!(tokens[3], Token::Whitespace);
        assert_eq!(tokens[4], Token::Number("42".to_string()));
        assert_eq!(tokens[5], Token::Whitespace);
        assert_eq!(tokens[6], Token::Literal("ms".to_string()));
    }

    #[test]
    fn test_tokenize_with_uuid() {
        let line = "User abc123def456 logged in";
        let tokens = Tokenizer::tokenize(line);

        assert_eq!(tokens[0], Token::Literal("User".to_string()));
        assert_eq!(tokens[1], Token::Whitespace);
        assert_eq!(tokens[2], Token::Identifier("abc123def456".to_string()));
        assert_eq!(tokens[3], Token::Whitespace);
        assert_eq!(tokens[4], Token::Literal("logged".to_string()));
        assert_eq!(tokens[5], Token::Whitespace);
        assert_eq!(tokens[6], Token::Literal("in".to_string()));
    }

    #[test]
    fn test_tokenize_punctuation() {
        let line = "Error; {code, msg}";
        let tokens = Tokenizer::tokenize(line);

        // Note: ':' is no longer split as punctuation (kept with timestamps)
        // '.' is also kept with words (for floats/durations)
        assert!(tokens.contains(&Token::Punctuation(';')));
        assert!(tokens.contains(&Token::Punctuation('{')));
        assert!(tokens.contains(&Token::Punctuation('}')));
        assert!(tokens.contains(&Token::Punctuation(',')));
    }

    #[test]
    fn test_token_is_variable() {
        assert!(Token::Number("42".to_string()).is_variable());
        assert!(Token::Timestamp("2024-01-15".to_string()).is_variable());
        assert!(Token::Identifier("abc123".to_string()).is_variable());
        assert!(!Token::Literal("INFO".to_string()).is_variable());
        assert!(!Token::Whitespace.is_variable());
        assert!(!Token::Punctuation('[').is_variable());
    }

    #[test]
    fn test_tokenize_deterministic() {
        let line = "[2024-01-15 14:23:01] INFO User 12345 login";

        let tokens1 = Tokenizer::tokenize(line);
        let tokens2 = Tokenizer::tokenize(line);

        assert_eq!(tokens1, tokens2, "Tokenization should be deterministic");
    }

    #[test]
    fn test_tokenize_empty_line() {
        let tokens = Tokenizer::tokenize("");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_tokenize_whitespace_only() {
        let tokens = Tokenizer::tokenize("   ");
        assert_eq!(tokens.len(), 3);
        assert!(tokens.iter().all(|t| *t == Token::Whitespace));
    }
}
