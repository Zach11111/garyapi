//! GaryAPI RS - gary api rust edition
//!
//! This library provides a fast, type-safe server for serving images, quotes, and jokes for gary and goober

pub mod cache;
pub mod config;
pub mod handlers;
pub mod responses;
pub mod routing;
pub mod server;
pub mod types;

// Re-export commonly used types for convenience
pub use cache::{Cache, FileCache};
pub use config::Config;
pub use handlers::RequestHandler;
pub use responses::{ImageResponse, JsonResponse, ResponseBuilder};
pub use routing::Route;
pub use server::Server;
pub use types::{BaseUrl, CacheKey, ContentType, FileName};

/// Main application state containing all shared data
#[derive(Clone)]
pub struct AppState<C: Cache> {
    pub config: Config,
    pub cache: C,
}

impl<C: Cache> AppState<C> {
    /// Create new application state with the given config and cache
    pub fn new(config: Config, cache: C) -> Self {
        Self { config, cache }
    }
}

/// Result type used throughout the application
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Error types for the application
#[derive(Debug)]
pub enum GaryError {
    NotFound,
    InvalidRoute,
    CacheError,
    FileError(std::io::Error),
    ConfigError(String),
}

impl std::fmt::Display for GaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GaryError::NotFound => write!(f, "Resource not found"),
            GaryError::InvalidRoute => write!(f, "Invalid route"),
            GaryError::CacheError => write!(f, "Cache operation failed"),
            GaryError::FileError(e) => write!(f, "File operation failed: {}", e),
            GaryError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for GaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GaryError::FileError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GaryError {
    fn from(error: std::io::Error) -> Self {
        GaryError::FileError(error)
    }
}
