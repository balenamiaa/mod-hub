//! Advanced pattern matching for binary signatures with multiple algorithms.
//!
//! This module provides fast, robust pattern matching capabilities optimized for
//! reverse engineering and binary analysis. It includes multiple search algorithms
//! with different performance characteristics for various use cases.

use crate::errors::Error;
use std::collections::HashMap;

/// Represents a pattern that can contain wildcards and exact byte matches.
#[derive(Debug, Clone)]
pub struct Pattern {
    bytes: Vec<Option<u8>>,
    mask: String,
}

impl Pattern {
    /// Creates a new pattern from a string representation.
    /// Format: "48 8B ?? 74 ??" where ?? represents wildcards.
    pub fn new(pattern_str: &str) -> Result<Self, Error> {
        let parts: Vec<&str> = pattern_str.split_whitespace().collect();
        let mut bytes = Vec::with_capacity(parts.len());
        let mut mask = String::with_capacity(parts.len());

        for part in parts {
            if part == "?" || part == "??" {
                bytes.push(None);
                mask.push('?');
            } else if part.len() == 2 {
                let byte = u8::from_str_radix(part, 16)
                    .map_err(|_| Error::InvalidHex(part.to_string()))?;
                bytes.push(Some(byte));
                mask.push('x');
            } else {
                return Err(Error::InvalidPatternFormat(part.to_string()));
            }
        }

        if bytes.is_empty() {
            return Err(Error::EmptyPattern);
        }

        Ok(Pattern { bytes, mask })
    }

    /// Creates a pattern from raw bytes and a mask string.
    pub fn from_bytes_and_mask(bytes: &[u8], mask: &str) -> Result<Self, Error> {
        if bytes.len() != mask.len() {
            return Err(Error::MaskLengthMismatch);
        }

        let mut pattern_bytes = Vec::with_capacity(bytes.len());
        
        for (i, mask_char) in mask.chars().enumerate() {
            match mask_char {
                'x' | 'X' => pattern_bytes.push(Some(bytes[i])),
                '?' => pattern_bytes.push(None),
                _ => return Err(Error::InvalidMaskChar(mask_char)),
            }
        }

        Ok(Pattern {
            bytes: pattern_bytes,
            mask: mask.to_string(),
        })
    }

    /// Returns the length of the pattern.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns true if the pattern is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the mask string.
    pub fn mask(&self) -> &str {
        &self.mask
    }

    /// Returns the bytes vector.
    pub fn bytes(&self) -> &[Option<u8>] {
        &self.bytes
    }

    /// Checks if the pattern matches at a specific offset in the data.
    pub fn matches_at(&self, data: &[u8], offset: usize) -> bool {
        if offset + self.len() > data.len() {
            return false;
        }

        for (i, pattern_byte) in self.bytes.iter().enumerate() {
            if let Some(expected) = pattern_byte {
                if data[offset + i] != *expected {
                    return false;
                }
            }
        }
        true
    }
}

/// Result of a pattern search operation.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub offset: usize,
    pub size: usize,
}

/// Trait for different pattern matching algorithms.
pub trait PatternMatcher {
    fn find_all(&self, pattern: &Pattern, data: &[u8]) -> Vec<PatternMatch>;
    fn find_first(&self, pattern: &Pattern, data: &[u8]) -> Option<PatternMatch>;
}

/// Naive pattern matching - simple but slow O(nm) algorithm.
/// Best for: Small patterns or small data sets.
pub struct NaiveMatcher;

impl PatternMatcher for NaiveMatcher {
    fn find_all(&self, pattern: &Pattern, data: &[u8]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        
        if pattern.is_empty() || data.len() < pattern.len() {
            return matches;
        }

        for i in 0..=data.len() - pattern.len() {
            if pattern.matches_at(data, i) {
                matches.push(PatternMatch {
                    offset: i,
                    size: pattern.len(),
                });
            }
        }
        
        matches
    }

    fn find_first(&self, pattern: &Pattern, data: &[u8]) -> Option<PatternMatch> {
        if pattern.is_empty() || data.len() < pattern.len() {
            return None;
        }

        for i in 0..=data.len() - pattern.len() {
            if pattern.matches_at(data, i) {
                return Some(PatternMatch {
                    offset: i,
                    size: pattern.len(),
                });
            }
        }
        
        None
    }
}

/// Boyer-Moore pattern matching with bad character heuristic.
/// Best for: Large data sets with long patterns, especially with wildcards at the end.
pub struct BoyerMooreMatcher;

impl BoyerMooreMatcher {
    fn build_bad_char_table(pattern: &Pattern) -> HashMap<u8, usize> {
        let mut table = HashMap::new();
        
        for (i, byte_opt) in pattern.bytes().iter().enumerate() {
            if let Some(byte) = byte_opt {
                table.insert(*byte, i);
            }
        }
        
        table
    }
}

impl PatternMatcher for BoyerMooreMatcher {
    fn find_all(&self, pattern: &Pattern, data: &[u8]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        
        if pattern.is_empty() || data.len() < pattern.len() {
            return matches;
        }

        let bad_char_table = Self::build_bad_char_table(pattern);
        let pattern_len = pattern.len();
        let mut i = 0;

        while i <= data.len() - pattern_len {
            let mut j = pattern_len;
            
            // Match from right to left
            while j > 0 && pattern.matches_at(data, i) {
                if let Some(pattern_byte) = pattern.bytes()[j - 1] {
                    if data[i + j - 1] != pattern_byte {
                        break;
                    }
                }
                j -= 1;
            }

            if j == 0 {
                matches.push(PatternMatch {
                    offset: i,
                    size: pattern_len,
                });
                i += 1;
            } else {
                // Apply bad character heuristic
                let bad_char = data[i + j - 1];
                if let Some(&pos) = bad_char_table.get(&bad_char) {
                    i += std::cmp::max(1, j - pos - 1);
                } else {
                    i += j;
                }
            }
        }
        
        matches
    }

    fn find_first(&self, pattern: &Pattern, data: &[u8]) -> Option<PatternMatch> {
        if pattern.is_empty() || data.len() < pattern.len() {
            return None;
        }

        let bad_char_table = Self::build_bad_char_table(pattern);
        let pattern_len = pattern.len();
        let mut i = 0;

        while i <= data.len() - pattern_len {
            if pattern.matches_at(data, i) {
                return Some(PatternMatch {
                    offset: i,
                    size: pattern_len,
                });
            }

            // Apply bad character heuristic for next position
            if i + pattern_len < data.len() {
                let bad_char = data[i + pattern_len - 1];
                if let Some(&pos) = bad_char_table.get(&bad_char) {
                    i += std::cmp::max(1, pattern_len - pos - 1);
                } else {
                    i += pattern_len;
                }
            } else {
                i += 1;
            }
        }
        
        None
    }
}

/// KMP (Knuth-Morris-Pratt) pattern matching with failure function.
/// Best for: Patterns with repeated subsequences, consistent O(n+m) performance.
pub struct KmpMatcher;

impl KmpMatcher {
    fn build_failure_function(pattern: &Pattern) -> Vec<usize> {
        let mut failure = vec![0; pattern.len()];
        let mut j = 0;

        for i in 1..pattern.len() {
            while j > 0 && !Self::pattern_chars_equal(pattern, i, j) {
                j = failure[j - 1];
            }
            
            if Self::pattern_chars_equal(pattern, i, j) {
                j += 1;
            }
            
            failure[i] = j;
        }
        
        failure
    }

    fn pattern_chars_equal(pattern: &Pattern, i: usize, j: usize) -> bool {
        match (pattern.bytes()[i], pattern.bytes()[j]) {
            (Some(a), Some(b)) => a == b,
            (None, _) | (_, None) => true,  // Wildcards match anything
        }
    }

    fn pattern_matches_data(pattern: &Pattern, data: &[u8], pattern_idx: usize, data_idx: usize) -> bool {
        if let Some(pattern_byte) = pattern.bytes()[pattern_idx] {
            data[data_idx] == pattern_byte
        } else {
            true  // Wildcard matches anything
        }
    }
}

impl PatternMatcher for KmpMatcher {
    fn find_all(&self, pattern: &Pattern, data: &[u8]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        
        if pattern.is_empty() || data.len() < pattern.len() {
            return matches;
        }

        let failure = Self::build_failure_function(pattern);
        let mut j = 0;

        for i in 0..data.len() {
            while j > 0 && !Self::pattern_matches_data(pattern, data, j, i) {
                j = failure[j - 1];
            }

            if Self::pattern_matches_data(pattern, data, j, i) {
                j += 1;
            }

            if j == pattern.len() {
                matches.push(PatternMatch {
                    offset: i - pattern.len() + 1,
                    size: pattern.len(),
                });
                j = failure[j - 1];
            }
        }
        
        matches
    }

    fn find_first(&self, pattern: &Pattern, data: &[u8]) -> Option<PatternMatch> {
        if pattern.is_empty() || data.len() < pattern.len() {
            return None;
        }

        let failure = Self::build_failure_function(pattern);
        let mut j = 0;

        for i in 0..data.len() {
            while j > 0 && !Self::pattern_matches_data(pattern, data, j, i) {
                j = failure[j - 1];
            }

            if Self::pattern_matches_data(pattern, data, j, i) {
                j += 1;
            }

            if j == pattern.len() {
                return Some(PatternMatch {
                    offset: i - pattern.len() + 1,
                    size: pattern.len(),
                });
            }
        }
        
        None
    }
}

/// Hybrid matcher that automatically selects the best algorithm based on pattern characteristics.
pub struct HybridMatcher;

impl HybridMatcher {
    fn select_matcher(pattern: &Pattern) -> Box<dyn PatternMatcher> {
        // Use Boyer-Moore for longer patterns with few wildcards
        let wildcard_ratio = pattern.bytes().iter()
            .map(|b| if b.is_none() { 1.0 } else { 0.0 })
            .sum::<f32>() / pattern.len() as f32;

        if pattern.len() >= 8 && wildcard_ratio < 0.3 {
            Box::new(BoyerMooreMatcher)
        } 
        // Use KMP for patterns with potential repetitions
        else if pattern.len() >= 4 {
            Box::new(KmpMatcher)
        }
        // Use naive for short patterns
        else {
            Box::new(NaiveMatcher)
        }
    }
}

impl PatternMatcher for HybridMatcher {
    fn find_all(&self, pattern: &Pattern, data: &[u8]) -> Vec<PatternMatch> {
        let matcher = Self::select_matcher(pattern);
        matcher.find_all(pattern, data)
    }

    fn find_first(&self, pattern: &Pattern, data: &[u8]) -> Option<PatternMatch> {
        let matcher = Self::select_matcher(pattern);
        matcher.find_first(pattern, data)
    }
}

/// High-level pattern scanner for convenient usage.
pub struct PatternScanner {
    matcher: Box<dyn PatternMatcher>,
}

impl Default for PatternScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternScanner {
    /// Creates a new scanner with the hybrid matcher.
    pub fn new() -> Self {
        Self {
            matcher: Box::new(HybridMatcher),
        }
    }

    /// Creates a scanner with a specific matcher.
    pub fn with_matcher(matcher: Box<dyn PatternMatcher>) -> Self {
        Self { matcher }
    }

    /// Scans for a pattern and returns all matches.
    pub fn scan(&self, pattern_str: &str, data: &[u8]) -> Result<Vec<PatternMatch>, Error> {
        let pattern = Pattern::new(pattern_str)?;
        Ok(self.matcher.find_all(&pattern, data))
    }

    /// Scans for a pattern and returns the first match.
    pub fn scan_first(&self, pattern_str: &str, data: &[u8]) -> Result<Option<PatternMatch>, Error> {
        let pattern = Pattern::new(pattern_str)?;
        Ok(self.matcher.find_first(&pattern, data))
    }

    /// Scans with a pre-compiled pattern.
    pub fn scan_pattern(&self, pattern: &Pattern, data: &[u8]) -> Vec<PatternMatch> {
        self.matcher.find_all(pattern, data)
    }

    /// Scans with a pre-compiled pattern and returns the first match.
    pub fn scan_pattern_first(&self, pattern: &Pattern, data: &[u8]) -> Option<PatternMatch> {
        self.matcher.find_first(pattern, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new("48 8B ?? 74 ??").unwrap();
        assert_eq!(pattern.len(), 5);
        assert_eq!(pattern.mask(), "xx?x?");
        
        let bytes = pattern.bytes();
        assert_eq!(bytes[0], Some(0x48));
        assert_eq!(bytes[1], Some(0x8B));
        assert_eq!(bytes[2], None);
        assert_eq!(bytes[3], Some(0x74));
        assert_eq!(bytes[4], None);
    }

    #[test]
    fn test_pattern_matching() {
        let data = [0x48, 0x8B, 0x05, 0x74, 0x12, 0x90, 0x48, 0x8B, 0xFF, 0x74, 0x34];
        let pattern = Pattern::new("48 8B ?? 74").unwrap();
        
        let scanner = PatternScanner::new();
        let matches = scanner.scan_pattern(&pattern, &data);
        
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].offset, 0);
        assert_eq!(matches[1].offset, 6);
    }

    #[test]
    fn test_different_matchers() {
        let data = [0x12, 0x34, 0x56, 0x78, 0x9A, 0x34, 0x56, 0xBC];
        let pattern = Pattern::new("34 56").unwrap();

        let matchers: Vec<Box<dyn PatternMatcher>> = vec![
            Box::new(NaiveMatcher),
            Box::new(BoyerMooreMatcher),
            Box::new(KmpMatcher),
        ];

        for matcher in matchers {
            let matches = matcher.find_all(&pattern, &data);
            assert_eq!(matches.len(), 2);
            assert_eq!(matches[0].offset, 1);
            assert_eq!(matches[1].offset, 5);
        }
    }
}