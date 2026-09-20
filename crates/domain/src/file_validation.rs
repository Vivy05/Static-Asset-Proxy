//! Parsing and validation for files served from immutable static artifacts.
//! This module does not accept uploads or manage artifact versions.

use thiserror::Error;

pub const MAX_REQUEST_PATH_BYTES: usize = 2048;
pub const MAX_SEGMENT_BYTES: usize = 255;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPath(String);

impl AssetPath {
    /// Parses the URL path after the HTTP framework has separated its query.
    /// The returned path is relative to an artifact root and safe as an object key suffix.
    pub fn parse(request_path: &str) -> Result<Self, FileValidationError> {
        if !request_path.starts_with('/') {
            return Err(FileValidationError::InvalidPath);
        }
        if request_path.len() > MAX_REQUEST_PATH_BYTES * 3 {
            return Err(FileValidationError::PathTooLong);
        }

        let decoded = percent_decode(request_path)?;
        let path = std::str::from_utf8(&decoded).map_err(|_| FileValidationError::InvalidUtf8)?;
        if path.len() > MAX_REQUEST_PATH_BYTES {
            return Err(FileValidationError::PathTooLong);
        }
        if path
            .chars()
            .any(|ch| matches!(ch, '\\' | '%' | '?' | '#') || ch.is_control())
        {
            return Err(FileValidationError::InvalidPath);
        }

        if path.contains("//") {
            return Err(FileValidationError::InvalidPath);
        }
        let mut segments = Vec::new();
        for segment in path.split('/') {
            if segment.is_empty() {
                continue;
            }
            if segment == "." || segment == ".." || segment.as_bytes().len() > MAX_SEGMENT_BYTES {
                return Err(FileValidationError::InvalidPath);
            }
            segments.push(segment);
        }
        if segments.is_empty() || request_path.ends_with('/') {
            segments.push("index.html");
        }
        Ok(Self(segments.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub path: AssetPath,
    pub size_bytes: u64,
    pub content_type: String,
}

impl FileMetadata {
    /// Validates metadata returned by a trusted artifact reader before creating an HTTP response.
    pub fn new(
        path: AssetPath,
        size_bytes: u64,
        content_type: impl Into<String>,
    ) -> Result<Self, FileValidationError> {
        if size_bytes > MAX_FILE_BYTES {
            return Err(FileValidationError::FileTooLarge);
        }
        let content_type = content_type.into();
        if content_type.is_empty()
            || content_type.len() > 128
            || !content_type.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'+' | b'.')
            })
            || content_type.split('/').count() != 2
            || content_type.starts_with('/')
            || content_type.ends_with('/')
        {
            return Err(FileValidationError::InvalidContentType);
        }
        Ok(Self {
            path,
            size_bytes,
            content_type,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FileValidationError {
    #[error("invalid static file path")]
    InvalidPath,
    #[error("static file path is too long")]
    PathTooLong,
    #[error("invalid percent encoding")]
    InvalidPercentEncoding,
    #[error("static file path is not valid UTF-8")]
    InvalidUtf8,
    #[error("static file exceeds the configured size limit")]
    FileTooLarge,
    #[error("invalid content type")]
    InvalidContentType,
}

fn percent_decode(input: &str) -> Result<Vec<u8>, FileValidationError> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(FileValidationError::InvalidPercentEncoding);
            }
            let high =
                hex_value(bytes[index + 1]).ok_or(FileValidationError::InvalidPercentEncoding)?;
            let low =
                hex_value(bytes[index + 2]).ok_or(FileValidationError::InvalidPercentEncoding)?;
            let value = (high << 4) | low;
            // Encoded separators could change the path after a second decode or in a downstream reader.
            if matches!(value, b'/' | b'\\' | b'%') {
                return Err(FileValidationError::InvalidPath);
            }
            output.push(value);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    Ok(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetPath, FileMetadata, FileValidationError, MAX_FILE_BYTES};

    #[test]
    fn parses_root_nested_and_unicode_paths() {
        assert_eq!(AssetPath::parse("/").expect("root").as_str(), "index.html");
        assert_eq!(
            AssetPath::parse("/assets/app.js").expect("file").as_str(),
            "assets/app.js"
        );
        assert_eq!(
            AssetPath::parse("/docs/").expect("directory").as_str(),
            "docs/index.html"
        );
        assert_eq!(
            AssetPath::parse("/%E9%A1%B5%E9%9D%A2.html")
                .expect("unicode")
                .as_str(),
            "页面.html"
        );
    }

    #[test]
    fn rejects_traversal_and_ambiguous_encodings() {
        for path in [
            "/../secret",
            "/%2e%2e/secret",
            "/a/./b",
            "/a//b",
            "/a%2fb",
            "/a%5cb",
            "/%252e%252e/x",
            "/a\\b",
            "/a?x",
            "/a#x",
            "/a%00b",
            "/%ff",
            "/bad%2",
        ] {
            assert!(AssetPath::parse(path).is_err(), "accepted {path}");
        }
    }

    #[test]
    fn validates_file_metadata() {
        let path = AssetPath::parse("/assets/app.js").expect("path");
        assert!(FileMetadata::new(path.clone(), 0, "application/javascript").is_ok());
        assert_eq!(
            FileMetadata::new(path.clone(), MAX_FILE_BYTES + 1, "text/html").expect_err("size"),
            FileValidationError::FileTooLarge
        );
        assert_eq!(
            FileMetadata::new(path.clone(), 12, "text//html").expect_err("type"),
            FileValidationError::InvalidContentType
        );
        assert_eq!(
            FileMetadata::new(path, 12, "text/html\r\nX-Test: x").expect_err("header"),
            FileValidationError::InvalidContentType
        );
    }
}
