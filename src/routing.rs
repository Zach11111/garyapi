//! Type-safe routing system with zero-cost abstractions
//!
//! This module provides compile-time route parsing and matching using enums
//! and pattern matching instead of runtime string comparisons.

use crate::types::{FileName, ResourceType};

/// All possible routes in the API with compile-time dispatch
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// GET / - Returns embedded documentation
    Docs,
    /// GET /gary/count - Returns file count for Gary
    GaryCount,
    /// GET /goober/count - Returns file count for Goober
    GooberCount,
    /// GET /gary - Returns random Gary image URL
    GaryUrl,
    /// GET /goober - Returns random Goober image URL
    GooberUrl,
    /// GET /quote - Returns random quote
    Quote,
    /// GET /joke - Returns random joke
    Joke,
    /// GET /gary/image/* - Returns random Gary image
    GaryImage,
    /// GET /goober/image/* - Returns random Goober image
    GooberImage,
    /// GET /Gary/{filename} - Returns specific Gary image
    GaryFile(FileName),
    /// GET /Goober/{filename} - Returns specific Goober image
    GooberFile(FileName),
    /// Invalid or unknown route
    NotFound,
}

impl Route {
    /// Parse a path into a Route with zero allocations for common paths
    pub fn from_path(path: &str) -> Self {
        match path {
            "/" => Self::Docs,
            "/gary/count" => Self::GaryCount,
            "/goober/count" => Self::GooberCount,
            "/gary" => Self::GaryUrl,
            "/goober" => Self::GooberUrl,
            "/quote" => Self::Quote,
            "/joke" => Self::Joke,
            p if p.starts_with("/gary/image/") => Self::GaryImage,
            p if p.starts_with("/goober/image/") => Self::GooberImage,
            p if p.starts_with("/Gary/") => {
                let filename = &p[6..]; // Skip "/Gary/"
                if filename.is_empty() {
                    Self::NotFound
                } else {
                    match FileName::new(filename) {
                        Ok(f) => Self::GaryFile(f),
                        Err(_) => Self::NotFound,
                    }
                }
            }
            p if p.starts_with("/Goober/") => {
                let filename = &p[8..]; // Skip "/Goober/"
                if filename.is_empty() {
                    Self::NotFound
                } else {
                    match FileName::new(filename) {
                        Ok(f) => Self::GooberFile(f),
                        Err(_) => Self::NotFound,
                    }
                }
            }
            _ => Self::NotFound,
        }
    }

    /// Get the resource type for file-based routes
    pub const fn resource_type(&self) -> Option<ResourceType> {
        match self {
            Self::GaryUrl | Self::GaryImage | Self::GaryFile(_) => Some(ResourceType::Gary),
            Self::GooberUrl | Self::GooberImage | Self::GooberFile(_) => Some(ResourceType::Goober),
            _ => None,
        }
    }

    /// Check if this route serves an image
    pub const fn is_image_route(&self) -> bool {
        matches!(
            self,
            Self::GaryImage | Self::GooberImage | Self::GaryFile(_) | Self::GooberFile(_)
        )
    }

    /// Check if this route returns a URL
    pub const fn is_url_route(&self) -> bool {
        matches!(self, Self::GaryUrl | Self::GooberUrl)
    }

    /// Check if this route returns text content
    pub const fn is_text_route(&self) -> bool {
        matches!(self, Self::Quote | Self::Joke)
    }

    /// Get the specific filename for file routes
    pub fn filename(&self) -> Option<&FileName> {
        match self {
            Self::GaryFile(f) | Self::GooberFile(f) => Some(f),
            _ => None,
        }
    }
}

/// Route matcher for efficient path parsing
pub struct RouteMatcher;

impl RouteMatcher {
    /// Create a new route matcher
    pub const fn new() -> Self {
        Self
    }

    /// Match a path to a route with compile-time optimization
    #[inline]
    pub fn match_route(&self, path: &str) -> Route {
        Route::from_path(path)
    }

    /// Pre-validate a path before parsing (optional optimization)
    pub fn is_valid_path(&self, path: &str) -> bool {
        !path.is_empty() && path.starts_with('/') && path.len() < 1000 // Reasonable limit
    }
}

impl Default for RouteMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Route patterns for compile-time matching
pub struct RoutePatterns;

impl RoutePatterns {
    pub const GARY: &'static str = "/gary";
    pub const GOOBER: &'static str = "/goober";
    pub const QUOTE: &'static str = "/quote";
    pub const JOKE: &'static str = "/joke";
    pub const GARY_IMAGE_PREFIX: &'static str = "/gary/image/";
    pub const GOOBER_IMAGE_PREFIX: &'static str = "/goober/image/";
    pub const GARY_FILE_PREFIX: &'static str = "/Gary/";
    pub const GOOBER_FILE_PREFIX: &'static str = "/Goober/";
}

/// Route handler trait for compile-time dispatch
pub trait RouteHandler<T> {
    /// Handle the given route and return the appropriate response
    fn handle(&self, route: Route) -> impl std::future::Future<Output = T> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_parsing() {
        assert_eq!(Route::from_path("/gary"), Route::GaryUrl);
        assert_eq!(Route::from_path("/goober"), Route::GooberUrl);
        assert_eq!(Route::from_path("/quote"), Route::Quote);
        assert_eq!(Route::from_path("/joke"), Route::Joke);
        assert_eq!(Route::from_path("/gary/image/random"), Route::GaryImage);
        assert_eq!(Route::from_path("/goober/image/random"), Route::GooberImage);
        assert_eq!(Route::from_path("/invalid"), Route::NotFound);
    }

    #[test]
    fn test_file_routes() {
        match Route::from_path("/Gary/test.jpg") {
            Route::GaryFile(filename) => {
                assert_eq!(filename.as_ref(), "test.jpg");
            }
            _ => panic!("Expected GaryFile route"),
        }

        match Route::from_path("/Goober/test.png") {
            Route::GooberFile(filename) => {
                assert_eq!(filename.as_ref(), "test.png");
            }
            _ => panic!("Expected GooberFile route"),
        }
    }

    #[test]
    fn test_route_properties() {
        assert_eq!(Route::GaryUrl.resource_type(), Some(ResourceType::Gary));
        assert_eq!(Route::GooberUrl.resource_type(), Some(ResourceType::Goober));
        assert_eq!(Route::Quote.resource_type(), None);

        assert!(Route::GaryImage.is_image_route());
        assert!(Route::GaryUrl.is_url_route());
        assert!(Route::Quote.is_text_route());
    }

    #[test]
    fn test_route_matcher() {
        let matcher = RouteMatcher::new();
        assert!(matcher.is_valid_path("/gary"));
        assert!(!matcher.is_valid_path(""));
        assert!(!matcher.is_valid_path("gary"));

        assert_eq!(matcher.match_route("/gary"), Route::GaryUrl);
    }

    #[test]
    fn test_invalid_filenames() {
        assert_eq!(Route::from_path("/Gary/"), Route::NotFound);
        assert_eq!(Route::from_path("/Gary/file/with/slashes"), Route::NotFound);
        assert_eq!(Route::from_path("/Goober/"), Route::NotFound);
    }
}
