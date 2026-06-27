use super::Tool;

/// Read a file from the filesystem.
pub struct ReadFile;

impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Input: file path."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let path = input.trim();
        if path.is_empty() {
            anyhow::bail!("read_file requires a file path");
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read {path}: {e}"))?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "hello world").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let result = ReadFile.execute(&path).unwrap();
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_read_file_empty_path() {
        let result = ReadFile.execute("");
        assert!(result.is_err());
    }
}
