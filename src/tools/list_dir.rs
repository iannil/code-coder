use super::Tool;

/// List directory contents.
pub struct ListDir;

impl Tool for ListDir {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        "List entries in a directory. Input: directory path (default: \".\")."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let path = input.trim();
        let path = if path.is_empty() { "." } else { path };

        let entries = std::fs::read_dir(path)
            .map_err(|e| anyhow::anyhow!("cannot list {path}: {e}"))?;

        let mut result = String::new();
        let mut entry_count = 0;

        for entry in entries {
            let entry = entry.map_err(|e| anyhow::anyhow!("read error: {e}"))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry.file_type().ok();
            let is_dir = file_type.map(|t| t.is_dir()).unwrap_or(false);
            let is_symlink = file_type.map(|t| t.is_symlink()).unwrap_or(false);

            if is_dir {
                result.push_str(&format!("{}/\n", name));
            } else if is_symlink {
                result.push_str(&format!("{}@\n", name));
            } else {
                result.push_str(&format!("{}\n", name));
            }
            entry_count += 1;
        }

        Ok(format!("{} entries:\n{}", entry_count, result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_current_dir() {
        let result = ListDir.execute(".").unwrap();
        assert!(result.contains("entries:"));
    }

    #[test]
    fn test_list_nonexistent() {
        let result = ListDir.execute("/nonexistent_path_xyz");
        assert!(result.is_err());
    }
}
