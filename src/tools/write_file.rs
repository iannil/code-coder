use super::Tool;

/// Write content to a file.
pub struct WriteFile;

impl Tool for WriteFile {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Input format: <path>\n<content>"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let input = input.trim();
        let newline = input.find('\n').ok_or_else(|| {
            anyhow::anyhow!("write_file requires '<path>\\n<content>'")
        })?;

        let path = input[..newline].trim();
        let content = &input[newline + 1..];

        if path.is_empty() {
            anyhow::bail!("path cannot be empty");
        }

        // Create parent directories if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("cannot create directory {parent:?}: {e}"))?;
        }

        std::fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("cannot write {path}: {e}"))?;

        Ok(format!("wrote {} bytes to {path}", content.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_write_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let input = format!("{}\nhello world", path.to_str().unwrap());
        let result = WriteFile.execute(&input).unwrap();
        assert!(result.contains("wrote"));

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert_eq!(contents, "hello world");
    }

    #[test]
    fn test_write_file_no_newline() {
        let result = WriteFile.execute("just-a-path");
        assert!(result.is_err());
    }
}
