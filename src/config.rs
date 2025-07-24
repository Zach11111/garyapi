//! Configuration management with zero-cost abstractions
//!
//! This module provides type-safe configuration loading from environment variables
//! with sensible defaults and validation.

use crate::types::{BaseUrl, DirectoryPath};
use std::env;

/// Application configuration with strongly typed fields
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory containing Gary images
    pub gary_dir: DirectoryPath,
    /// Directory containing Goober images
    pub goober_dir: DirectoryPath,
    /// Path to quotes JSON file
    pub quotes_file: String,
    /// Path to jokes JSON file
    pub jokes_file: String,
    /// Base URL for Gary images
    pub gary_base_url: BaseUrl,
    /// Base URL for Goober images
    pub goober_base_url: BaseUrl,
    /// Server port
    pub port: u16,
    /// Server bind address
    pub bind_address: String,
}

impl Config {
    /// Load configuration from environment variables with defaults
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok(); // Load .env file if present

        let gary_dir = env::var("GARY_DIR")
            .unwrap_or_else(|_| "gary_images".to_string())
            .into();

        let goober_dir = env::var("GOOBER_DIR")
            .unwrap_or_else(|_| "goober_images".to_string())
            .into();

        let quotes_file = env::var("QUOTES_FILE").unwrap_or_else(|_| "quotes.json".to_string());

        let jokes_file = env::var("JOKES_FILE").unwrap_or_else(|_| "jokes.json".to_string());

        let gary_base_url = env::var("GARYURL")
            .unwrap_or_else(|_| "http://localhost:8080/Gary".to_string())
            .into();

        let goober_base_url = env::var("GOOBERURL")
            .unwrap_or_else(|_| "http://localhost:8080/Goober".to_string())
            .into();

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .map_err(|e| ConfigError::InvalidPort(e))?;

        let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());

        Ok(Self {
            gary_dir,
            goober_dir,
            quotes_file,
            jokes_file,
            gary_base_url,
            goober_base_url,
            port,
            bind_address,
        })
    }

    /// Create a new configuration with explicit values
    pub fn new(
        gary_dir: impl Into<DirectoryPath>,
        goober_dir: impl Into<DirectoryPath>,
        quotes_file: impl Into<String>,
        jokes_file: impl Into<String>,
        gary_base_url: impl Into<BaseUrl>,
        goober_base_url: impl Into<BaseUrl>,
        port: u16,
        bind_address: impl Into<String>,
    ) -> Self {
        Self {
            gary_dir: gary_dir.into(),
            goober_dir: goober_dir.into(),
            quotes_file: quotes_file.into(),
            jokes_file: jokes_file.into(),
            gary_base_url: gary_base_url.into(),
            goober_base_url: goober_base_url.into(),
            port,
            bind_address: bind_address.into(),
        }
    }

    /// Get the full server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::InvalidConfiguration(
                "Port cannot be 0".to_string(),
            ));
        }

        if self.bind_address.is_empty() {
            return Err(ConfigError::InvalidConfiguration(
                "Bind address cannot be empty".to_string(),
            ));
        }

        if self.gary_dir.as_str().is_empty() {
            return Err(ConfigError::InvalidConfiguration(
                "Gary directory cannot be empty".to_string(),
            ));
        }

        if self.goober_dir.as_str().is_empty() {
            return Err(ConfigError::InvalidConfiguration(
                "Goober directory cannot be empty".to_string(),
            ));
        }

        if self.quotes_file.is_empty() {
            return Err(ConfigError::InvalidConfiguration(
                "Quotes file cannot be empty".to_string(),
            ));
        }

        if self.jokes_file.is_empty() {
            return Err(ConfigError::InvalidConfiguration(
                "Jokes file cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        println!("Configuration loaded:");
        println!("  Gary directory: {}", self.gary_dir.as_str());
        println!("  Goober directory: {}", self.goober_dir.as_str());
        println!("  Quotes file: {}", self.quotes_file);
        println!("  Jokes file: {}", self.jokes_file);
        println!("  Server address: {}", self.server_address());
        println!(
            "  Gary base URL: {}",
            String::from_utf8_lossy(self.gary_base_url.as_bytes())
        );
        println!(
            "  Goober base URL: {}",
            String::from_utf8_lossy(self.goober_base_url.as_bytes())
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(
            "gary_images",
            "goober_images",
            "quotes.json",
            "jokes.json",
            "http://localhost:8080/Gary",
            "http://localhost:8080/Goober",
            8080,
            "0.0.0.0",
        )
    }
}

/// Configuration loading errors
#[derive(Debug)]
pub enum ConfigError {
    /// Invalid port number
    InvalidPort(std::num::ParseIntError),
    /// Invalid configuration value
    InvalidConfiguration(String),
    /// Environment variable error
    EnvVar(env::VarError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPort(e) => write!(f, "Invalid port number: {}", e),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::EnvVar(e) => write!(f, "Environment variable error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPort(e) => Some(e),
            Self::EnvVar(e) => Some(e),
            _ => None,
        }
    }
}

impl From<env::VarError> for ConfigError {
    fn from(error: env::VarError) -> Self {
        Self::EnvVar(error)
    }
}

/// Configuration builder for fluent API
pub struct ConfigBuilder {
    gary_dir: Option<DirectoryPath>,
    goober_dir: Option<DirectoryPath>,
    quotes_file: Option<String>,
    jokes_file: Option<String>,
    gary_base_url: Option<BaseUrl>,
    goober_base_url: Option<BaseUrl>,
    port: Option<u16>,
    bind_address: Option<String>,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            gary_dir: None,
            goober_dir: None,
            quotes_file: None,
            jokes_file: None,
            gary_base_url: None,
            goober_base_url: None,
            port: None,
            bind_address: None,
        }
    }

    /// Set Gary directory
    pub fn gary_dir(mut self, dir: impl Into<DirectoryPath>) -> Self {
        self.gary_dir = Some(dir.into());
        self
    }

    /// Set Goober directory
    pub fn goober_dir(mut self, dir: impl Into<DirectoryPath>) -> Self {
        self.goober_dir = Some(dir.into());
        self
    }

    /// Set quotes file
    pub fn quotes_file(mut self, file: impl Into<String>) -> Self {
        self.quotes_file = Some(file.into());
        self
    }

    /// Set jokes file
    pub fn jokes_file(mut self, file: impl Into<String>) -> Self {
        self.jokes_file = Some(file.into());
        self
    }

    /// Set Gary base URL
    pub fn gary_base_url(mut self, url: impl Into<BaseUrl>) -> Self {
        self.gary_base_url = Some(url.into());
        self
    }

    /// Set Goober base URL
    pub fn goober_base_url(mut self, url: impl Into<BaseUrl>) -> Self {
        self.goober_base_url = Some(url.into());
        self
    }

    /// Set port
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set bind address
    pub fn bind_address(mut self, address: impl Into<String>) -> Self {
        self.bind_address = Some(address.into());
        self
    }

    /// Build the configuration with defaults for missing values
    pub fn build(self) -> Config {
        let default = Config::default();
        Config {
            gary_dir: self.gary_dir.unwrap_or(default.gary_dir),
            goober_dir: self.goober_dir.unwrap_or(default.goober_dir),
            quotes_file: self.quotes_file.unwrap_or(default.quotes_file),
            jokes_file: self.jokes_file.unwrap_or(default.jokes_file),
            gary_base_url: self.gary_base_url.unwrap_or(default.gary_base_url),
            goober_base_url: self.goober_base_url.unwrap_or(default.goober_base_url),
            port: self.port.unwrap_or(default.port),
            bind_address: self.bind_address.unwrap_or(default.bind_address),
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.gary_dir.as_str(), "gary_images");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .port(9090)
            .gary_dir("custom_gary")
            .build();

        assert_eq!(config.port, 9090);
        assert_eq!(config.gary_dir.as_str(), "custom_gary");
        assert_eq!(config.goober_dir.as_str(), "goober_images"); // Default
    }

    #[test]
    fn test_server_address() {
        let config = Config::default();
        assert_eq!(config.server_address(), "0.0.0.0:8080");
    }
}
