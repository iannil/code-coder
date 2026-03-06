//! Edit tool - high-performance diff and patch operations
//!
//! This module provides file editing with:
//! - Exact string replacement
//! - Diff-based patching using the similar crate
//! - Replace-all functionality
//! - Edit validation
//! - Fuzzy matching with multiple replacer strategies

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

// Similarity thresholds for block anchor fallback matching
const SINGLE_CANDIDATE_SIMILARITY_THRESHOLD: f64 = 0.0;
const MULTIPLE_CANDIDATES_SIMILARITY_THRESHOLD: f64 = 0.3;

/// Options for editing files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    /// The text to find and replace
    pub old_string: String,

    /// The replacement text
    pub new_string: String,

    /// Whether to replace all occurrences
    #[serde(default)]
    pub replace_all: bool,
}

/// Result of an edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    /// Whether the edit was successful
    pub success: bool,

    /// Number of replacements made
    pub replacements: usize,

    /// The unified diff showing changes
    pub diff: String,

    /// Error message if failed
    pub error: Option<String>,

    /// Original content hash (for conflict detection)
    pub original_hash: Option<String>,
}

impl EditResult {
    /// Create a successful result
    pub fn ok(replacements: usize, diff: String) -> Self {
        Self {
            success: true,
            replacements,
            diff,
            error: None,
            original_hash: None,
        }
    }

    /// Create a failed result
    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            replacements: 0,
            diff: String::new(),
            error: Some(error.into()),
            original_hash: None,
        }
    }
}

/// File editor with diff support
pub struct Editor;

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    /// Create a new Editor
    pub fn new() -> Self {
        Self
    }

    /// Edit a file by replacing old_string with new_string
    pub fn edit(&self, path: &Path, operation: &EditOperation) -> Result<EditResult> {
        // Read the file
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Validate that old_string exists
        if !content.contains(&operation.old_string) {
            return Ok(EditResult::err(format!(
                "old_string not found in file: {}",
                path.display()
            )));
        }

        // Check if old_string is unique (unless replace_all is true)
        if !operation.replace_all {
            let count = content.matches(&operation.old_string).count();
            if count > 1 {
                return Ok(EditResult::err(format!(
                    "old_string is not unique in file (found {} occurrences). Use replace_all=true or provide more context.",
                    count
                )));
            }
        }

        // Perform the replacement
        let new_content = if operation.replace_all {
            content.replace(&operation.old_string, &operation.new_string)
        } else {
            content.replacen(&operation.old_string, &operation.new_string, 1)
        };

        let replacements = if operation.replace_all {
            content.matches(&operation.old_string).count()
        } else {
            1
        };

        // Generate diff
        let diff = self.generate_diff(&content, &new_content, path);

        // Write the new content atomically
        self.atomic_write(path, &new_content)?;

        Ok(EditResult::ok(replacements, diff))
    }

    /// Edit a file with multiple operations
    pub fn edit_multiple(&self, path: &Path, operations: &[EditOperation]) -> Result<EditResult> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let mut new_content = content.clone();
        let mut total_replacements = 0;

        for operation in operations {
            if !new_content.contains(&operation.old_string) {
                return Ok(EditResult::err(format!(
                    "old_string not found in file: '{}'",
                    &operation.old_string[..operation.old_string.len().min(50)]
                )));
            }

            if operation.replace_all {
                let count = new_content.matches(&operation.old_string).count();
                new_content = new_content.replace(&operation.old_string, &operation.new_string);
                total_replacements += count;
            } else {
                let count = new_content.matches(&operation.old_string).count();
                if count > 1 {
                    return Ok(EditResult::err(format!(
                        "old_string is not unique (found {} occurrences)",
                        count
                    )));
                }
                new_content = new_content.replacen(&operation.old_string, &operation.new_string, 1);
                total_replacements += 1;
            }
        }

        let diff = self.generate_diff(&content, &new_content, path);
        self.atomic_write(path, &new_content)?;

        Ok(EditResult::ok(total_replacements, diff))
    }

    /// Generate a unified diff between two strings
    pub fn generate_diff(&self, old: &str, new: &str, path: &Path) -> String {
        let diff = TextDiff::from_lines(old, new);

        let mut output = String::new();
        output.push_str(&format!("--- a/{}\n", path.display()));
        output.push_str(&format!("+++ b/{}\n", path.display()));

        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                output.push('\n');
            }

            for op in group {
                for change in diff.iter_changes(op) {
                    let sign = match change.tag() {
                        ChangeTag::Delete => '-',
                        ChangeTag::Insert => '+',
                        ChangeTag::Equal => ' ',
                    };

                    output.push(sign);
                    output.push_str(change.value());
                    if !change.missing_newline() {
                        output.push('\n');
                    }
                }
            }
        }

        output
    }

    /// Apply a unified diff to a file
    pub fn apply_diff(&self, _path: &Path, _diff: &str) -> Result<EditResult> {
        // Parse the diff and apply changes
        // This is a simplified implementation - a full implementation would use
        // a proper diff parsing library
        anyhow::bail!("apply_diff is not yet implemented - use edit() instead")
    }

    /// Compute a diff between two files
    pub fn diff_files(&self, old_path: &Path, new_path: &Path) -> Result<String> {
        let old_content = fs::read_to_string(old_path)
            .with_context(|| format!("Failed to read file: {}", old_path.display()))?;
        let new_content = fs::read_to_string(new_path)
            .with_context(|| format!("Failed to read file: {}", new_path.display()))?;

        Ok(self.generate_diff(&old_content, &new_content, new_path))
    }

    /// Atomic write to a file
    fn atomic_write(&self, path: &Path, content: &str) -> Result<()> {
        let parent = path.parent().unwrap_or(Path::new("."));

        // Create a temporary file in the same directory
        let temp_path = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));

        // Write to temporary file
        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temporary file: {}", temp_path.display()))?;

        // Rename temporary file to target (atomic on most filesystems)
        fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to rename temporary file to: {}", path.display()))?;

        Ok(())
    }
}

/// Compute the similarity ratio between two strings (0.0 to 1.0)
pub fn similarity_ratio(s1: &str, s2: &str) -> f64 {
    let diff = TextDiff::from_chars(s1, s2);
    diff.ratio() as f64
}

/// Find the best match for a string in a list of candidates
pub fn find_best_match<'a>(needle: &str, haystack: &[&'a str]) -> Option<(&'a str, f64)> {
    haystack
        .iter()
        .map(|&s| (s, similarity_ratio(needle, s)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .filter(|(_, ratio)| *ratio > 0.6)
}

// ============================================================================
// Fuzzy Replacer Algorithms
// ============================================================================

/// Calculate Levenshtein edit distance between two strings
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.is_empty() {
        return b_chars.len();
    }
    if b_chars.is_empty() {
        return a_chars.len();
    }

    // Ensure a is the shorter string (memory optimization)
    let (a_chars, b_chars) = if a_chars.len() > b_chars.len() {
        (b_chars, a_chars)
    } else {
        (a_chars, b_chars)
    };

    let mut row: Vec<usize> = (0..=a_chars.len()).collect();

    for (j, b_char) in b_chars.iter().enumerate() {
        let mut prev = row[0];
        row[0] = j + 1;

        for (i, a_char) in a_chars.iter().enumerate() {
            let current = row[i + 1];
            let cost = if a_char == b_char { 0 } else { 1 };
            row[i + 1] = (row[i] + 1).min(row[i + 1] + 1).min(prev + cost);
            prev = current;
        }
    }

    row[a_chars.len()]
}

/// Calculate Jaro similarity between two strings (0.0 to 1.0)
///
/// The Jaro similarity is based on matching characters within a window
/// and transpositions of matched characters.
pub fn jaro_similarity(s1: &str, s2: &str) -> f64 {
    if s1.is_empty() && s2.is_empty() {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }
    if s1 == s2 {
        return 1.0;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let s1_len = s1_chars.len();
    let s2_len = s2_chars.len();

    // Match window size
    let match_distance = (s1_len.max(s2_len) / 2).saturating_sub(1);

    let mut s1_matches = vec![false; s1_len];
    let mut s2_matches = vec![false; s2_len];

    let mut matches = 0usize;
    let mut transpositions = 0usize;

    // Find matching characters
    for i in 0..s1_len {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(s2_len);

        for j in start..end {
            if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    // Count transpositions
    let mut k = 0usize;
    for i in 0..s1_len {
        if !s1_matches[i] {
            continue;
        }
        while !s2_matches[k] {
            k += 1;
        }
        if s1_chars[i] != s2_chars[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let m = matches as f64;
    let t = (transpositions / 2) as f64;

    (m / s1_len as f64 + m / s2_len as f64 + (m - t) / m) / 3.0
}

/// Calculate Jaro-Winkler similarity between two strings (0.0 to 1.0)
///
/// Jaro-Winkler is a modification of Jaro that gives more weight to strings
/// that share a common prefix. This is particularly useful for code matching
/// where identifiers often share prefixes.
///
/// # Arguments
/// * `s1` - First string to compare
/// * `s2` - Second string to compare
/// * `prefix_weight` - Weight given to common prefix (typically 0.1, max 0.25)
///
/// # Returns
/// Similarity score from 0.0 (completely different) to 1.0 (identical)
pub fn jaro_winkler_similarity(s1: &str, s2: &str, prefix_weight: Option<f64>) -> f64 {
    let jaro = jaro_similarity(s1, s2);

    // Clamp prefix weight to valid range [0, 0.25]
    let p = prefix_weight.unwrap_or(0.1).clamp(0.0, 0.25);

    // Find common prefix length (max 4 characters)
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let prefix_len = s1_chars
        .iter()
        .zip(s2_chars.iter())
        .take(4)
        .take_while(|(a, b)| a == b)
        .count();

    jaro + (prefix_len as f64 * p * (1.0 - jaro))
}

/// A fuzzy match result with position and score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyMatch {
    /// Start position in haystack
    pub start: usize,
    /// End position in haystack
    pub end: usize,
    /// Similarity score (0.0 to 1.0)
    pub score: f64,
    /// The matched text
    pub matched_text: String,
}

/// Find the best fuzzy match for a needle in a haystack
///
/// Uses Jaro-Winkler for similarity scoring with a sliding window approach.
/// Returns the best match if it exceeds the threshold, None otherwise.
pub fn fuzzy_find(needle: &str, haystack: &str, threshold: f64) -> Option<FuzzyMatch> {
    if needle.is_empty() || haystack.is_empty() {
        return None;
    }

    let needle_len = needle.len();
    let haystack_len = haystack.len();

    if needle_len > haystack_len {
        return None;
    }

    let mut best_match: Option<FuzzyMatch> = None;
    let mut best_score = threshold;

    // Sliding window approach
    for start in 0..=(haystack_len - needle_len) {
        // Try windows of varying sizes around the needle length
        for size_delta in 0..=needle_len.min(20) {
            let window_size = needle_len + size_delta;
            if start + window_size > haystack_len {
                break;
            }

            let window = &haystack[start..start + window_size];
            let score = jaro_winkler_similarity(needle, window, None);

            if score > best_score {
                best_score = score;
                best_match = Some(FuzzyMatch {
                    start,
                    end: start + window_size,
                    score,
                    matched_text: window.to_string(),
                });
            }
        }
    }

    best_match
}

/// Find the best match among multiple candidates
///
/// Returns the candidate with the highest similarity score above the threshold
pub fn find_best_fuzzy_match<'a>(
    needle: &str,
    candidates: &[&'a str],
    threshold: f64,
) -> Option<(&'a str, f64)> {
    candidates
        .iter()
        .map(|&s| (s, jaro_winkler_similarity(needle, s, None)))
        .filter(|(_, score)| *score >= threshold)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
}

/// Result of a fuzzy replace operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyReplaceResult {
    /// The new content after replacement
    pub content: String,

    /// Whether a match was found
    pub found: bool,

    /// The actual string that was matched (may differ from old_string)
    pub matched_string: Option<String>,

    /// Which replacer strategy succeeded
    pub strategy: Option<String>,

    /// Error message if no match found
    pub error: Option<String>,
}

impl FuzzyReplaceResult {
    fn ok(content: String, matched_string: String, strategy: &str) -> Self {
        Self {
            content,
            found: true,
            matched_string: Some(matched_string),
            strategy: Some(strategy.to_string()),
            error: None,
        }
    }

    fn err(error: impl Into<String>) -> Self {
        Self {
            content: String::new(),
            found: false,
            matched_string: None,
            strategy: None,
            error: Some(error.into()),
        }
    }
}

/// Replace content using fuzzy matching with multiple strategies
///
/// Tries the following replacer strategies in order:
/// 1. Simple - exact match
/// 2. LineTrimmed - line-by-line trimmed comparison
/// 3. BlockAnchor - first/last line anchors with similarity scoring
/// 4. WhitespaceNormalized - normalizes whitespace before matching
/// 5. IndentationFlexible - removes indentation before matching
/// 6. EscapeNormalized - handles escaped strings
/// 7. TrimmedBoundary - trims boundaries before matching
/// 8. ContextAware - uses context lines as anchors
/// 9. MultiOccurrence - yields all exact matches
pub fn replace_with_fuzzy_match(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> FuzzyReplaceResult {
    if old_string == new_string {
        return FuzzyReplaceResult::err("old_string and new_string must be different");
    }

    let replacers: Vec<(&str, fn(&str, &str) -> Vec<String>)> = vec![
        ("Simple", simple_replacer),
        ("LineTrimmed", line_trimmed_replacer),
        ("BlockAnchor", block_anchor_replacer),
        ("WhitespaceNormalized", whitespace_normalized_replacer),
        ("IndentationFlexible", indentation_flexible_replacer),
        ("EscapeNormalized", escape_normalized_replacer),
        ("TrimmedBoundary", trimmed_boundary_replacer),
        ("ContextAware", context_aware_replacer),
        ("MultiOccurrence", multi_occurrence_replacer),
    ];

    let mut found_any = false;

    for (name, replacer) in replacers {
        for search in replacer(content, old_string) {
            if let Some(index) = content.find(&search) {
                found_any = true;

                if replace_all {
                    let new_content = content.replace(&search, new_string);
                    return FuzzyReplaceResult::ok(new_content, search, name);
                }

                // Check uniqueness
                if let Some(last_index) = content.rfind(&search) {
                    if index != last_index {
                        // Multiple matches, continue to next replacer
                        continue;
                    }
                }

                let mut new_content = String::with_capacity(content.len());
                new_content.push_str(&content[..index]);
                new_content.push_str(new_string);
                new_content.push_str(&content[index + search.len()..]);

                return FuzzyReplaceResult::ok(new_content, search, name);
            }
        }
    }

    if found_any {
        FuzzyReplaceResult::err(
            "Found multiple matches for oldString. Provide more surrounding lines in oldString to identify the correct match.",
        )
    } else {
        FuzzyReplaceResult::err("oldString not found in content")
    }
}

/// Simple replacer - exact match
fn simple_replacer(_content: &str, find: &str) -> Vec<String> {
    vec![find.to_string()]
}

/// Line-trimmed replacer - matches lines with trimmed whitespace
fn line_trimmed_replacer(content: &str, find: &str) -> Vec<String> {
    let original_lines: Vec<&str> = content.split('\n').collect();
    let mut search_lines: Vec<&str> = find.split('\n').collect();

    // Remove trailing empty line if present
    if search_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        search_lines.pop();
    }

    let mut matches = Vec::new();

    for i in 0..=original_lines.len().saturating_sub(search_lines.len()) {
        let mut is_match = true;

        for j in 0..search_lines.len() {
            if i + j >= original_lines.len() {
                is_match = false;
                break;
            }

            if original_lines[i + j].trim() != search_lines[j].trim() {
                is_match = false;
                break;
            }
        }

        if is_match && !search_lines.is_empty() {
            // Calculate the actual matched text
            let match_start = original_lines[..i].iter().map(|l| l.len() + 1).sum::<usize>();
            let mut match_end = match_start;
            for k in 0..search_lines.len() {
                match_end += original_lines[i + k].len();
                if k < search_lines.len() - 1 {
                    match_end += 1; // newline
                }
            }

            if match_end <= content.len() {
                matches.push(content[match_start..match_end].to_string());
            }
        }
    }

    matches
}

/// Block anchor replacer - uses first/last line as anchors with similarity scoring
fn block_anchor_replacer(content: &str, find: &str) -> Vec<String> {
    let original_lines: Vec<&str> = content.split('\n').collect();
    let mut search_lines: Vec<&str> = find.split('\n').collect();

    if search_lines.len() < 3 {
        return vec![];
    }

    // Remove trailing empty line if present
    if search_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        search_lines.pop();
    }

    let first_line_search = search_lines[0].trim();
    let last_line_search = search_lines[search_lines.len() - 1].trim();
    let search_block_size = search_lines.len();

    // Collect all candidate positions
    let mut candidates: Vec<(usize, usize)> = Vec::new();

    for i in 0..original_lines.len() {
        if original_lines[i].trim() != first_line_search {
            continue;
        }

        for j in (i + 2)..original_lines.len() {
            if original_lines[j].trim() == last_line_search {
                candidates.push((i, j));
                break;
            }
        }
    }

    if candidates.is_empty() {
        return vec![];
    }

    // Handle single candidate
    if candidates.len() == 1 {
        let (start_line, end_line) = candidates[0];
        let actual_block_size = end_line - start_line + 1;

        let similarity = calculate_block_similarity(
            &original_lines,
            &search_lines,
            start_line,
            actual_block_size,
            search_block_size,
        );

        if similarity >= SINGLE_CANDIDATE_SIMILARITY_THRESHOLD {
            return vec![extract_block(content, &original_lines, start_line, end_line)];
        }
        return vec![];
    }

    // Multiple candidates - find best match
    let mut best_match: Option<(usize, usize)> = None;
    let mut max_similarity: f64 = -1.0;

    for (start_line, end_line) in &candidates {
        let actual_block_size = end_line - start_line + 1;
        let similarity = calculate_block_similarity(
            &original_lines,
            &search_lines,
            *start_line,
            actual_block_size,
            search_block_size,
        );

        if similarity > max_similarity {
            max_similarity = similarity;
            best_match = Some((*start_line, *end_line));
        }
    }

    if max_similarity >= MULTIPLE_CANDIDATES_SIMILARITY_THRESHOLD {
        if let Some((start_line, end_line)) = best_match {
            return vec![extract_block(content, &original_lines, start_line, end_line)];
        }
    }

    vec![]
}

fn calculate_block_similarity(
    original_lines: &[&str],
    search_lines: &[&str],
    start_line: usize,
    actual_block_size: usize,
    search_block_size: usize,
) -> f64 {
    let lines_to_check = (search_block_size - 2).min(actual_block_size - 2);

    if lines_to_check == 0 {
        return 1.0;
    }

    let mut total_similarity = 0.0;
    let mut comparisons = 0;

    for j in 1..search_block_size.min(actual_block_size) - 1 {
        if start_line + j >= original_lines.len() {
            break;
        }

        let original_line = original_lines[start_line + j].trim();
        let search_line = search_lines[j].trim();
        let max_len = original_line.len().max(search_line.len());

        if max_len == 0 {
            continue;
        }

        let distance = levenshtein_distance(original_line, search_line);
        total_similarity += 1.0 - (distance as f64 / max_len as f64);
        comparisons += 1;
    }

    if comparisons > 0 {
        total_similarity / comparisons as f64
    } else {
        1.0
    }
}

fn extract_block(content: &str, lines: &[&str], start_line: usize, end_line: usize) -> String {
    let match_start: usize = lines[..start_line].iter().map(|l| l.len() + 1).sum();
    let mut match_end = match_start;
    for k in start_line..=end_line {
        match_end += lines[k].len();
        if k < end_line {
            match_end += 1;
        }
    }

    if match_end <= content.len() {
        content[match_start..match_end].to_string()
    } else {
        String::new()
    }
}

/// Whitespace-normalized replacer - normalizes whitespace before matching
fn whitespace_normalized_replacer(content: &str, find: &str) -> Vec<String> {
    let normalize_whitespace = |text: &str| -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    };

    let normalized_find = normalize_whitespace(find);
    let mut matches = Vec::new();

    // Handle single line matches
    let lines: Vec<&str> = content.split('\n').collect();
    for line in &lines {
        let normalized_line = normalize_whitespace(line);

        if normalized_line == normalized_find {
            matches.push(line.to_string());
        } else if normalized_line.contains(&normalized_find) {
            // Try to find the actual substring
            let words: Vec<&str> = find.split_whitespace().collect();
            if !words.is_empty() {
                let pattern = words
                    .iter()
                    .map(|w| regex::escape(w))
                    .collect::<Vec<_>>()
                    .join(r"\s+");

                if let Ok(re) = Regex::new(&pattern) {
                    if let Some(m) = re.find(line) {
                        matches.push(m.as_str().to_string());
                    }
                }
            }
        }
    }

    // Handle multi-line matches
    let find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.len() > 1 && lines.len() >= find_lines.len() {
        for i in 0..=lines.len() - find_lines.len() {
            let block: Vec<&str> = lines[i..i + find_lines.len()].to_vec();
            let normalized_block = normalize_whitespace(&block.join("\n"));

            if normalized_block == normalized_find {
                matches.push(block.join("\n"));
            }
        }
    }

    matches
}

/// Indentation-flexible replacer - removes indentation before matching
fn indentation_flexible_replacer(content: &str, find: &str) -> Vec<String> {
    let remove_indentation = |text: &str| -> String {
        let lines: Vec<&str> = text.split('\n').collect();
        let non_empty_lines: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();

        if non_empty_lines.is_empty() {
            return text.to_string();
        }

        let min_indent = non_empty_lines
            .iter()
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        lines
            .iter()
            .map(|line| {
                if line.trim().is_empty() {
                    *line
                } else if line.len() >= min_indent {
                    &line[min_indent..]
                } else {
                    *line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let normalized_find = remove_indentation(find);
    let content_lines: Vec<&str> = content.split('\n').collect();
    let find_lines: Vec<&str> = find.split('\n').collect();
    let mut matches = Vec::new();

    if content_lines.len() >= find_lines.len() {
        for i in 0..=content_lines.len() - find_lines.len() {
            let block = content_lines[i..i + find_lines.len()].join("\n");

            if remove_indentation(&block) == normalized_find {
                matches.push(block);
            }
        }
    }

    matches
}

/// Escape-normalized replacer - handles escaped strings
fn escape_normalized_replacer(content: &str, find: &str) -> Vec<String> {
    let unescape_string = |s: &str| -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    let escaped = match next {
                        'n' => {
                            chars.next();
                            '\n'
                        }
                        't' => {
                            chars.next();
                            '\t'
                        }
                        'r' => {
                            chars.next();
                            '\r'
                        }
                        '\'' | '"' | '`' | '\\' | '$' => {
                            chars.next();
                            next
                        }
                        '\n' => {
                            chars.next();
                            '\n'
                        }
                        _ => {
                            result.push(c);
                            continue;
                        }
                    };
                    result.push(escaped);
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    };

    let unescaped_find = unescape_string(find);
    let mut matches = Vec::new();

    // Try direct match with unescaped find string
    if content.contains(&unescaped_find) {
        matches.push(unescaped_find.clone());
    }

    // Try finding escaped versions in content
    let lines: Vec<&str> = content.split('\n').collect();
    let find_lines: Vec<&str> = unescaped_find.split('\n').collect();

    if lines.len() >= find_lines.len() {
        for i in 0..=lines.len() - find_lines.len() {
            let block = lines[i..i + find_lines.len()].join("\n");
            let unescaped_block = unescape_string(&block);

            if unescaped_block == unescaped_find {
                matches.push(block);
            }
        }
    }

    matches
}

/// Trimmed-boundary replacer - trims boundaries before matching
fn trimmed_boundary_replacer(content: &str, find: &str) -> Vec<String> {
    let trimmed_find = find.trim();

    if trimmed_find == find {
        return vec![];
    }

    let mut matches = Vec::new();

    // Try direct match
    if content.contains(trimmed_find) {
        matches.push(trimmed_find.to_string());
    }

    // Try block matches
    let lines: Vec<&str> = content.split('\n').collect();
    let find_lines: Vec<&str> = find.split('\n').collect();

    if lines.len() >= find_lines.len() {
        for i in 0..=lines.len() - find_lines.len() {
            let block = lines[i..i + find_lines.len()].join("\n");

            if block.trim() == trimmed_find {
                matches.push(block);
            }
        }
    }

    matches
}

/// Context-aware replacer - uses context lines as anchors
fn context_aware_replacer(content: &str, find: &str) -> Vec<String> {
    let mut find_lines: Vec<&str> = find.split('\n').collect();

    if find_lines.len() < 3 {
        return vec![];
    }

    // Remove trailing empty line
    if find_lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        find_lines.pop();
    }

    let content_lines: Vec<&str> = content.split('\n').collect();
    let first_line = find_lines[0].trim();
    let last_line = find_lines[find_lines.len() - 1].trim();
    let mut matches = Vec::new();

    for i in 0..content_lines.len() {
        if content_lines[i].trim() != first_line {
            continue;
        }

        for j in (i + 2)..content_lines.len() {
            if content_lines[j].trim() == last_line {
                let block_lines = &content_lines[i..=j];

                if block_lines.len() == find_lines.len() {
                    let mut matching_lines = 0;
                    let mut total_non_empty = 0;

                    for k in 1..block_lines.len() - 1 {
                        let block_line = block_lines[k].trim();
                        let find_line = find_lines[k].trim();

                        if !block_line.is_empty() || !find_line.is_empty() {
                            total_non_empty += 1;
                            if block_line == find_line {
                                matching_lines += 1;
                            }
                        }
                    }

                    if total_non_empty == 0
                        || (matching_lines as f64 / total_non_empty as f64) >= 0.5
                    {
                        matches.push(block_lines.join("\n"));
                        break;
                    }
                }
                break;
            }
        }
    }

    matches
}

/// Multi-occurrence replacer - yields all exact matches
fn multi_occurrence_replacer(content: &str, find: &str) -> Vec<String> {
    let mut matches = Vec::new();
    let mut start = 0;

    while let Some(index) = content[start..].find(find) {
        matches.push(find.to_string());
        start += index + find.len();
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_edit_single_replacement() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "Hello, world!");

        let editor = Editor::new();
        let operation = EditOperation {
            old_string: "world".to_string(),
            new_string: "Rust".to_string(),
            replace_all: false,
        };

        let result = editor.edit(&path, &operation).unwrap();
        assert!(result.success);
        assert_eq!(result.replacements, 1);
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, Rust!");
    }

    #[test]
    fn test_edit_replace_all() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "foo bar foo baz foo");

        let editor = Editor::new();
        let operation = EditOperation {
            old_string: "foo".to_string(),
            new_string: "qux".to_string(),
            replace_all: true,
        };

        let result = editor.edit(&path, &operation).unwrap();
        assert!(result.success);
        assert_eq!(result.replacements, 3);
        assert_eq!(fs::read_to_string(&path).unwrap(), "qux bar qux baz qux");
    }

    #[test]
    fn test_edit_non_unique_fails() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "foo bar foo baz foo");

        let editor = Editor::new();
        let operation = EditOperation {
            old_string: "foo".to_string(),
            new_string: "qux".to_string(),
            replace_all: false,
        };

        let result = editor.edit(&path, &operation).unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not unique"));
    }

    #[test]
    fn test_edit_not_found() {
        let dir = TempDir::new().unwrap();
        let path = setup_test_file(&dir, "test.txt", "Hello, world!");

        let editor = Editor::new();
        let operation = EditOperation {
            old_string: "nonexistent".to_string(),
            new_string: "replacement".to_string(),
            replace_all: false,
        };

        let result = editor.edit(&path, &operation).unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not found"));
    }

    #[test]
    fn test_similarity_ratio() {
        assert!(similarity_ratio("hello", "hello") > 0.99);
        assert!(similarity_ratio("hello", "helo") > 0.7);
        assert!(similarity_ratio("hello", "world") < 0.5);
    }

    #[test]
    fn test_find_best_match() {
        let candidates = vec!["apple", "application", "banana", "app"];
        let (best, ratio) = find_best_match("appl", &candidates).unwrap();
        // "apple" has higher similarity to "appl" than "app" because it contains "appl" as prefix
        assert_eq!(best, "apple");
        assert!(ratio > 0.7);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", "world"), 5);
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_fuzzy_replace_simple() {
        let content = "Hello, world!";
        let result = replace_with_fuzzy_match(content, "world", "Rust", false);
        assert!(result.found);
        assert_eq!(result.content, "Hello, Rust!");
        assert_eq!(result.strategy, Some("Simple".to_string()));
    }

    #[test]
    fn test_fuzzy_replace_line_trimmed() {
        let content = "  function foo() {  \n    return bar;\n  }";
        let find = "function foo() {\n  return bar;\n}";
        let result = replace_with_fuzzy_match(content, find, "function baz() {}", false);
        assert!(result.found);
        assert!(result.strategy == Some("LineTrimmed".to_string()));
    }

    #[test]
    fn test_fuzzy_replace_whitespace_normalized() {
        let content = "const   x   =   1;";
        let find = "const x = 1";
        let result = replace_with_fuzzy_match(content, find, "const y = 2", false);
        assert!(result.found);
        assert!(result.strategy == Some("WhitespaceNormalized".to_string()));
    }

    #[test]
    fn test_fuzzy_replace_indentation_flexible() {
        // Test indentation flexibility - content has different indentation than search
        // but same structure after removing leading indentation
        let content = "        function foo() {\n            return 1;\n        }";
        let find = "    function foo() {\n        return 1;\n    }";
        let result = replace_with_fuzzy_match(content, find, "function bar() {}", false);
        // The key is that it finds the match despite different indentation levels
        assert!(result.found, "Should find match with different indentation: {:?}", result);
    }

    #[test]
    fn test_fuzzy_replace_not_found() {
        let content = "Hello, world!";
        let result = replace_with_fuzzy_match(content, "nonexistent", "replacement", false);
        assert!(!result.found);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_fuzzy_replace_multiple_matches() {
        let content = "foo bar foo baz foo";
        let result = replace_with_fuzzy_match(content, "foo", "qux", false);
        assert!(!result.found);
        assert!(result.error.as_ref().unwrap().contains("multiple matches"));
    }

    #[test]
    fn test_fuzzy_replace_all() {
        let content = "foo bar foo baz foo";
        let result = replace_with_fuzzy_match(content, "foo", "qux", true);
        assert!(result.found);
        assert_eq!(result.content, "qux bar qux baz qux");
    }

    #[test]
    fn test_fuzzy_replace_same_strings() {
        let content = "Hello, world!";
        let result = replace_with_fuzzy_match(content, "world", "world", false);
        assert!(!result.found);
        assert!(result
            .error
            .as_ref()
            .unwrap()
            .contains("must be different"));
    }

    #[test]
    fn test_fuzzy_replace_escape_normalized() {
        // Test that escaped strings in the search can match literal content
        let content = "const msg = \"Hello\nWorld\";";  // Content has actual newline
        let find = "const msg = \"Hello\\nWorld\";";    // Search has escaped newline
        let result = replace_with_fuzzy_match(content, find, "const msg = \"Hi\";", false);
        // This should use escape normalized - it converts \\n in find to \n
        assert!(result.found, "Should find match with escape normalization: {:?}", result);
    }

    #[test]
    fn test_fuzzy_replace_block_anchor() {
        let content = "function test() {\n  // some comment\n  return 42;\n}";
        let find = "function test() {\n  // different comment\n}";
        let result = replace_with_fuzzy_match(content, find, "function newTest() {}", false);
        // BlockAnchor should match based on first/last line anchors
        assert!(result.found);
        assert!(result.strategy == Some("BlockAnchor".to_string()));
    }

    // ========================================================================
    // Jaro-Winkler Tests
    // ========================================================================

    #[test]
    fn test_jaro_similarity_identical() {
        assert!((jaro_similarity("hello", "hello") - 1.0).abs() < 0.001);
        assert!((jaro_similarity("", "") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_similarity_empty() {
        assert!((jaro_similarity("hello", "") - 0.0).abs() < 0.001);
        assert!((jaro_similarity("", "hello") - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_similarity_similar_strings() {
        // Classic test cases
        let sim = jaro_similarity("MARTHA", "MARHTA");
        assert!(sim > 0.9, "Expected > 0.9, got {}", sim);

        let sim = jaro_similarity("DWAYNE", "DUANE");
        assert!(sim > 0.8, "Expected > 0.8, got {}", sim);

        let sim = jaro_similarity("DIXON", "DICKSONX");
        assert!(sim > 0.7, "Expected > 0.7, got {}", sim);
    }

    #[test]
    fn test_jaro_winkler_identical() {
        assert!((jaro_winkler_similarity("hello", "hello", None) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler_common_prefix_boost() {
        // Jaro-Winkler should give higher score than Jaro for common prefix
        let jaro = jaro_similarity("PREFIX_abc", "PREFIX_xyz");
        let jw = jaro_winkler_similarity("PREFIX_abc", "PREFIX_xyz", None);
        assert!(jw > jaro, "Jaro-Winkler {} should be > Jaro {}", jw, jaro);
    }

    #[test]
    fn test_jaro_winkler_code_identifiers() {
        // Common patterns in code
        assert!(jaro_winkler_similarity("getUserById", "getUserByName", None) > 0.8);
        assert!(jaro_winkler_similarity("handleClick", "handleKeyPress", None) > 0.6);
        assert!(jaro_winkler_similarity("parseJSON", "parseXML", None) > 0.7);
    }

    #[test]
    fn test_jaro_winkler_prefix_weight() {
        let default_weight = jaro_winkler_similarity("PREFIX_a", "PREFIX_b", None);
        let high_weight = jaro_winkler_similarity("PREFIX_a", "PREFIX_b", Some(0.2));
        let low_weight = jaro_winkler_similarity("PREFIX_a", "PREFIX_b", Some(0.05));

        assert!(high_weight > default_weight);
        assert!(default_weight > low_weight);
    }

    #[test]
    fn test_fuzzy_find_basic() {
        let result = fuzzy_find("world", "Hello, world!", 0.8);
        assert!(result.is_some());
        let m = result.unwrap();
        assert!(m.score > 0.99, "Expected exact match, got score {}", m.score);
        assert_eq!(m.matched_text, "world");
    }

    #[test]
    fn test_fuzzy_find_no_match() {
        let result = fuzzy_find("xyz123", "Hello, world!", 0.8);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_best_fuzzy_match() {
        let candidates = vec!["apple", "application", "banana", "app"];
        let result = find_best_fuzzy_match("appl", &candidates, 0.7);
        assert!(result.is_some());
        let (best, score) = result.unwrap();
        assert_eq!(best, "apple");
        assert!(score > 0.8);
    }

    #[test]
    fn test_find_best_fuzzy_match_threshold() {
        let candidates = vec!["completely_different", "nothing_similar"];
        let result = find_best_fuzzy_match("target", &candidates, 0.9);
        assert!(result.is_none(), "Should not find match above high threshold");
    }
}
