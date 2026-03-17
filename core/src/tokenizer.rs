//! Tokenization for log lines and text entries

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
    /// A detected identifier (UUID, hash, etc.)
    Identifier(String),
    /// Whitespace
    Whitespace,
    /// Punctuation
    Punctuation(char),
}

impl Token {
    /// Get the string representation of the token
    pub fn as_str(&self) -> &str {
        match self {
            Token::Literal(s) | Token::Number(s) | Token::Timestamp(s) | Token::Identifier(s) => s,
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
            Token::Number(_) | Token::Timestamp(_) | Token::Identifier(_)
        )
    }
}

/// Tokenizer for breaking text into tokens
pub struct Tokenizer;

impl Tokenizer {
    /// Tokenize a single line into tokens
    pub fn tokenize(line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();

        for ch in line.chars() {
            match ch {
                // Punctuation gets its own token
                '[' | ']' | '(' | ')' | '{' | '}' | ':' | ',' | '.' | ';' => {
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
                // Regular characters
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

    /// Classify a word into the appropriate token type
    fn classify_word(word: &str) -> Token {
        // Check if it looks like a timestamp first (contains digits and dashes/colons)
        // This must come before number check since timestamps like "2024-01-15" contain digits and dashes
        if word.contains('-') || word.contains(':') {
            let has_digit = word.chars().any(|c| c.is_ascii_digit());
            let has_separator = word.contains('-') || word.contains(':');
            if has_digit && has_separator {
                return Token::Timestamp(word.to_string());
            }
        }

        // Check if it's a pure number
        if word.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
            // But make sure it's not just dashes or dots
            if word.chars().any(|c| c.is_ascii_digit()) {
                return Token::Number(word.to_string());
            }
        }

        // Check if it looks like a UUID or hash (hexadecimal, certain length patterns)
        if word.len() >= 8 {
            let hex_count = word.chars().filter(|c| c.is_ascii_hexdigit()).count();
            let total_count = word.chars().count();
            if hex_count == total_count || (word.contains('-') && hex_count > total_count / 2) {
                return Token::Identifier(word.to_string());
            }
        }

        // Default: literal
        Token::Literal(word.to_string())
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
        assert_eq!(tokens[1], Token::Timestamp("2024-01-15".to_string()));
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
        let line = "Error: {code: 500, msg: timeout}";
        let tokens = Tokenizer::tokenize(line);

        assert!(tokens.contains(&Token::Punctuation(':')));
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
