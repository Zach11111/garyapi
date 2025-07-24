//! Type definitions and zero-cost abstractions
//!
//! This module provides strongly-typed wrappers around primitive types
//! to prevent mixing up different kinds of strings and provide compile-time guarantees.

use bytes::Bytes;
use std::fmt;
use std::ops::Deref;

/// Zero-cost wrapper for file names
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileName(String);

impl FileName {
    /// Create a new FileName, validating it doesn't contain path separators
    pub fn new(name: impl Into<String>) -> Result<Self, &'static str> {
        let name = name.into();
        if name.contains('/') || name.contains('\\') {
            Err("File name cannot contain path separators")
        } else if name.is_empty() {
            Err("File name cannot be empty")
        } else {
            Ok(Self(name))
        }
    }

    /// Create a FileName without validation (unsafe but zero-cost)
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        let s = self.0.as_str();
        match s.rfind('.') {
            Some(idx) if idx > 0 && idx < s.len() - 1 => Some(&s[idx + 1..]),
            _ => None,
        }
    }

    /// Get as bytes for efficient serialization
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Deref for FileName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for FileName {
    fn from(s: String) -> Self {
        Self::new_unchecked(s)
    }
}

impl AsRef<str> for FileName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Zero-cost wrapper for base URLs
#[derive(Debug, Clone)]
pub struct BaseUrl(Bytes);

impl BaseUrl {
    pub fn new(url: impl Into<Bytes>) -> Self {
        Self(url.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<String> for BaseUrl {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for BaseUrl {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

/// Zero-cost wrapper for cache keys
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

impl Deref for CacheKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<FileName> for CacheKey {
    fn from(filename: FileName) -> Self {
        Self(filename.0)
    }
}

impl From<String> for CacheKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// MIME content types with compile-time dispatch
#[derive(Debug, Clone, Copy)]
pub enum ContentType {
    ImageJpeg,
    ImagePng,
    ImageGif,
    ImageWebp,
    ApplicationJson,
    TextPlain,
    ApplicationOctetStream,
}

impl ContentType {
    /// Get content type from file extension with zero allocation
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Self::ImageJpeg,
            "png" => Self::ImagePng,
            "gif" => Self::ImageGif,
            "webp" => Self::ImageWebp,
            _ => Self::ApplicationOctetStream,
        }
    }

    /// Get content type from filename
    pub fn from_filename(filename: &FileName) -> Self {
        filename
            .extension()
            .map(Self::from_extension)
            .unwrap_or(Self::ApplicationOctetStream)
    }

    /// Get the MIME type string
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ImageJpeg => "image/jpeg",
            Self::ImagePng => "image/png",
            Self::ImageGif => "image/gif",
            Self::ImageWebp => "image/webp",
            Self::ApplicationJson => "application/json",
            Self::TextPlain => "text/plain",
            Self::ApplicationOctetStream => "application/octet-stream",
        }
    }
}

/// Resource types in the API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Gary,
    Goober,
}

impl ResourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gary => "gary",
            Self::Goober => "goober",
        }
    }

    pub const fn default_image(self) -> &'static str {
        match self {
            Self::Gary => "Gary76.jpg",
            Self::Goober => "goober8.jpg",
        }
    }
}

/// Content types that can be served
#[derive(Debug, Clone)]
pub enum Content {
    Quote(Bytes),
    Joke(Bytes),
    ImageUrl {
        resource: ResourceType,
        filename: FileName,
    },
    Image(Bytes),
}

/// Strongly typed directory path
#[derive(Debug, Clone)]
pub struct DirectoryPath(String);

impl DirectoryPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn join(&self, filename: &FileName) -> String {
        format!("{}/{}", self.0, filename.as_ref())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DirectoryPath {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for DirectoryPath {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

/// JSON response constants as compile-time byte slices
pub struct JsonConstants;

impl JsonConstants {
    pub const GARY_PREFIX: &'static [u8] = b"{\"url\":\"";
    pub const GARY_SUFFIX: &'static [u8] = b"/";
    pub const GARY_END: &'static [u8] = b"\"}";

    pub const GOOBER_PREFIX: &'static [u8] = b"{\"url\":\"";
    pub const GOOBER_SUFFIX: &'static [u8] = b"/";
    pub const GOOBER_END: &'static [u8] = b"\"}";

    pub const QUOTE_PREFIX: &'static [u8] = b"{\"quote\":\"";
    pub const QUOTE_END: &'static [u8] = b"\"}";

    pub const JOKE_PREFIX: &'static [u8] = b"{\"joke\":\"";
    pub const JOKE_END: &'static [u8] = b"\"}";

    pub const ERROR_PREFIX: &'static [u8] = b"{\"error\":\"";
    pub const ERROR_END: &'static [u8] = b"\"}";
}

/// HTTP response constants
pub struct HttpConstants;

impl HttpConstants {
    pub const NOT_FOUND: &'static [u8] = b"Not Found";
    pub const CACHE_CONTROL_NO_STORE: &'static str = "no-store";
    pub const HEADER_CONTENT_TYPE: &'static str = "content-type";
    pub const HEADER_CONTENT_LENGTH: &'static str = "content-length";
    pub const HEADER_CACHE_CONTROL: &'static str = "cache-control";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_validation() {
        assert!(FileName::new("test.jpg").is_ok());
        assert!(FileName::new("test/file.jpg").is_err());
        assert!(FileName::new("").is_err());
    }

    #[test]
    fn test_content_type_from_extension() {
        assert!(matches!(
            ContentType::from_extension("jpg"),
            ContentType::ImageJpeg
        ));
        assert!(matches!(
            ContentType::from_extension("png"),
            ContentType::ImagePng
        ));
        assert!(matches!(
            ContentType::from_extension("unknown"),
            ContentType::ApplicationOctetStream
        ));
    }

    #[test]
    fn test_filename_extension() {
        let filename = FileName::new_unchecked("test.jpg");
        assert_eq!(filename.extension(), Some("jpg"));

        let filename = FileName::new_unchecked("noextension");
        assert_eq!(filename.extension(), None);
    }
}
