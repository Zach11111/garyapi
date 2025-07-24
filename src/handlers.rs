//! Request handlers with trait-based abstractions and zero-cost dispatch
//!
//! This module provides type-safe request handlers that use compile-time dispatch
//! and zero-cost abstractions to efficiently handle different types of requests.

use crate::{
    AppState, GaryError,
    cache::Cache,
    responses::{DefaultResponser, Responser, fast},
    routing::Route,
    types::{FileName, ResourceType},
};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response, body::Incoming};
use std::marker::PhantomData;

/// Request handler trait for zero-cost abstraction over different handler types
pub trait RequestHandler<C: Cache>: Send + Sync + 'static {
    /// Check if this handler can handle the given route
    fn can_handle(&self, route: &Route) -> bool;
}

/// Async request handling trait (separate to maintain object safety)
#[async_trait::async_trait]
pub trait AsyncRequestHandler<C: Cache>: RequestHandler<C> {
    /// Handle an HTTP request and return a response
    async fn handle_request(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError>;
}

/// Main request dispatcher that routes requests to appropriate handlers
pub struct RequestDispatcher<C: Cache> {
    _phantom: PhantomData<C>,
}

impl<C: Cache> RequestDispatcher<C> {
    /// Create a new request dispatcher
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Dispatch a request to the appropriate handler
    pub async fn dispatch(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Response<Full<Bytes>> {
        if req.method() != Method::GET {
            return DefaultResponser::not_found_response();
        }

        let route = Route::from_path(req.uri().path());

        match self.handle_route(route, state).await {
            Ok(response) => response,
            Err(_) => DefaultResponser::not_found_response(),
        }
    }

    /// Handle a specific route with compile-time dispatch
    async fn handle_route(
        &self,
        route: Route,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        match route {
            Route::Docs => {
                // Serve embedded docs.html
                const DOCS_HTML: &str = include_str!("docs.html");
                Ok(Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header("content-type", "text/html; charset=utf-8")
                    .body(Full::new(Bytes::from_static(DOCS_HTML.as_bytes())))
                    .unwrap())
            }
            Route::GaryCount => {
                let count = state.cache.file_count(ResourceType::Gary);
                let json = format!("{{\"count\":{}}}", count);
                Ok(Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(json)))
                    .unwrap())
            }
            Route::GooberCount => {
                let count = state.cache.file_count(ResourceType::Goober);
                let json = format!("{{\"count\":{}}}", count);
                Ok(Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(json)))
                    .unwrap())
            }
            Route::GaryUrl => self.handle_url_route(ResourceType::Gary, state).await,
            Route::GooberUrl => self.handle_url_route(ResourceType::Goober, state).await,
            Route::Quote => self.handle_quote_route(state).await,
            Route::Joke => self.handle_joke_route(state).await,
            Route::GaryImage => {
                self.handle_random_image_route(ResourceType::Gary, state)
                    .await
            }
            Route::GooberImage => {
                self.handle_random_image_route(ResourceType::Goober, state)
                    .await
            }
            Route::GaryFile(filename) => {
                self.handle_file_route(ResourceType::Gary, filename, state)
                    .await
            }
            Route::GooberFile(filename) => {
                self.handle_file_route(ResourceType::Goober, filename, state)
                    .await
            }
            Route::NotFound => Err(GaryError::NotFound),
        }
    }

    /// Handle URL routes (returns JSON with image URL)
    async fn handle_url_route(
        &self,
        resource: ResourceType,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        let filename = state
            .cache
            .get_random_file(resource)
            .unwrap_or_else(|| FileName::new_unchecked(resource.default_image().to_string()));

        let base_url = match resource {
            ResourceType::Gary => &state.config.gary_base_url,
            ResourceType::Goober => &state.config.goober_base_url,
        };

        Ok(fast::gary_url(base_url, &filename))
    }

    /// Handle quote routes
    async fn handle_quote_route(
        &self,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        match state.cache.get_random_quote() {
            Some(quote) => Ok(fast::quote(&quote)),
            None => Ok(fast::error(b"No quotes available")),
        }
    }

    /// Handle joke routes
    async fn handle_joke_route(
        &self,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        match state.cache.get_random_joke() {
            Some(joke) => Ok(fast::joke(&joke)),
            None => Ok(fast::error(b"No jokes available")),
        }
    }

    /// Handle random image routes
    async fn handle_random_image_route(
        &self,
        resource: ResourceType,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        let filename = state
            .cache
            .get_random_file(resource)
            .unwrap_or_else(|| FileName::new_unchecked(resource.default_image().to_string()));

        self.serve_image_file(resource, &filename, state).await
    }

    /// Handle specific file routes
    async fn handle_file_route(
        &self,
        resource: ResourceType,
        filename: FileName,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        self.serve_image_file(resource, &filename, state).await
    }

    /// Serve an image file with caching
    async fn serve_image_file(
        &self,
        resource: ResourceType,
        filename: &FileName,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        let cache_key = filename.clone().into();

        if let Some(content) = state.cache.get_image(&cache_key) {
            return Ok(fast::image(content, filename));
        }

        let dir = match resource {
            ResourceType::Gary => &state.config.gary_dir,
            ResourceType::Goober => &state.config.goober_dir,
        };

        let file_path = dir.join(filename);
        let read_result = tokio::fs::read(&file_path).await;
        match read_result {
            Ok(content) => {
                let bytes = Bytes::from(content);
                if bytes.len() < 1024 * 1024 {
                    state.cache.store_image(cache_key, bytes.clone());
                }
                Ok(fast::image(bytes, filename))
            }
            Err(e) => Err(GaryError::FileError(e)),
        }
    }
}

impl<C: Cache> Default for RequestDispatcher<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Specialized handler for URL routes
pub struct UrlHandler<C: Cache> {
    _phantom: PhantomData<C>,
}

impl<C: Cache> UrlHandler<C> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C: Cache> RequestHandler<C> for UrlHandler<C> {
    fn can_handle(&self, route: &Route) -> bool {
        route.is_url_route()
    }

}

#[async_trait::async_trait]
impl<C: Cache> AsyncRequestHandler<C> for UrlHandler<C> {
    async fn handle_request(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        let route = Route::from_path(req.uri().path());
        let dispatcher = RequestDispatcher::new();

        match route {
            Route::GaryUrl | Route::GooberUrl => dispatcher.handle_route(route, state).await,
            _ => Err(GaryError::InvalidRoute),
        }
    }
}

/// Specialized handler for image routes
pub struct ImageHandler<C: Cache> {
    _phantom: PhantomData<C>,
}

impl<C: Cache> ImageHandler<C> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C: Cache> RequestHandler<C> for ImageHandler<C> {
    fn can_handle(&self, route: &Route) -> bool {
        route.is_image_route()
    }

}

#[async_trait::async_trait]
impl<C: Cache> AsyncRequestHandler<C> for ImageHandler<C> {
    async fn handle_request(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        let route = Route::from_path(req.uri().path());
        let dispatcher = RequestDispatcher::new();

        if route.is_image_route() {
            dispatcher.handle_route(route, state).await
        } else {
            Err(GaryError::InvalidRoute)
        }
    }
}

/// Specialized handler for text content routes (quotes/jokes)
pub struct TextHandler<C: Cache> {
    _phantom: PhantomData<C>,
}

impl<C: Cache> TextHandler<C> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C: Cache> RequestHandler<C> for TextHandler<C> {
    fn can_handle(&self, route: &Route) -> bool {
        route.is_text_route()
    }

}

#[async_trait::async_trait]
impl<C: Cache> AsyncRequestHandler<C> for TextHandler<C> {
    async fn handle_request(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Result<Response<Full<Bytes>>, GaryError> {
        let route = Route::from_path(req.uri().path());
        let dispatcher = RequestDispatcher::new();

        if route.is_text_route() {
            dispatcher.handle_route(route, state).await
        } else {
            Err(GaryError::InvalidRoute)
        }
    }
}

/// Handler registry for managing multiple handlers (simplified version)
pub struct HandlerRegistry<C: Cache> {
    _phantom: PhantomData<C>,
}

impl<C: Cache> HandlerRegistry<C> {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Handle a request using static dispatch
    pub async fn handle_request(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Response<Full<Bytes>> {
        let _route = Route::from_path(req.uri().path());
        let dispatcher = RequestDispatcher::new();
        dispatcher.dispatch(req, state).await
    }
}

impl<C: Cache> Default for HandlerRegistry<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Main handler facade that provides a simple interface
pub struct MainHandler<C: Cache> {
    dispatcher: RequestDispatcher<C>,
}

impl<C: Cache> MainHandler<C> {
    /// Create a new main handler
    pub const fn new() -> Self {
        Self {
            dispatcher: RequestDispatcher::new(),
        }
    }

    /// Handle an HTTP request
    pub async fn handle(
        &self,
        req: Request<Incoming>,
        state: &AppState<C>,
    ) -> Response<Full<Bytes>> {
        self.dispatcher.dispatch(req, state).await
    }
}

impl<C: Cache> Default for MainHandler<C> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::FileCache, config::Config};
    use hyper::{Uri, body::Incoming};

    fn create_test_state() -> AppState<FileCache> {
        let config = Config::default();
        let cache = FileCache::new();
        AppState::new(config, cache)
    }

    fn create_test_request(uri: &str) -> Request<Incoming> {
        // SAFETY: this is only for test dummies; the body is never read. not a sin
        let dummy_body: Incoming = unsafe { std::mem::zeroed() };
        Request::builder()
            .method(Method::GET)
            .uri(uri.parse::<Uri>().unwrap())
            .body(dummy_body)
            .unwrap()
    }

    #[tokio::test]
    async fn test_main_handler() {
        let handler = MainHandler::new();
        let state = create_test_state();
        let req = create_test_request("/gary");

        let response = handler.handle(req, &state).await;
        assert_eq!(response.status(), hyper::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_url_handler() {
        let handler = UrlHandler::<FileCache>::new();
        let _state = create_test_state();
        let route = Route::GaryUrl;

        assert!(handler.can_handle(&route));
    }

    #[tokio::test]
    async fn test_image_handler() {
        let handler = ImageHandler::<FileCache>::new();
        let route = Route::GaryImage;

        assert!(handler.can_handle(&route));
    }

    #[tokio::test]
    async fn test_text_handler() {
        let handler = TextHandler::<FileCache>::new();
        let route = Route::Quote;

        assert!(handler.can_handle(&route));
    }

    #[test]
    fn test_route_dispatcher() {
        let dispatcher = RequestDispatcher::<FileCache>::new();
        let _dispatcher_clone = dispatcher;
    }
}
