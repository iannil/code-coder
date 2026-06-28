/// Path validation for permission system.
/// Checks whether a file path is within an allowed set of root directories.

use std::path::Path;

/// Validate that `path` is within one of the `allowed_roots`.
/// Returns `Ok(())` if valid, or an error message explaining why not.
pub fn validate_path(path_str: &str, allowed_roots: &[String]) -> Result<(), String> {
    let path = Path::new(path_str);

    // Reject absolute paths that don't match any allowed root
    if path.is_absolute() {
        for root in allowed_roots {
            if path_str.starts_with(root) {
                return Ok(());
            }
        }
        return Err(format!(
            "absolute path '{path_str}' is outside allowed roots"
        ));
    }

    // Relative paths are generally OK (constrained by project root)
    // Reject path traversal attempts
    if path_str.contains("..") {
        return Err(format!("path traversal detected: '{path_str}'"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path_allowed() {
        let roots = vec!["/home/user/project".to_string()];
        assert!(validate_path("src/main.rs", &roots).is_ok());
    }

    #[test]
    fn test_path_traversal_denied() {
        let roots = vec!["/home/user/project".to_string()];
        assert!(validate_path("../etc/passwd", &roots).is_err());
    }

    #[test]
    fn test_double_dot_in_middle() {
        let roots = vec!["/home/user/project".to_string()];
        assert!(validate_path("src/../../etc/passwd", &roots).is_err());
    }

    #[test]
    fn test_absolute_path_outside_root() {
        let roots = vec!["/home/user/project".to_string()];
        assert!(validate_path("/etc/passwd", &roots).is_err());
    }

    #[test]
    fn test_absolute_path_inside_root() {
        let roots = vec!["/home/user/project".to_string()];
        assert!(validate_path("/home/user/project/src/main.rs", &roots).is_ok());
    }
}
