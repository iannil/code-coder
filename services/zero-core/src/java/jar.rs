//! JAR file reader and extractor
//!
//! Provides in-memory JAR/ZIP extraction using the `zip` crate,
//! eliminating the need for external `unzip` commands.
//!
//! # Example
//!
//! ```ignore
//! let reader = JarReader::open("myapp.jar")?;
//! for class_info in reader.class_files()? {
//!     println!("{}", class_info.name);
//! }
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use rayon::prelude::*;
use thiserror::Error;
use zip::ZipArchive;

use super::classfile::{parse_class_file, ClassFileError, ClassInfo};

/// Errors that can occur during JAR reading
#[derive(Error, Debug)]
pub enum JarError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Class file error: {0}")]
    ClassFile(#[from] ClassFileError),

    #[error("JAR file not found: {0}")]
    NotFound(String),
}

/// Entry in a JAR file
#[derive(Debug, Clone)]
pub struct JarEntry {
    /// Path within the JAR
    pub path: String,
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Uncompressed size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
}

/// JAR file reader with in-memory extraction
pub struct JarReader {
    archive: ZipArchive<BufReader<File>>,
    path: String,
}

impl JarReader {
    /// Open a JAR file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, JarError> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        if !path.as_ref().exists() {
            return Err(JarError::NotFound(path_str));
        }

        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let archive = ZipArchive::new(reader)?;

        Ok(Self {
            archive,
            path: path_str,
        })
    }

    /// Get the JAR file path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the number of entries in the JAR
    pub fn len(&self) -> usize {
        self.archive.len()
    }

    /// Check if the JAR is empty
    pub fn is_empty(&self) -> bool {
        self.archive.is_empty()
    }

    /// List all entries in the JAR
    pub fn entries(&mut self) -> Vec<JarEntry> {
        (0..self.archive.len())
            .filter_map(|i| {
                let file = self.archive.by_index_raw(i).ok()?;
                Some(JarEntry {
                    path: file.name().to_string(),
                    compressed_size: file.compressed_size(),
                    size: file.size(),
                    is_dir: file.is_dir(),
                })
            })
            .collect()
    }

    /// Get paths of all .class files
    pub fn class_file_paths(&mut self) -> Vec<String> {
        self.entries()
            .into_iter()
            .filter(|e| !e.is_dir && e.path.ends_with(".class"))
            .map(|e| e.path)
            .collect()
    }

    /// Read a file from the JAR into memory
    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, JarError> {
        let mut file = self.archive.by_name(path)?;
        let mut contents = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut contents)?;
        Ok(contents)
    }

    /// Read MANIFEST.MF if present
    pub fn read_manifest(&mut self) -> Option<String> {
        self.read_file("META-INF/MANIFEST.MF")
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    /// Parse all class files in the JAR (sequential)
    pub fn parse_classes(&mut self, max_classes: Option<usize>) -> Result<Vec<ClassInfo>, JarError> {
        let class_paths: Vec<_> = self.class_file_paths();
        let limit = max_classes.unwrap_or(class_paths.len()).min(class_paths.len());

        let mut classes = Vec::with_capacity(limit);

        for path in class_paths.into_iter().take(limit) {
            if let Ok(data) = self.read_file(&path) {
                if let Ok(cf) = parse_class_file(&data) {
                    classes.push(cf.into());
                }
            }
        }

        Ok(classes)
    }

    /// Parse all class files in parallel (reads files first, then parses in parallel)
    pub fn parse_classes_parallel(
        &mut self,
        max_classes: Option<usize>,
    ) -> Result<Vec<ClassInfo>, JarError> {
        let class_paths: Vec<_> = self.class_file_paths();
        let limit = max_classes.unwrap_or(class_paths.len()).min(class_paths.len());

        // First, read all class files into memory (sequential, due to ZipArchive)
        let mut class_data: Vec<(String, Vec<u8>)> = Vec::with_capacity(limit);
        for path in class_paths.into_iter().take(limit) {
            if let Ok(data) = self.read_file(&path) {
                class_data.push((path, data));
            }
        }

        // Then parse in parallel using rayon
        let classes: Vec<ClassInfo> = class_data
            .into_par_iter()
            .filter_map(|(_path, data)| {
                parse_class_file(&data).ok().map(|cf| cf.into())
            })
            .collect();

        Ok(classes)
    }

    /// Find configuration files (properties, xml, yml, yaml)
    pub fn find_config_files(&mut self) -> Vec<String> {
        self.entries()
            .into_iter()
            .filter(|e| {
                if e.is_dir {
                    return false;
                }
                let path = e.path.to_lowercase();
                path.ends_with(".properties")
                    || path.ends_with(".xml")
                    || path.ends_with(".yml")
                    || path.ends_with(".yaml")
            })
            .filter(|e| {
                // Skip META-INF/maven files (dependency info)
                !e.path.contains("META-INF/maven/")
                    // Skip signature files
                    && !e.path.ends_with(".SF")
                    && !e.path.ends_with(".DSA")
                    && !e.path.ends_with(".RSA")
            })
            .map(|e| e.path)
            .collect()
    }

    /// Read config file contents (only small files)
    pub fn read_config_files(&mut self) -> HashMap<String, Option<String>> {
        let paths = self.find_config_files();
        let mut configs = HashMap::new();

        for path in paths {
            let content = self.read_file(&path).ok().and_then(|data| {
                // Only read files smaller than 10KB
                if data.len() < 10_000 {
                    String::from_utf8(data).ok()
                } else {
                    None
                }
            });
            configs.insert(path, content);
        }

        configs
    }
}

/// Parse manifest file content into key-value pairs
pub fn parse_manifest(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_key = String::new();
    let mut current_value = String::new();

    for line in content.lines() {
        // Continuation lines start with a space
        if line.starts_with(' ') && !current_key.is_empty() {
            current_value.push_str(line.trim_start());
            continue;
        }

        // Save previous entry
        if !current_key.is_empty() {
            map.insert(current_key, current_value);
        }

        // Parse new entry
        if let Some(colon_pos) = line.find(':') {
            current_key = line[..colon_pos].trim().to_string();
            current_value = line[colon_pos + 1..].trim().to_string();
        } else {
            current_key = String::new();
            current_value = String::new();
        }
    }

    // Don't forget the last entry
    if !current_key.is_empty() {
        map.insert(current_key, current_value);
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_jar() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());

        // Add a minimal class file
        let class_bytes = create_minimal_class("com/example/Test");
        let options = SimpleFileOptions::default();
        zip.start_file("com/example/Test.class", options).unwrap();
        zip.write_all(&class_bytes).unwrap();

        // Add manifest
        zip.start_file("META-INF/MANIFEST.MF", options).unwrap();
        zip.write_all(b"Manifest-Version: 1.0\nMain-Class: com.example.Main\n")
            .unwrap();

        // Add a config file
        zip.start_file("application.properties", options).unwrap();
        zip.write_all(b"app.name=test\n").unwrap();

        zip.finish().unwrap();
        file
    }

    fn create_minimal_class(name: &str) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Magic
        bytes.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        // Version (Java 17)
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&61u16.to_be_bytes());
        // Constant pool count (3)
        bytes.extend_from_slice(&3u16.to_be_bytes());

        // UTF8 entry with class name
        bytes.push(1);
        bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());

        // Class entry pointing to UTF8
        bytes.push(7);
        bytes.extend_from_slice(&1u16.to_be_bytes());

        // Access flags, this_class, super_class, interfaces_count
        bytes.extend_from_slice(&0x0001u16.to_be_bytes()); // public
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());

        bytes
    }

    #[test]
    fn test_open_jar() {
        let jar = create_test_jar();
        let reader = JarReader::open(jar.path()).unwrap();
        assert!(!reader.is_empty());
    }

    #[test]
    fn test_list_entries() {
        let jar = create_test_jar();
        let mut reader = JarReader::open(jar.path()).unwrap();
        let entries = reader.entries();

        assert!(entries.iter().any(|e| e.path == "com/example/Test.class"));
        assert!(entries.iter().any(|e| e.path == "META-INF/MANIFEST.MF"));
    }

    #[test]
    fn test_read_manifest() {
        let jar = create_test_jar();
        let mut reader = JarReader::open(jar.path()).unwrap();
        let manifest = reader.read_manifest().unwrap();

        assert!(manifest.contains("Main-Class"));
    }

    #[test]
    fn test_parse_classes() {
        let jar = create_test_jar();
        let mut reader = JarReader::open(jar.path()).unwrap();
        let classes = reader.parse_classes(None).unwrap();

        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "com.example.Test");
        assert_eq!(classes[0].package_name, "com.example");
    }

    #[test]
    fn test_parse_manifest() {
        let manifest = "Manifest-Version: 1.0\nMain-Class: com.example.Main\nBuild-Jdk: 17.0.1\n";
        let parsed = parse_manifest(manifest);

        assert_eq!(parsed.get("Main-Class"), Some(&"com.example.Main".to_string()));
        assert_eq!(parsed.get("Build-Jdk"), Some(&"17.0.1".to_string()));
    }

    #[test]
    fn test_find_config_files() {
        let jar = create_test_jar();
        let mut reader = JarReader::open(jar.path()).unwrap();
        let configs = reader.find_config_files();

        assert!(configs.iter().any(|p| p == "application.properties"));
    }
}
