//! High-performance async server implementation with zero-cost abstractions
//!
//! This module provides a type-safe, high-performance HTTP server that uses
//! Rust's zero-cost abstractions and async/await for maximum efficiency.

use crate::{
    AppState, Result,
    cache::{Cache, CacheLoader, DefaultCacheLoader},
    config::Config,
    handlers::MainHandler,
};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::net::TcpListener;

/// High-performance HTTP server with zero-cost abstractions
pub struct Server<C: Cache> {
    state: Arc<AppState<C>>,
    handler: MainHandler<C>,
    metrics: ServerMetrics,
}

impl<C: Cache> Server<C> {
    /// Create a new server with the given state
    pub fn new(state: AppState<C>) -> Self {
        Self {
            state: Arc::new(state),
            handler: MainHandler::new(),
            metrics: ServerMetrics::new(),
        }
    }

    /// Create a server from configuration with default cache loader
    pub async fn from_config(config: Config) -> Result<Server<C>>
    where
        C: Default,
    {
        let cache = C::default();
        let loader = DefaultCacheLoader::new();

        loader.initialize_cache(&cache, &config).await;

        let state = AppState::new(config, cache);
        Ok(Self::new(state))
    }

    /// Create a server with custom cache and loader
    pub async fn with_cache_loader<L: CacheLoader<C>>(
        config: Config,
        cache: C,
        loader: L,
    ) -> Result<Self> {
        loader.initialize_cache(&cache, &config).await;

        let state = AppState::new(config, cache);
        Ok(Self::new(state))
    }

    /// Start the server and run indefinitely
    pub async fn serve(self) -> Result<()> {
        let addr = self.state.config.server_address();
        let listener = TcpListener::bind(&addr).await?;

        self.state.config.print_summary();
        println!("Gary API server running on {}", addr);

        self.serve_with_listener(listener).await
    }

    /// Serve with a custom TcpListener (useful for testing)
    pub async fn serve_with_listener(self, listener: TcpListener) -> Result<()> {
        let state = self.state;
        let handler = Arc::new(self.handler);
        let metrics = Arc::new(self.metrics);

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream); //why? i dont know
            let state = state.clone();
            let handler = handler.clone();
            let metrics = metrics.clone();

            tokio::task::spawn(async move {
                let start_time = Instant::now();
                let connection_metrics = metrics.clone();

                let service = service_fn(move |req| {
                    let state = state.clone();
                    let handler = handler.clone();
                    let metrics = metrics.clone();
                    let start = Instant::now();

                    async move {
                        // Update metrics
                        metrics.increment_requests();

                        // Handle the request
                        let response = handler.handle(req, &state).await;

                        // Update response time metrics
                        metrics.record_response_time(start.elapsed());

                        Ok::<_, hyper::Error>(response)
                    }
                });

                // Handle the connection and update metrics
                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    eprintln!("Error serving connection from {}: {}", remote_addr, err);
                } else {
                    // Update connection metrics
                    connection_metrics.record_connection_duration(start_time.elapsed());
                }
            });
        }
    }

    /// Get server metrics
    pub fn metrics(&self) -> &ServerMetrics {
        &self.metrics
    }

    /// Get server state
    pub fn state(&self) -> &Arc<AppState<C>> {
        &self.state
    }

    /// Gracefully shutdown the server (placeholder for future implementation)
    pub async fn shutdown(self) -> Result<()> {
        println!("Server shutting down...");
        Ok(())
    }
}

/// Server performance metrics; maybe usings for prometheus or similar in the future
#[derive(Debug)]
pub struct ServerMetrics {
    request_count: std::sync::atomic::AtomicU64,
    total_response_time: std::sync::atomic::AtomicU64,
    total_connection_time: std::sync::atomic::AtomicU64,
    connection_count: std::sync::atomic::AtomicU64,
    start_time: Instant,
}

impl ServerMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            request_count: std::sync::atomic::AtomicU64::new(0),
            total_response_time: std::sync::atomic::AtomicU64::new(0),
            total_connection_time: std::sync::atomic::AtomicU64::new(0),
            connection_count: std::sync::atomic::AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Increment request counter
    pub fn increment_requests(&self) {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record response time
    pub fn record_response_time(&self, duration: Duration) {
        self.total_response_time.fetch_add(
            duration.as_micros() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Record connection duration
    pub fn record_connection_duration(&self, duration: Duration) {
        self.connection_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_connection_time.fetch_add(
            duration.as_micros() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Get total request count
    pub fn request_count(&self) -> u64 {
        self.request_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get average response time in microseconds
    pub fn average_response_time(&self) -> f64 {
        let total_time = self
            .total_response_time
            .load(std::sync::atomic::Ordering::Relaxed);
        let count = self.request_count();

        if count > 0 {
            total_time as f64 / count as f64
        } else {
            0.0
        }
    }

    /// Get average connection duration in microseconds
    pub fn average_connection_duration(&self) -> f64 {
        let total_time = self
            .total_connection_time
            .load(std::sync::atomic::Ordering::Relaxed);
        let count = self
            .connection_count
            .load(std::sync::atomic::Ordering::Relaxed);

        if count > 0 {
            total_time as f64 / count as f64
        } else {
            0.0
        }
    }

    /// Get server uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get requests per second
    pub fn requests_per_second(&self) -> f64 {
        let uptime_secs = self.uptime().as_secs_f64();
        if uptime_secs > 0.0 {
            self.request_count() as f64 / uptime_secs
        } else {
            0.0
        }
    }

    /// Print metrics summary
    pub fn print_summary(&self) {
        println!("Server Metrics:");
        println!("  Uptime: {:?}", self.uptime());
        println!("  Total requests: {}", self.request_count());
        println!("  Requests per second: {:.2}", self.requests_per_second());
        println!(
            "  Average response time: {:.2} μs",
            self.average_response_time()
        );
        println!(
            "  Average connection duration: {:.2} μs",
            self.average_connection_duration()
        );
    }
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Server builder for fluent API
pub struct ServerBuilder<C: Cache> {
    config: Option<Config>,
    cache: Option<C>,
}

impl<C: Cache> ServerBuilder<C> {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: None,
            cache: None,
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Set cache
    pub fn with_cache(mut self, cache: C) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Build the server
    pub async fn build(self) -> Result<Server<C>>
    where
        C: Default,
    {
        let config = self.config.unwrap_or_default();
        let cache = self.cache.unwrap_or_default();

        let loader = DefaultCacheLoader::new();
        loader.initialize_cache(&cache, &config).await;

        let state = AppState::new(config, cache);
        Ok(Server::new(state))
    }

    /// Build the server with custom cache loader
    pub async fn build_with_loader<L: CacheLoader<C>>(self, loader: L) -> Result<Server<C>>
    where
        C: Default,
    {
        let config = self.config.unwrap_or_default();
        let cache = self.cache.unwrap_or_default();

        loader.initialize_cache(&cache, &config).await;

        let state = AppState::new(config, cache);
        Ok(Server::new(state))
    }
}

impl<C: Cache> Default for ServerBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenient type alias for the most common server configuration
pub type GaryServer = Server<crate::cache::FileCache>;

/// Quick server creation functions
impl GaryServer {
    /// Create and start a server with default configuration
    pub async fn run_with_defaults() -> Result<()> {
        let config = Config::from_env()?;
        config.validate()?;

        let server = Self::from_config(config).await?;
        server.serve().await
    }

    /// Create and start a server with custom configuration
    pub async fn run_with_config(config: Config) -> Result<()> {
        config.validate()?;

        let server = Self::from_config(config).await?;
        server.serve().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::FileCache, config::ConfigBuilder};
    use tokio::time::sleep;

    #[test]
    fn test_server_metrics() {
        let metrics = ServerMetrics::new();

        assert_eq!(metrics.request_count(), 0);
        assert_eq!(metrics.average_response_time(), 0.0);

        metrics.increment_requests();
        assert_eq!(metrics.request_count(), 1);

        metrics.record_response_time(Duration::from_micros(100));
        assert_eq!(metrics.average_response_time(), 100.0);
    }

    #[tokio::test]
    async fn test_server_builder() {
        let config = ConfigBuilder::new().port(0).build(); // Use port 0 for testing
        let cache = FileCache::new();

        let server = ServerBuilder::new()
            .with_config(config)
            .with_cache(cache)
            .build()
            .await
            .expect("Failed to build server");

        assert_eq!(server.state().config.port, 0);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = Config::default();
        let cache = FileCache::new();
        let state = AppState::new(config, cache);

        let server = Server::new(state);
        assert_eq!(server.metrics().request_count(), 0);
    }

    #[test]
    fn test_metrics_calculations() {
        let metrics = ServerMetrics::new();

        metrics.increment_requests();
        metrics.record_response_time(Duration::from_micros(100));
        metrics.increment_requests();
        metrics.record_response_time(Duration::from_micros(200));

        assert_eq!(metrics.request_count(), 2);
        assert_eq!(metrics.average_response_time(), 150.0);
    }

    #[tokio::test]
    async fn test_metrics_uptime() {
        let metrics = ServerMetrics::new();

        sleep(Duration::from_millis(10)).await;

        let uptime = metrics.uptime();
        assert!(uptime >= Duration::from_millis(10));
    }

    #[test]
    fn test_requests_per_second() {
        let metrics = ServerMetrics::new();

        assert_eq!(metrics.requests_per_second(), 0.0);

        metrics.increment_requests();
        metrics.increment_requests();

        let rps = metrics.requests_per_second();
        assert!(rps > 0.0);
    }
}
