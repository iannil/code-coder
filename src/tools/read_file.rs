use super::{Tool, try_extract_json_field};

/// Read a file from the filesystem.
pub struct ReadFile;

impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Input: file path or JSON object with 'path' field."
    }

    fn input_schema(&self) -> &'static str {
        r#"{"type":"object","properties":{"path":{"type":"string","description":"File path to read"}},"required":["path"]}"#
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let path = try_extract_json_field(input, "path").unwrap_or_else(|| input.trim().to_string());
        let path_clone = path.clone();
        if path.is_empty() {
            anyhow::bail!("read_file requires a file path");
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("cannot read {path_clone}: {e}"))?;
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
