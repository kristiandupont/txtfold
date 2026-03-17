//! Pattern matching for token classification

use once_cell::sync::Lazy;
use regex::Regex;

/// Compiled regex patterns for token classification
/// Order matters - more specific patterns should be checked first
pub struct PatternMatcher;

// Timestamp patterns
static TIMESTAMP_FULL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}[\sT]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})?$").unwrap()
});

static TIMESTAMP_TIME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d+)?$").unwrap()
});

static TIMESTAMP_DATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap()
});

// UUID pattern
static UUID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()
});

// Duration patterns (e.g., "42ms", "3.5s", "100MB")
static DURATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d+(\.\d+)?(ms|s|m|h|d|MB|GB|KB|B)$").unwrap()
});

// IP address patterns
static IPV4: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap()
});

static IPV6: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f:]+::[0-9a-f:]*$").unwrap()
});

// Hash patterns (hex strings of common lengths)
static HASH_MD5: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f]{32}$").unwrap()
});

static HASH_SHA1: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f]{40}$").unwrap()
});

static HASH_SHA256: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-f]{64}$").unwrap()
});

// Number patterns (integer or float)
static NUMBER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^-?\d+(\.\d+)?$").unwrap()
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    Timestamp,
    Number,
    Identifier,
    Duration,
    IpAddress,
    None,
}

impl PatternMatcher {
    /// Classify a word using pattern matching
    /// Returns the pattern type and priority (lower = higher priority)
    pub fn classify(word: &str) -> PatternType {
        // Check patterns in order of specificity (most specific first)

        // Full timestamps (most specific)
        if TIMESTAMP_FULL.is_match(word) {
            return PatternType::Timestamp;
        }

        // Time-only timestamps
        if TIMESTAMP_TIME.is_match(word) {
            return PatternType::Timestamp;
        }

        // Date-only timestamps
        if TIMESTAMP_DATE.is_match(word) {
            return PatternType::Timestamp;
        }

        // UUIDs
        if UUID.is_match(word) {
            return PatternType::Identifier;
        }

        // Hashes
        if HASH_MD5.is_match(word) || HASH_SHA1.is_match(word) || HASH_SHA256.is_match(word) {
            return PatternType::Identifier;
        }

        // Durations (check before numbers since "42ms" contains digits)
        if DURATION.is_match(word) {
            return PatternType::Duration;
        }

        // IP addresses
        if IPV4.is_match(word) || IPV6.is_match(word) {
            return PatternType::IpAddress;
        }

        // Plain numbers
        if NUMBER.is_match(word) {
            return PatternType::Number;
        }

        // Fallback: check for hex identifiers (8+ chars, all hex)
        if word.len() >= 8 && word.chars().all(|c| c.is_ascii_hexdigit()) {
            return PatternType::Identifier;
        }

        PatternType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_full() {
        assert_eq!(
            PatternMatcher::classify("2024-01-15 10:23:45"),
            PatternType::Timestamp
        );
        assert_eq!(
            PatternMatcher::classify("2024-01-15T10:23:45"),
            PatternType::Timestamp
        );
        assert_eq!(
            PatternMatcher::classify("2024-01-15T10:23:45.123Z"),
            PatternType::Timestamp
        );
        assert_eq!(
            PatternMatcher::classify("2024-01-15T10:23:45+00:00"),
            PatternType::Timestamp
        );
    }

    #[test]
    fn test_timestamp_time() {
        assert_eq!(
            PatternMatcher::classify("10:23:45"),
            PatternType::Timestamp
        );
        assert_eq!(
            PatternMatcher::classify("10:23:45.123"),
            PatternType::Timestamp
        );
    }

    #[test]
    fn test_timestamp_date() {
        assert_eq!(
            PatternMatcher::classify("2024-01-15"),
            PatternType::Timestamp
        );
    }

    #[test]
    fn test_uuid() {
        assert_eq!(
            PatternMatcher::classify("550e8400-e29b-41d4-a716-446655440000"),
            PatternType::Identifier
        );
    }

    #[test]
    fn test_duration() {
        assert_eq!(PatternMatcher::classify("42ms"), PatternType::Duration);
        assert_eq!(PatternMatcher::classify("3.5s"), PatternType::Duration);
        assert_eq!(PatternMatcher::classify("100MB"), PatternType::Duration);
        assert_eq!(PatternMatcher::classify("5m"), PatternType::Duration);
    }

    #[test]
    fn test_ipv4() {
        assert_eq!(
            PatternMatcher::classify("192.168.1.1"),
            PatternType::IpAddress
        );
        assert_eq!(
            PatternMatcher::classify("10.0.0.1"),
            PatternType::IpAddress
        );
    }

    #[test]
    fn test_hash() {
        // MD5
        assert_eq!(
            PatternMatcher::classify("d41d8cd98f00b204e9800998ecf8427e"),
            PatternType::Identifier
        );
        // SHA1
        assert_eq!(
            PatternMatcher::classify("da39a3ee5e6b4b0d3255bfef95601890afd80709"),
            PatternType::Identifier
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(PatternMatcher::classify("42"), PatternType::Number);
        assert_eq!(PatternMatcher::classify("3.14"), PatternType::Number);
        assert_eq!(PatternMatcher::classify("-100"), PatternType::Number);
    }

    #[test]
    fn test_hex_identifier() {
        assert_eq!(
            PatternMatcher::classify("abc123def456"),
            PatternType::Identifier
        );
        assert_eq!(
            PatternMatcher::classify("deadbeef"),
            PatternType::Identifier
        );
    }

    #[test]
    fn test_non_pattern() {
        assert_eq!(PatternMatcher::classify("hello"), PatternType::None);
        assert_eq!(PatternMatcher::classify("GET"), PatternType::None);
        assert_eq!(PatternMatcher::classify("/api/users"), PatternType::None);
    }

    #[test]
    fn test_priority_order() {
        // "42ms" should be Duration, not Number
        assert_eq!(PatternMatcher::classify("42ms"), PatternType::Duration);

        // "2024-01-15" should be Timestamp, not Number sequence
        assert_eq!(
            PatternMatcher::classify("2024-01-15"),
            PatternType::Timestamp
        );
    }
}
