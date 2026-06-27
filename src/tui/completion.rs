/// ─── File Completion Helper ────────────────────────────────────────────────
///
/// Implements @ file completion: detect `@` in input, scan project root,
/// filter by partial path, return matching candidates.

use std::path::Path;

/// A file candidate for completion
#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    /// Display name (relative path from project root)
    pub display: String,
    /// Full path
    pub path: String,
}

/// Scan the project root for files matching a partial query.
/// Returns up to `max_results` candidates.
pub fn search_files(project_root: &str, query: &str, max_results: usize) -> Vec<CompletionCandidate> {
    let root = Path::new(project_root);
    if !root.exists() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    // Walk the directory tree (limited depth)
    if let Ok(entries) = std::fs::read_dir(root) {
        let mut dirs: Vec<(String, String)> = Vec::new(); // (relative, full)

        for entry in entries.flatten() {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Skip hidden files/dirs
            if relative.starts_with('.') || relative.contains("/.") {
                continue;
            }
            // Skip common non-code directories
            if relative == "node_modules"
                || relative == "target"
                || relative == ".git"
                || relative == ".reasonix"
            {
                continue;
            }

            if path.is_dir() {
                dirs.push((relative, path.to_string_lossy().to_string()));
            } else if query_lower.is_empty()
                || relative.to_lowercase().contains(&query_lower)
            {
                results.push(CompletionCandidate {
                    display: relative.clone(),
                    path: path.to_string_lossy().to_string(),
                });
            }

            if results.len() >= max_results {
                break;
            }
        }

        // Also search one level into subdirectories (shallow)
        for (dir_rel, dir_path) in dirs.iter().take(20) {
            if results.len() >= max_results {
                break;
            }
            if let Ok(sub_entries) = std::fs::read_dir(dir_path) {
                for entry in sub_entries.flatten() {
                    if results.len() >= max_results {
                        break;
                    }
                    let sub_path = entry.path();
                    if sub_path.is_dir() {
                        continue;
                    }
                    let sub_relative = format!("{}/{}", dir_rel, sub_path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                    );
                    if sub_relative.starts_with('.') || sub_relative.contains("/.") {
                        continue;
                    }
                    if query_lower.is_empty()
                        || sub_relative.to_lowercase().contains(&query_lower)
                    {
                        results.push(CompletionCandidate {
                            display: sub_relative,
                            path: sub_path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }

    results
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_empty_query() {
        let results = search_files("/tmp", "", 5);
        // Should return some files without crashing
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_search_nonexistent_root() {
        let results = search_files("/nonexistent/path/xyz", "test", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_limits_results() {
        let results = search_files("/", "", 3);
        assert!(results.len() <= 3);
    }
}
