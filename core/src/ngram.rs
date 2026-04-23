//! N-gram based outlier detection
//!
//! This algorithm builds a frequency table of word-level n-grams,
//! scores each entry by rarity, and highlights entries with rare patterns.

use crate::entry::Entry;
use crate::metadata::{AlgorithmMetadata, InputType, ParamDefault, ParamRange, ParamType, Parameter};
use std::collections::HashMap;

/// Score statistics for reporting
#[derive(Debug, Clone, Copy)]
pub struct ScoreStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
}

/// N-gram outlier detector (word-based)
pub struct NgramOutlierDetector {
    /// N-gram size (e.g., 2 for bigrams, 3 for trigrams)
    n: usize,
    /// Frequency table: n-gram -> count
    ngram_freq: HashMap<String, usize>,
    /// Entry scores sorted by score ascending (for threshold detection and get_outliers)
    entry_scores: Vec<(usize, f64)>, // (entry_index, score)
    /// Index → score map for O(1) lookup by entry index
    score_by_index: HashMap<usize, f64>,
    /// Total n-grams seen
    total_ngrams: usize,
    /// Outlier threshold (entries with score below this are outliers)
    threshold: Option<f64>,
    /// Actual threshold used after auto-detection
    effective_threshold: f64,
}

impl NgramOutlierDetector {
    /// Metadata describing this algorithm
    pub const METADATA: AlgorithmMetadata = AlgorithmMetadata {
        name: "ngram",
        aliases: &["n-gram", "ngrams"],
        description: "Identifies entries with rare word combinations using n-gram frequency analysis",
        best_for: "Finding unusual entries in mostly uniform logs",
        parameters: &[
            Parameter {
                name: "ngram_size",
                type_info: ParamType::USize,
                default: ParamDefault::USize(2),
                range: Some(ParamRange::USize { min: 1, max: 10 }),
                description: "N-gram size (word-level). 2 = bigrams, 3 = trigrams",
                special_values: &[],
            },
            Parameter {
                name: "outlier_threshold",
                type_info: ParamType::Float,
                default: ParamDefault::Float(0.0),
                range: Some(ParamRange::Float { min: 0.0, max: 1.0 }),
                description: "Outlier threshold for rarity score. 0.0 = auto-detect bottom ~5%",
                special_values: &[(0.0, "auto-detect")],
            },
        ],
        input_types: &[InputType::Text],
    };

    /// Create a new word-based n-gram detector with specified n-gram size and threshold
    /// Use threshold of 0.0 or negative to enable auto-detection
    pub fn new(n: usize, threshold: f64) -> Self {
        let threshold_opt = if threshold <= 0.0 {
            None
        } else {
            Some(threshold)
        };

        NgramOutlierDetector {
            n,
            ngram_freq: HashMap::new(),
            entry_scores: Vec::new(),
            score_by_index: HashMap::new(),
            total_ngrams: 0,
            threshold: threshold_opt,
            effective_threshold: threshold.max(0.0),
        }
    }

    /// Process entries to build n-gram frequency table and score entries
    pub fn process(&mut self, entries: &[Entry]) {
        // First pass: build frequency table
        for entry in entries {
            let content = entry.as_single_string();
            let ngrams = self.extract_ngrams(&content);

            for ngram in ngrams {
                *self.ngram_freq.entry(ngram).or_insert(0) += 1;
                self.total_ngrams += 1;
            }
        }

        // Second pass: score each entry by rarity
        for (idx, entry) in entries.iter().enumerate() {
            let content = entry.as_single_string();
            let score = self.calculate_rarity_score(&content);
            self.entry_scores.push((idx, score));
            self.score_by_index.insert(idx, score);
        }

        // Sort by score (ascending - lowest scores are most unusual)
        self.entry_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Auto-detect threshold if not provided
        if self.threshold.is_none() {
            self.effective_threshold = self.calculate_auto_threshold(entries.len());
        } else {
            self.effective_threshold = self.threshold.unwrap();
        }
    }

    /// Calculate automatic threshold to flag bottom 5% as outliers
    fn calculate_auto_threshold(&self, total_entries: usize) -> f64 {
        if self.entry_scores.is_empty() {
            return 0.0;
        }

        // Flag bottom 5% as outliers (minimum 1, maximum 20% of entries)
        let outlier_percentage = 0.05;
        let min_outliers = 1;
        let max_outlier_percentage = 0.20;

        let target_outliers = (total_entries as f64 * outlier_percentage)
            .max(min_outliers as f64)
            .min(total_entries as f64 * max_outlier_percentage)
            .ceil() as usize;

        // Get the score at the target percentile
        let target_outliers = target_outliers.min(self.entry_scores.len());
        if target_outliers == 0 {
            return 0.0;
        }

        // Use score just above the Nth lowest entry as threshold
        let threshold_score = self.entry_scores[target_outliers.saturating_sub(1)].1;

        // Add small epsilon to ensure we include exactly the target number
        threshold_score * 1.0001
    }

    /// Extract word-level n-grams from a string
    fn extract_ngrams(&self, text: &str) -> Vec<String> {
        // Tokenize into words (split on whitespace and common punctuation)
        let words: Vec<&str> = text
            .split(|c: char| c.is_whitespace() || "[](){}:,;\"'".contains(c))
            .filter(|w| !w.is_empty())
            .collect();

        if words.len() < self.n {
            // If not enough words, return the whole text as a single token
            return vec![words.join(" ")];
        }

        // Create n-grams from consecutive words
        words
            .windows(self.n)
            .map(|window| window.join(" "))
            .collect()
    }

    /// Calculate rarity score for an entry (sum of inverse frequencies)
    /// Lower score = more unusual
    fn calculate_rarity_score(&self, text: &str) -> f64 {
        let ngrams = self.extract_ngrams(text);

        if ngrams.is_empty() {
            return 1.0;
        }

        // Score = average frequency of n-grams (normalized)
        let total_freq: f64 = ngrams
            .iter()
            .map(|ng| {
                let count = self.ngram_freq.get(ng).copied().unwrap_or(1);
                count as f64 / self.total_ngrams as f64
            })
            .sum();

        total_freq / ngrams.len() as f64
    }

    /// Get outlier indices (entries with score below threshold)
    pub fn get_outliers(&self) -> Vec<usize> {
        self.entry_scores
            .iter()
            .filter(|(_, score)| *score < self.effective_threshold)
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// Get the effective threshold being used (after auto-detection if applicable)
    pub fn get_effective_threshold(&self) -> f64 {
        self.effective_threshold
    }

    /// Check if threshold was auto-detected
    pub fn is_auto_threshold(&self) -> bool {
        self.threshold.is_none()
    }

    /// Get score statistics for reporting
    pub fn get_score_stats(&self) -> ScoreStats {
        if self.entry_scores.is_empty() {
            return ScoreStats {
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                median: 0.0,
            };
        }

        let scores: Vec<f64> = self.entry_scores.iter().map(|(_, s)| *s).collect();
        let min = scores.iter().copied().fold(f64::INFINITY, f64::min);
        let max = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let median = scores[scores.len() / 2];

        ScoreStats {
            min,
            max,
            mean,
            median,
        }
    }

    /// Get entry score by index
    pub fn get_score(&self, entry_idx: usize) -> Option<f64> {
        self.score_by_index.get(&entry_idx).copied()
    }

    /// Get top N most common n-grams
    pub fn get_top_ngrams(&self, n: usize) -> Vec<(String, usize)> {
        let mut ngrams: Vec<(String, usize)> = self
            .ngram_freq
            .iter()
            .map(|(ng, count)| (ng.clone(), *count))
            .collect();

        ngrams.sort_by(|a, b| b.1.cmp(&a.1));
        ngrams.truncate(n);
        ngrams
    }

    /// Get the percentage of normal (non-outlier) entries
    pub fn get_normal_percentage(&self, total_entries: usize) -> f64 {
        let outlier_count = self.get_outliers().len();
        let normal_count = total_entries.saturating_sub(outlier_count);
        (normal_count as f64 / total_entries as f64) * 100.0
    }

    /// Get count of normal entries
    pub fn get_normal_count(&self, total_entries: usize) -> usize {
        let outlier_count = self.get_outliers().len();
        total_entries.saturating_sub(outlier_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ngram_extraction() {
        let detector = NgramOutlierDetector::new(2, 0.01);
        let ngrams = detector.extract_ngrams("ERROR connection failed timeout");

        // Should create bigrams of words
        assert_eq!(ngrams.len(), 3);
        assert_eq!(ngrams[0], "ERROR connection");
        assert_eq!(ngrams[1], "connection failed");
        assert_eq!(ngrams[2], "failed timeout");
    }

    #[test]
    fn test_ngram_extraction_short_string() {
        let detector = NgramOutlierDetector::new(3, 0.01);
        let ngrams = detector.extract_ngrams("one two");

        // String with fewer words than n returns combined words
        assert_eq!(ngrams.len(), 1);
        assert_eq!(ngrams[0], "one two");
    }

    #[test]
    fn test_frequency_counting() {
        let mut detector = NgramOutlierDetector::new(2, 0.01);
        let entries = vec![
            Entry::from_line("ERROR Connection failed".to_string(), 1),
            Entry::from_line("ERROR Timeout occurred".to_string(), 2),
            Entry::from_line("INFO All good".to_string(), 3),
        ];

        detector.process(&entries);

        // "ERROR Connection" and "ERROR Timeout" should each appear once
        assert!(detector.ngram_freq.get("ERROR Connection").is_some());
        assert!(detector.ngram_freq.get("ERROR Timeout").is_some());
    }

    #[test]
    fn test_outlier_detection() {
        let mut detector = NgramOutlierDetector::new(2, 0.15);
        let entries = vec![
            Entry::from_line("INFO User login".to_string(), 1),
            Entry::from_line("INFO User logout".to_string(), 2),
            Entry::from_line("INFO User login".to_string(), 3),
            Entry::from_line("INFO User logout".to_string(), 4),
            Entry::from_line("ERROR Database explosion".to_string(), 5), // Outlier
        ];

        detector.process(&entries);

        let outliers = detector.get_outliers();
        assert!(!outliers.is_empty());
        // The ERROR line should be an outlier (has rare word combinations)
        assert!(outliers.contains(&4));
    }

    #[test]
    fn test_scoring() {
        let mut detector = NgramOutlierDetector::new(2, 0.05);
        let entries = vec![
            Entry::from_line("common pattern repeating".to_string(), 1),
            Entry::from_line("common pattern repeating".to_string(), 2),
            Entry::from_line("rare unique unusual".to_string(), 3),
        ];

        detector.process(&entries);

        // The rare entry should have a lower score than common ones
        let score_common = detector.get_score(0).unwrap();
        let score_rare = detector.get_score(2).unwrap();
        assert!(score_rare < score_common);
    }

    #[test]
    fn test_top_ngrams() {
        let mut detector = NgramOutlierDetector::new(2, 0.01);
        let entries = vec![
            Entry::from_line("login success user".to_string(), 1),
            Entry::from_line("login success admin".to_string(), 2),
            Entry::from_line("logout complete".to_string(), 3),
        ];

        detector.process(&entries);

        let top = detector.get_top_ngrams(3);
        assert!(!top.is_empty());
        // "login success" should be most common (appears twice)
        assert_eq!(top[0].0, "login success");
    }

    #[test]
    fn test_normal_percentage() {
        let mut detector = NgramOutlierDetector::new(2, 0.1);
        let entries = vec![
            Entry::from_line("normal pattern".to_string(), 1),
            Entry::from_line("normal pattern".to_string(), 2),
            Entry::from_line("normal pattern".to_string(), 3),
            Entry::from_line("completely different rare".to_string(), 4), // Outlier
        ];

        detector.process(&entries);

        let normal_pct = detector.get_normal_percentage(entries.len());
        assert!(normal_pct > 50.0);
    }

    #[test]
    fn test_deterministic() {
        let entries = vec![
            Entry::from_line("Test A".to_string(), 1),
            Entry::from_line("Test B".to_string(), 2),
        ];

        let mut detector1 = NgramOutlierDetector::new(3, 0.01);
        detector1.process(&entries);

        let mut detector2 = NgramOutlierDetector::new(3, 0.01);
        detector2.process(&entries);

        // Same input should produce same outliers
        assert_eq!(detector1.get_outliers(), detector2.get_outliers());
    }
}
