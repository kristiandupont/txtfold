//! Edit distance clustering algorithm for grouping similar entries

use crate::entry::Entry;
use serde::{Deserialize, Serialize};

/// A cluster of similar entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    /// Representative entry (exemplar) for this cluster
    pub exemplar: String,
    /// Indices of entries in this cluster
    pub entry_indices: Vec<usize>,
    /// All line numbers covered by this cluster
    pub line_numbers: Vec<usize>,
}

/// Edit distance clustering analyzer
pub struct EditDistanceClusterer {
    /// Distance threshold for grouping (0.0 = identical, 1.0 = completely different)
    pub threshold: f64,
    /// Clusters discovered
    pub clusters: Vec<Cluster>,
}

impl EditDistanceClusterer {
    /// Create a new clusterer with the given threshold
    pub fn new(threshold: f64) -> Self {
        EditDistanceClusterer {
            threshold,
            clusters: Vec::new(),
        }
    }

    /// Process entries and create clusters
    pub fn process(&mut self, entries: &[Entry]) {
        if entries.is_empty() {
            return;
        }

        let mut assigned: Vec<bool> = vec![false; entries.len()];

        for i in 0..entries.len() {
            if assigned[i] {
                continue;
            }

            // Start a new cluster with this entry as exemplar
            let exemplar = entries[i].as_single_string();
            let mut cluster_indices = vec![i];
            let mut line_numbers = entries[i]
                .metadata
                .as_ref()
                .map(|m| m.line_numbers.clone())
                .unwrap_or_default();

            assigned[i] = true;

            // Find all similar entries
            for j in (i + 1)..entries.len() {
                if assigned[j] {
                    continue;
                }

                let other = entries[j].as_single_string();
                let distance = normalized_edit_distance(&exemplar, &other);

                if distance <= self.threshold {
                    cluster_indices.push(j);
                    if let Some(metadata) = &entries[j].metadata {
                        line_numbers.extend(metadata.line_numbers.iter().copied());
                    }
                    assigned[j] = true;
                }
            }

            self.clusters.push(Cluster {
                exemplar,
                entry_indices: cluster_indices,
                line_numbers,
            });
        }

        // Sort clusters by size (largest first)
        self.clusters.sort_by(|a, b| b.entry_indices.len().cmp(&a.entry_indices.len()));
    }

    /// Get clusters grouped by size
    pub fn get_clusters(&self) -> &[Cluster] {
        &self.clusters
    }
}

/// Calculate normalized edit distance between two strings (0.0 = identical, 1.0 = completely different)
fn normalized_edit_distance(s1: &str, s2: &str) -> f64 {
    let distance = levenshtein_distance(s1, s2);
    let max_len = s1.len().max(s2.len());

    if max_len == 0 {
        return 0.0;
    }

    distance as f64 / max_len as f64
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    // Create distance matrix
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    // Fill matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };

            matrix[i][j] = (matrix[i - 1][j] + 1)           // deletion
                .min(matrix[i][j - 1] + 1)                  // insertion
                .min(matrix[i - 1][j - 1] + cost);          // substitution
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_one_substitution() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    }

    #[test]
    fn test_levenshtein_insertion() {
        assert_eq!(levenshtein_distance("hello", "helllo"), 1);
    }

    #[test]
    fn test_levenshtein_deletion() {
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
    }

    #[test]
    fn test_levenshtein_completely_different() {
        assert_eq!(levenshtein_distance("abc", "xyz"), 3);
    }

    #[test]
    fn test_normalized_distance() {
        assert_eq!(normalized_edit_distance("hello", "hello"), 0.0);
        assert_eq!(normalized_edit_distance("hello", "hallo"), 0.2); // 1/5
        assert_eq!(normalized_edit_distance("abc", "xyz"), 1.0); // 3/3
    }

    #[test]
    fn test_clustering_identical_entries() {
        let entries = vec![
            Entry::from_line("Error: connection timeout".to_string(), 1),
            Entry::from_line("Error: connection timeout".to_string(), 2),
            Entry::from_line("Error: connection timeout".to_string(), 3),
        ];

        let mut clusterer = EditDistanceClusterer::new(0.0);
        clusterer.process(&entries);

        assert_eq!(clusterer.clusters.len(), 1);
        assert_eq!(clusterer.clusters[0].entry_indices.len(), 3);
    }

    #[test]
    fn test_clustering_similar_entries() {
        let entries = vec![
            Entry::from_line("Error: connection timeout".to_string(), 1),
            Entry::from_line("Error: connection timeout".to_string(), 2),
            Entry::from_line("Error: database timeout".to_string(), 3),
            Entry::from_line("Info: user logged in".to_string(), 4),
        ];

        // "Error: connection timeout" vs "Error: database timeout"
        // Diff: "connection" (10 chars) vs "database" (8 chars)
        // Distance: ~10 edits in ~26 char string = ~0.38, too high for 0.3 threshold
        let mut clusterer = EditDistanceClusterer::new(0.3);
        clusterer.process(&entries);

        // Should have 3 clusters: identical errors (2), different error (1), info (1)
        assert_eq!(clusterer.clusters.len(), 3);
        assert_eq!(clusterer.clusters[0].entry_indices.len(), 2); // identical errors
        assert_eq!(clusterer.clusters[1].entry_indices.len(), 1); // database error
        assert_eq!(clusterer.clusters[2].entry_indices.len(), 1); // info
    }

    #[test]
    fn test_clustering_with_numbers() {
        let entries = vec![
            Entry::from_line("Thread http-nio-8080-exec-13 started".to_string(), 1),
            Entry::from_line("Thread http-nio-8080-exec-7 started".to_string(), 2),
            Entry::from_line("Thread http-nio-8080-exec-42 started".to_string(), 3),
        ];

        // These should cluster together with threshold 0.1 (only 1-2 char difference in ~35 char strings)
        let mut clusterer = EditDistanceClusterer::new(0.1);
        clusterer.process(&entries);

        assert_eq!(clusterer.clusters.len(), 1);
        assert_eq!(clusterer.clusters[0].entry_indices.len(), 3);
    }

    #[test]
    fn test_clustering_sorts_by_size() {
        let entries = vec![
            Entry::from_line("A".to_string(), 1),
            Entry::from_line("B".to_string(), 2),
            Entry::from_line("B".to_string(), 3),
            Entry::from_line("B".to_string(), 4),
        ];

        let mut clusterer = EditDistanceClusterer::new(0.0);
        clusterer.process(&entries);

        // Should have 2 clusters, larger one first
        assert_eq!(clusterer.clusters.len(), 2);
        assert_eq!(clusterer.clusters[0].entry_indices.len(), 3); // B cluster
        assert_eq!(clusterer.clusters[1].entry_indices.len(), 1); // A cluster
    }
}
