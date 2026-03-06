//! Java class file parser
//!
//! Parses Java .class files according to JVMS Chapter 4 specification.
//! Uses zero-copy parsing where possible for maximum performance.
//!
//! # Example
//!
//! ```ignore
//! let data = std::fs::read("MyClass.class")?;
//! let class_file = parse_class_file(&data)?;
//! println!("Class: {} (Java {})", class_file.this_class_name, class_file.java_version);
//! ```

use std::io::{Cursor, Read};
use thiserror::Error;

/// Magic number for Java class files: 0xCAFEBABE
pub const CLASS_MAGIC: u32 = 0xCAFEBABE;

/// Bytecode version to Java version mapping
const BYTECODE_VERSION_MAP: &[(u16, &str)] = &[
    (45, "1.1"),
    (46, "1.2"),
    (47, "1.3"),
    (48, "1.4"),
    (49, "5.0"),
    (50, "6.0"),
    (51, "7.0"),
    (52, "8.0"),
    (53, "9.0"),
    (54, "10.0"),
    (55, "11.0"),
    (56, "12.0"),
    (57, "13.0"),
    (58, "14.0"),
    (59, "15.0"),
    (60, "16.0"),
    (61, "17.0"),
    (62, "18.0"),
    (63, "19.0"),
    (64, "20.0"),
    (65, "21.0"),
    (66, "22.0"),
    (67, "23.0"),
    (68, "24.0"),
];

/// Errors that can occur during class file parsing
#[derive(Error, Debug)]
pub enum ClassFileError {
    #[error("Invalid magic number: expected 0xCAFEBABE, got 0x{0:08X}")]
    InvalidMagic(u32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid constant pool tag: {0}")]
    InvalidConstantPoolTag(u8),

    #[error("Invalid constant pool index: {0}")]
    InvalidConstantPoolIndex(u16),

    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Truncated class file")]
    Truncated,
}

/// Java class file access flags
#[derive(Debug, Clone, Copy, Default)]
pub struct AccessFlags(pub u16);

impl AccessFlags {
    pub const ACC_PUBLIC: u16 = 0x0001;
    pub const ACC_FINAL: u16 = 0x0010;
    pub const ACC_SUPER: u16 = 0x0020;
    pub const ACC_INTERFACE: u16 = 0x0200;
    pub const ACC_ABSTRACT: u16 = 0x0400;
    pub const ACC_SYNTHETIC: u16 = 0x1000;
    pub const ACC_ANNOTATION: u16 = 0x2000;
    pub const ACC_ENUM: u16 = 0x4000;
    pub const ACC_MODULE: u16 = 0x8000;

    pub fn is_public(&self) -> bool {
        self.0 & Self::ACC_PUBLIC != 0
    }

    pub fn is_final(&self) -> bool {
        self.0 & Self::ACC_FINAL != 0
    }

    pub fn is_interface(&self) -> bool {
        self.0 & Self::ACC_INTERFACE != 0
    }

    pub fn is_abstract(&self) -> bool {
        self.0 & Self::ACC_ABSTRACT != 0
    }

    pub fn is_annotation(&self) -> bool {
        self.0 & Self::ACC_ANNOTATION != 0
    }

    pub fn is_enum(&self) -> bool {
        self.0 & Self::ACC_ENUM != 0
    }

    /// Get modifier strings
    pub fn to_modifiers(&self) -> Vec<String> {
        let mut modifiers = Vec::new();
        if self.is_public() {
            modifiers.push("public".to_string());
        }
        if self.is_final() {
            modifiers.push("final".to_string());
        }
        if self.is_abstract() && !self.is_interface() {
            modifiers.push("abstract".to_string());
        }
        modifiers
    }
}

/// Class type (class, interface, enum, or annotation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassType {
    Class,
    Interface,
    Enum,
    Annotation,
}

impl ClassType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClassType::Class => "class",
            ClassType::Interface => "interface",
            ClassType::Enum => "enum",
            ClassType::Annotation => "annotation",
        }
    }
}

/// Constant pool entry (simplified for class name extraction)
#[derive(Debug, Clone)]
pub enum ConstantPoolEntry {
    /// Placeholder for index 0 (unused)
    Empty,
    /// UTF-8 string
    Utf8(String),
    /// Class reference (index to UTF-8)
    Class(u16),
    /// Other constant types (we skip their content)
    Other,
    /// Long/Double take two slots
    LongOrDouble,
}

/// Parsed Java class file
#[derive(Debug, Clone)]
pub struct ClassFile {
    pub magic: u32,
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: Vec<ConstantPoolEntry>,
    pub access_flags: AccessFlags,
    pub this_class: u16,
    pub super_class: u16,
    pub interfaces_count: u16,
    /// The resolved class name (from constant pool)
    pub this_class_name: String,
    /// Java version string (e.g., "17.0")
    pub java_version: String,
}

impl ClassFile {
    /// Get the class type from access flags
    pub fn class_type(&self) -> ClassType {
        if self.access_flags.is_annotation() {
            ClassType::Annotation
        } else if self.access_flags.is_enum() {
            ClassType::Enum
        } else if self.access_flags.is_interface() {
            ClassType::Interface
        } else {
            ClassType::Class
        }
    }
}

/// High-level class information (for API consumers)
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Fully qualified class name (e.g., "com.example.MyClass")
    pub name: String,
    /// Package name (e.g., "com.example")
    pub package_name: String,
    /// Simple class name (e.g., "MyClass")
    pub simple_name: String,
    /// Class type
    pub class_type: ClassType,
    /// Access modifiers
    pub modifiers: Vec<String>,
    /// Bytecode major version
    pub bytecode_version: u16,
    /// Java version string
    pub java_version: String,
}

impl From<ClassFile> for ClassInfo {
    fn from(cf: ClassFile) -> Self {
        let name = cf.this_class_name.replace('/', ".");
        let last_dot = name.rfind('.');
        let (package_name, simple_name) = match last_dot {
            Some(idx) => (name[..idx].to_string(), name[idx + 1..].to_string()),
            None => (String::new(), name.clone()),
        };

        Self {
            name,
            package_name,
            simple_name,
            class_type: cf.class_type(),
            modifiers: cf.access_flags.to_modifiers(),
            bytecode_version: cf.major_version,
            java_version: cf.java_version,
        }
    }
}

/// Parse a Java class file from bytes
pub fn parse_class_file(data: &[u8]) -> Result<ClassFile, ClassFileError> {
    let mut cursor = Cursor::new(data);

    // Read magic number (4 bytes, big-endian)
    let magic = read_u32(&mut cursor)?;
    if magic != CLASS_MAGIC {
        return Err(ClassFileError::InvalidMagic(magic));
    }

    // Read version
    let minor_version = read_u16(&mut cursor)?;
    let major_version = read_u16(&mut cursor)?;

    // Map major version to Java version string
    let java_version = BYTECODE_VERSION_MAP
        .iter()
        .find(|(v, _)| *v == major_version)
        .map(|(_, s)| s.to_string())
        .unwrap_or_else(|| format!("{}.0", major_version));

    // Read constant pool
    let constant_pool_count = read_u16(&mut cursor)?;
    let mut constant_pool = Vec::with_capacity(constant_pool_count as usize);
    constant_pool.push(ConstantPoolEntry::Empty); // Index 0 is unused

    let mut i = 1u16;
    while i < constant_pool_count {
        let tag = read_u8(&mut cursor)?;
        let entry = parse_constant_pool_entry(&mut cursor, tag)?;

        // Long and Double take two slots
        let takes_two_slots = matches!(entry, ConstantPoolEntry::LongOrDouble);
        constant_pool.push(entry);

        if takes_two_slots {
            constant_pool.push(ConstantPoolEntry::Empty);
            i += 2;
        } else {
            i += 1;
        }
    }

    // Read access flags
    let access_flags = AccessFlags(read_u16(&mut cursor)?);

    // Read this_class and super_class indices
    let this_class = read_u16(&mut cursor)?;
    let super_class = read_u16(&mut cursor)?;

    // Read interfaces count
    let interfaces_count = read_u16(&mut cursor)?;

    // Resolve class name from constant pool
    let this_class_name = resolve_class_name(&constant_pool, this_class)?;

    Ok(ClassFile {
        magic,
        minor_version,
        major_version,
        constant_pool,
        access_flags,
        this_class,
        super_class,
        interfaces_count,
        this_class_name,
        java_version,
    })
}

/// Parse a single constant pool entry
fn parse_constant_pool_entry(
    cursor: &mut Cursor<&[u8]>,
    tag: u8,
) -> Result<ConstantPoolEntry, ClassFileError> {
    match tag {
        1 => {
            // CONSTANT_Utf8
            let length = read_u16(cursor)?;
            let mut bytes = vec![0u8; length as usize];
            cursor.read_exact(&mut bytes)?;
            // Modified UTF-8, but ASCII subset is compatible
            let string = String::from_utf8(bytes)?;
            Ok(ConstantPoolEntry::Utf8(string))
        }
        7 => {
            // CONSTANT_Class
            let name_index = read_u16(cursor)?;
            Ok(ConstantPoolEntry::Class(name_index))
        }
        8 => {
            // CONSTANT_String
            skip_bytes(cursor, 2)?;
            Ok(ConstantPoolEntry::Other)
        }
        3 | 4 => {
            // CONSTANT_Integer, CONSTANT_Float
            skip_bytes(cursor, 4)?;
            Ok(ConstantPoolEntry::Other)
        }
        5 | 6 => {
            // CONSTANT_Long, CONSTANT_Double (8 bytes, takes 2 slots)
            skip_bytes(cursor, 8)?;
            Ok(ConstantPoolEntry::LongOrDouble)
        }
        9 | 10 | 11 | 12 => {
            // CONSTANT_Fieldref, CONSTANT_Methodref, CONSTANT_InterfaceMethodref, CONSTANT_NameAndType
            skip_bytes(cursor, 4)?;
            Ok(ConstantPoolEntry::Other)
        }
        15 => {
            // CONSTANT_MethodHandle
            skip_bytes(cursor, 3)?;
            Ok(ConstantPoolEntry::Other)
        }
        16 => {
            // CONSTANT_MethodType
            skip_bytes(cursor, 2)?;
            Ok(ConstantPoolEntry::Other)
        }
        17 | 18 => {
            // CONSTANT_Dynamic, CONSTANT_InvokeDynamic
            skip_bytes(cursor, 4)?;
            Ok(ConstantPoolEntry::Other)
        }
        19 | 20 => {
            // CONSTANT_Module, CONSTANT_Package
            skip_bytes(cursor, 2)?;
            Ok(ConstantPoolEntry::Other)
        }
        _ => Err(ClassFileError::InvalidConstantPoolTag(tag)),
    }
}

/// Resolve a class name from constant pool indices
fn resolve_class_name(
    constant_pool: &[ConstantPoolEntry],
    class_index: u16,
) -> Result<String, ClassFileError> {
    let entry = constant_pool
        .get(class_index as usize)
        .ok_or(ClassFileError::InvalidConstantPoolIndex(class_index))?;

    match entry {
        ConstantPoolEntry::Class(name_index) => {
            let name_entry = constant_pool
                .get(*name_index as usize)
                .ok_or(ClassFileError::InvalidConstantPoolIndex(*name_index))?;
            match name_entry {
                ConstantPoolEntry::Utf8(s) => Ok(s.clone()),
                _ => Err(ClassFileError::InvalidConstantPoolIndex(*name_index)),
            }
        }
        _ => Err(ClassFileError::InvalidConstantPoolIndex(class_index)),
    }
}

// Helper functions for reading big-endian values

fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, ClassFileError> {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, ClassFileError> {
    let mut buf = [0u8; 2];
    cursor.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, ClassFileError> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn skip_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<(), ClassFileError> {
    let mut buf = vec![0u8; n];
    cursor.read_exact(&mut buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid class file bytes for testing
    // This represents an empty class "Test" in default package
    fn minimal_class_file() -> Vec<u8> {
        let mut bytes = Vec::new();

        // Magic
        bytes.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        // Minor version
        bytes.extend_from_slice(&0u16.to_be_bytes());
        // Major version (Java 17 = 61)
        bytes.extend_from_slice(&61u16.to_be_bytes());
        // Constant pool count (3: index 0 unused, 1=UTF8, 2=Class)
        bytes.extend_from_slice(&3u16.to_be_bytes());

        // Constant pool entry 1: UTF8 "Test"
        bytes.push(1); // tag
        bytes.extend_from_slice(&4u16.to_be_bytes()); // length
        bytes.extend_from_slice(b"Test");

        // Constant pool entry 2: Class pointing to entry 1
        bytes.push(7); // tag
        bytes.extend_from_slice(&1u16.to_be_bytes()); // name_index

        // Access flags (public)
        bytes.extend_from_slice(&0x0001u16.to_be_bytes());
        // this_class (index 2)
        bytes.extend_from_slice(&2u16.to_be_bytes());
        // super_class (0 = none)
        bytes.extend_from_slice(&0u16.to_be_bytes());
        // interfaces_count
        bytes.extend_from_slice(&0u16.to_be_bytes());

        bytes
    }

    #[test]
    fn test_parse_minimal_class() {
        let data = minimal_class_file();
        let cf = parse_class_file(&data).expect("should parse");

        assert_eq!(cf.magic, CLASS_MAGIC);
        assert_eq!(cf.major_version, 61);
        assert_eq!(cf.java_version, "17.0");
        assert_eq!(cf.this_class_name, "Test");
        assert!(cf.access_flags.is_public());
        assert_eq!(cf.class_type(), ClassType::Class);
    }

    #[test]
    fn test_invalid_magic() {
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let result = parse_class_file(&data);
        assert!(matches!(result, Err(ClassFileError::InvalidMagic(_))));
    }

    #[test]
    fn test_class_info_conversion() {
        // Create a class file with "com/example/MyClass" name
        let mut bytes = Vec::new();

        // Magic
        bytes.extend_from_slice(&0xCAFEBABEu32.to_be_bytes());
        // Minor version
        bytes.extend_from_slice(&0u16.to_be_bytes());
        // Major version (Java 17 = 61)
        bytes.extend_from_slice(&61u16.to_be_bytes());
        // Constant pool count (3: index 0 unused, 1=UTF8, 2=Class)
        bytes.extend_from_slice(&3u16.to_be_bytes());

        // Constant pool entry 1: UTF8 "com/example/MyClass"
        let class_name = b"com/example/MyClass";
        bytes.push(1); // tag
        bytes.extend_from_slice(&(class_name.len() as u16).to_be_bytes()); // length
        bytes.extend_from_slice(class_name);

        // Constant pool entry 2: Class pointing to entry 1
        bytes.push(7); // tag
        bytes.extend_from_slice(&1u16.to_be_bytes()); // name_index

        // Access flags (public)
        bytes.extend_from_slice(&0x0001u16.to_be_bytes());
        // this_class (index 2)
        bytes.extend_from_slice(&2u16.to_be_bytes());
        // super_class (0 = none)
        bytes.extend_from_slice(&0u16.to_be_bytes());
        // interfaces_count
        bytes.extend_from_slice(&0u16.to_be_bytes());

        let cf = parse_class_file(&bytes).expect("should parse");
        let info: ClassInfo = cf.into();

        assert_eq!(info.name, "com.example.MyClass");
        assert_eq!(info.package_name, "com.example");
        assert_eq!(info.simple_name, "MyClass");
    }

    #[test]
    fn test_access_flags() {
        let flags = AccessFlags(0x2201); // public + interface + annotation
        assert!(flags.is_public());
        assert!(flags.is_interface());
        assert!(flags.is_annotation());
        assert!(!flags.is_enum());
    }

    #[test]
    fn test_class_type_detection() {
        // Annotation
        let flags = AccessFlags(0x2200);
        assert!(flags.is_annotation());

        // Enum
        let flags = AccessFlags(0x4000);
        assert!(flags.is_enum());

        // Interface
        let flags = AccessFlags(0x0200);
        assert!(flags.is_interface());
    }
}
