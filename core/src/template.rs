//! Template extraction algorithm for identifying patterns in log entries

use crate::entry::Entry;
use crate::tokenizer::{Token, Tokenizer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A template pattern extracted from log entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Template {
    /// The pattern tokens (mix of literals and variable placeholders)
    pub tokens: Vec<TemplateToken>,
    /// Human-readable pattern string
    pub pattern: String,
}

/// A token in a template (either fixed or variable)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemplateToken {
    /// A fixed literal token that must match exactly
    Fixed(String),
    /// A variable slot that can match different values
    Variable(VariableType),
}

/// Types of variables that can appear in templates
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariableType {
    /// A numeric value
    Number,
    /// A timestamp or date
    Timestamp,
    /// A date (YYYY-MM-DD)
    Date,
    /// An identifier (UUID, hash, etc.)
    Identifier,
    /// A duration value (e.g., "42ms", "3.5s")
    Duration,
    /// An IP address
    IpAddress,
    /// Any other variable content
    Any,
}

impl Template {
    /// Create a template from a sequence of tokens
    pub fn from_tokens(tokens: &[Token]) -> Self {
        let template_tokens: Vec<TemplateToken> = tokens
            .iter()
            .map(|token| match token {
                Token::Number(_) => TemplateToken::Variable(VariableType::Number),
                Token::Timestamp(_) => TemplateToken::Variable(VariableType::Timestamp),
                Token::Date(_) => TemplateToken::Variable(VariableType::Date),
                Token::Identifier(_) => TemplateToken::Variable(VariableType::Identifier),
                Token::Duration(_) => TemplateToken::Variable(VariableType::Duration),
                Token::IpAddress(_) => TemplateToken::Variable(VariableType::IpAddress),
                Token::Literal(s) => TemplateToken::Fixed(s.clone()),
                Token::Whitespace => TemplateToken::Fixed(" ".to_string()),
                Token::Punctuation(c) => TemplateToken::Fixed(c.to_string()),
            })
            .collect();

        let pattern = Self::tokens_to_pattern(&template_tokens);

        Template {
            tokens: template_tokens,
            pattern,
        }
    }

    /// Convert template tokens to a human-readable pattern string
    fn tokens_to_pattern(tokens: &[TemplateToken]) -> String {
        tokens
            .iter()
            .map(|token| match token {
                TemplateToken::Fixed(s) => s.clone(),
                TemplateToken::Variable(var_type) => match var_type {
                    VariableType::Number => "<NUM>".to_string(),
                    VariableType::Timestamp => "<TIME>".to_string(),
                    VariableType::Date => "<DATE>".to_string(),
                    VariableType::Identifier => "<ID>".to_string(),
                    VariableType::Duration => "<DURATION>".to_string(),
                    VariableType::IpAddress => "<IP>".to_string(),
                    VariableType::Any => "<VAR>".to_string(),
                },
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if a sequence of tokens matches this template
    pub fn matches(&self, tokens: &[Token]) -> bool {
        if self.tokens.len() != tokens.len() {
            return false;
        }

        for (template_token, actual_token) in self.tokens.iter().zip(tokens.iter()) {
            match template_token {
                TemplateToken::Fixed(expected) => {
                    let actual = match actual_token {
                        Token::Literal(s) => s,
                        Token::Whitespace => " ",
                        Token::Punctuation(c) => {
                            // Convert char to string for comparison
                            if expected == &c.to_string() {
                                continue;
                            } else {
                                return false;
                            }
                        }
                        _ => return false,
                    };
                    if expected != actual {
                        return false;
                    }
                }
                TemplateToken::Variable(var_type) => {
                    let matches = match var_type {
                        VariableType::Number => matches!(actual_token, Token::Number(_)),
                        VariableType::Timestamp => matches!(actual_token, Token::Timestamp(_)),
                        VariableType::Date => matches!(actual_token, Token::Date(_)),
                        VariableType::Identifier => matches!(actual_token, Token::Identifier(_)),
                        VariableType::Duration => matches!(actual_token, Token::Duration(_)),
                        VariableType::IpAddress => matches!(actual_token, Token::IpAddress(_)),
                        VariableType::Any => actual_token.is_variable(),
                    };
                    if !matches {
                        return false;
                    }
                }
            }
        }

        true
    }
}

/// A group of entries that match the same template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateGroup {
    /// The template pattern for this group
    pub template: Template,
    /// Entry indices that match this template
    pub entry_indices: Vec<usize>,
    /// Sample variable values extracted from entries
    pub variable_samples: HashMap<usize, Vec<String>>,
}

impl TemplateGroup {
    /// Create a new template group
    pub fn new(template: Template) -> Self {
        TemplateGroup {
            template,
            entry_indices: Vec::new(),
            variable_samples: HashMap::new(),
        }
    }

    /// Add an entry to this group
    pub fn add_entry(&mut self, entry_index: usize, tokens: &[Token]) {
        self.entry_indices.push(entry_index);

        // Extract variable values
        let mut var_index = 0;
        for (template_token, actual_token) in self.template.tokens.iter().zip(tokens.iter()) {
            if matches!(template_token, TemplateToken::Variable(_)) {
                let value = match actual_token {
                    Token::Number(s)
                    | Token::Timestamp(s)
                    | Token::Date(s)
                    | Token::Identifier(s)
                    | Token::Duration(s)
                    | Token::IpAddress(s)
                    | Token::Literal(s) => s.clone(),
                    _ => continue,
                };

                self.variable_samples
                    .entry(var_index)
                    .or_insert_with(Vec::new)
                    .push(value);

                var_index += 1;
            }
        }
    }

    /// Get the count of entries in this group
    pub fn count(&self) -> usize {
        self.entry_indices.len()
    }

    /// Derive a human-readable name from the pattern
    /// Takes first few non-variable tokens, max 5 words
    pub fn derive_name(&self) -> String {
        let mut words = Vec::new();
        let max_words = 5;

        for token in &self.template.tokens {
            if words.len() >= max_words {
                break;
            }

            match token {
                TemplateToken::Fixed(s) => {
                    // Skip punctuation and whitespace
                    if !s.trim().is_empty() && !matches!(s.as_str(), "[" | "]" | "(" | ")" | "{" | "}" | "," | ";" | ":") {
                        words.push(s.clone());
                    }
                }
                _ => {} // Skip variables
            }
        }

        if words.is_empty() {
            "Pattern".to_string()
        } else {
            words.join(" ")
        }
    }
}

/// Template extractor for grouping log entries by pattern
pub struct TemplateExtractor {
    groups: Vec<TemplateGroup>,
}

impl TemplateExtractor {
    /// Create a new template extractor
    pub fn new() -> Self {
        TemplateExtractor { groups: Vec::new() }
    }

    /// Process a batch of entries and extract templates
    pub fn process(&mut self, entries: &[Entry]) {
        for (index, entry) in entries.iter().enumerate() {
            // For now, we only handle single-line entries
            if let Some(line) = entry.first_line() {
                let tokens = Tokenizer::tokenize(line);
                self.add_entry(index, &tokens);
            }
        }
    }

    /// Add an entry to the appropriate template group
    fn add_entry(&mut self, entry_index: usize, tokens: &[Token]) {
        // Try to find a matching template
        let mut matched = false;
        for group in &mut self.groups {
            if group.template.matches(tokens) {
                group.add_entry(entry_index, tokens);
                matched = true;
                break;
            }
        }

        // If no match, create a new template group
        if !matched {
            let template = Template::from_tokens(tokens);
            let mut group = TemplateGroup::new(template);
            group.add_entry(entry_index, tokens);
            self.groups.push(group);
        }
    }

    /// Get all template groups, sorted by count (descending)
    pub fn get_groups(&self) -> Vec<&TemplateGroup> {
        let mut groups: Vec<&TemplateGroup> = self.groups.iter().collect();
        groups.sort_by(|a, b| b.count().cmp(&a.count()));
        groups
    }

    /// Get the total number of unique templates
    pub fn template_count(&self) -> usize {
        self.groups.len()
    }
}

impl Default for TemplateExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_from_tokens() {
        let tokens = vec![
            Token::Punctuation('['),
            Token::Timestamp("2024-01-15".to_string()),
            Token::Punctuation(']'),
            Token::Whitespace,
            Token::Literal("INFO".to_string()),
        ];

        let template = Template::from_tokens(&tokens);

        assert_eq!(template.tokens.len(), 5);
        assert_eq!(template.tokens[0], TemplateToken::Fixed("[".to_string()));
        assert_eq!(template.tokens[1], TemplateToken::Variable(VariableType::Timestamp));
        assert_eq!(template.tokens[2], TemplateToken::Fixed("]".to_string()));
        assert_eq!(template.tokens[3], TemplateToken::Fixed(" ".to_string()));
        assert_eq!(template.tokens[4], TemplateToken::Fixed("INFO".to_string()));

        assert_eq!(template.pattern, "[<TIME>] INFO");
    }

    #[test]
    fn test_template_matches() {
        let tokens1 = vec![
            Token::Punctuation('['),
            Token::Timestamp("2024-01-15".to_string()),
            Token::Punctuation(']'),
            Token::Whitespace,
            Token::Literal("INFO".to_string()),
        ];

        let template = Template::from_tokens(&tokens1);

        // Should match a different timestamp
        let tokens2 = vec![
            Token::Punctuation('['),
            Token::Timestamp("2024-01-16".to_string()),
            Token::Punctuation(']'),
            Token::Whitespace,
            Token::Literal("INFO".to_string()),
        ];
        assert!(template.matches(&tokens2));

        // Should not match different literal
        let tokens3 = vec![
            Token::Punctuation('['),
            Token::Timestamp("2024-01-15".to_string()),
            Token::Punctuation(']'),
            Token::Whitespace,
            Token::Literal("ERROR".to_string()),
        ];
        assert!(!template.matches(&tokens3));

        // Should not match different structure
        let tokens4 = vec![
            Token::Punctuation('['),
            Token::Timestamp("2024-01-15".to_string()),
            Token::Punctuation(']'),
        ];
        assert!(!template.matches(&tokens4));
    }

    #[test]
    fn test_template_extractor_groups_similar_entries() {
        let entries = vec![
            Entry::from_line("[2024-01-15] INFO User login".to_string(), 1),
            Entry::from_line("[2024-01-15] INFO User logout".to_string(), 2),
            Entry::from_line("[2024-01-16] INFO User login".to_string(), 3),
            Entry::from_line("[2024-01-15] ERROR Connection failed".to_string(), 4),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        // Should have 3 templates: "INFO User login" (2x), "INFO User logout" (1x), "ERROR Connection failed" (1x)
        assert_eq!(extractor.template_count(), 3);

        let groups = extractor.get_groups();
        assert_eq!(groups.len(), 3);

        // First group should have 2 entries (INFO login messages)
        assert_eq!(groups[0].count(), 2);
        assert_eq!(groups[0].template.pattern, "[<DATE>] INFO User login");

        // Check the other groups exist
        assert_eq!(groups[1].count(), 1);
        assert_eq!(groups[2].count(), 1);
    }

    #[test]
    fn test_template_extractor_groups_identical_structure() {
        // Test that entries with identical structure (only variables differ) are grouped
        let entries = vec![
            Entry::from_line("Request 123 took 50 ms".to_string(), 1),
            Entry::from_line("Request 456 took 100 ms".to_string(), 2),
            Entry::from_line("Request 789 took 25 ms".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        // All should be in the same template group
        assert_eq!(extractor.template_count(), 1);

        let groups = extractor.get_groups();
        assert_eq!(groups[0].count(), 3);
        // Note: "123", "456", "789" are classified as identifiers (hex digits) not numbers
        assert_eq!(groups[0].template.pattern, "Request <NUM> took <NUM> ms");
    }

    #[test]
    fn test_template_extractor_with_numbers() {
        let entries = vec![
            Entry::from_line("Request took 42 ms".to_string(), 1),
            Entry::from_line("Request took 100 ms".to_string(), 2),
            Entry::from_line("Request took 15 ms".to_string(), 3),
        ];

        let mut extractor = TemplateExtractor::new();
        extractor.process(&entries);

        assert_eq!(extractor.template_count(), 1);

        let groups = extractor.get_groups();
        assert_eq!(groups[0].count(), 3);
        assert_eq!(groups[0].template.pattern, "Request took <NUM> ms");

        // Check variable samples
        let samples = &groups[0].variable_samples;
        assert_eq!(samples.len(), 1);
        let values = samples.get(&0).unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"42".to_string()));
        assert!(values.contains(&"100".to_string()));
        assert!(values.contains(&"15".to_string()));
    }

    #[test]
    fn test_template_extractor_deterministic() {
        let entries = vec![
            Entry::from_line("[2024-01-15] INFO Message 1".to_string(), 1),
            Entry::from_line("[2024-01-16] INFO Message 2".to_string(), 2),
        ];

        let mut extractor1 = TemplateExtractor::new();
        extractor1.process(&entries);

        let mut extractor2 = TemplateExtractor::new();
        extractor2.process(&entries);

        assert_eq!(extractor1.template_count(), extractor2.template_count());
        assert_eq!(
            extractor1.get_groups()[0].template.pattern,
            extractor2.get_groups()[0].template.pattern
        );
    }

    #[test]
    fn test_template_group_variable_extraction() {
        let tokens = vec![
            Token::Literal("User".to_string()),
            Token::Whitespace,
            Token::Identifier("abc123".to_string()),
            Token::Whitespace,
            Token::Literal("logged".to_string()),
            Token::Whitespace,
            Token::Literal("in".to_string()),
        ];

        let template = Template::from_tokens(&tokens);
        let mut group = TemplateGroup::new(template);

        // Add first entry
        let tokens1 = vec![
            Token::Literal("User".to_string()),
            Token::Whitespace,
            Token::Identifier("abc123".to_string()),
            Token::Whitespace,
            Token::Literal("logged".to_string()),
            Token::Whitespace,
            Token::Literal("in".to_string()),
        ];
        group.add_entry(0, &tokens1);

        // Add second entry with different ID
        let tokens2 = vec![
            Token::Literal("User".to_string()),
            Token::Whitespace,
            Token::Identifier("xyz789".to_string()),
            Token::Whitespace,
            Token::Literal("logged".to_string()),
            Token::Whitespace,
            Token::Literal("in".to_string()),
        ];
        group.add_entry(1, &tokens2);

        assert_eq!(group.count(), 2);
        assert_eq!(group.entry_indices, vec![0, 1]);

        // Should have captured both IDs as variable samples
        let samples = group.variable_samples.get(&0).unwrap();
        assert_eq!(samples.len(), 2);
        assert!(samples.contains(&"abc123".to_string()));
        assert!(samples.contains(&"xyz789".to_string()));
    }
}
