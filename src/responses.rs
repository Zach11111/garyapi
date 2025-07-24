//! Response building abstractions with zero-cost builders
//!
//! This module provides type-safe response builders that use compile-time dispatch
//! and zero-cost abstractions to efficiently build HTTP responses.

use crate::types::{BaseUrl, ContentType, FileName, HttpConstants, JsonConstants, ResourceType};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::marker::PhantomData;

/// Response types for compile-time dispatch
pub trait ResponseType: Send + Sync + 'static {}

/// JSON response marker
pub struct JsonResponseType;
impl ResponseType for JsonResponseType {}

/// Image response marker
pub struct ImageResponseType;
impl ResponseType for ImageResponseType {}

/// Error response marker
pub struct ErrorResponseType;
impl ResponseType for ErrorResponseType {}

/// Zero-cost response builder with compile-time type safety
pub struct ResponseBuilder<T: ResponseType> {
    _phantom: PhantomData<T>,
}

impl<T: ResponseType> ResponseBuilder<T> {
    /// Create a new response builder
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// JSON response builder specialization
impl ResponseBuilder<JsonResponseType> {
    /// Build a URL response for Gary or Goober
    pub fn build_url_response(
        &self,
        resource: ResourceType,
        base_url: &BaseUrl,
        filename: &FileName,
    ) -> Response<Full<Bytes>> {
        let (prefix, suffix, end) = match resource {
            ResourceType::Gary => (
                JsonConstants::GARY_PREFIX,
                JsonConstants::GARY_SUFFIX,
                JsonConstants::GARY_END,
            ),
            ResourceType::Goober => (
                JsonConstants::GOOBER_PREFIX,
                JsonConstants::GOOBER_SUFFIX,
                JsonConstants::GOOBER_END,
            ),
        };

        let mut body = Vec::with_capacity(
            prefix.len() + base_url.len() + suffix.len() + filename.as_bytes().len() + end.len(),
        );

        body.extend_from_slice(prefix);
        body.extend_from_slice(base_url.as_bytes());
        body.extend_from_slice(suffix);
        body.extend_from_slice(filename.as_bytes());
        body.extend_from_slice(end);

        self.build_json_response_from_bytes(Bytes::from(body))
    }

    /// Build a quote response
    pub fn build_quote_response(&self, quote: &Bytes) -> Response<Full<Bytes>> {
        self.build_json_response(JsonConstants::QUOTE_PREFIX, quote, JsonConstants::QUOTE_END)
    }

    /// Build a joke response
    pub fn build_joke_response(&self, joke: &Bytes) -> Response<Full<Bytes>> {
        self.build_json_response(JsonConstants::JOKE_PREFIX, joke, JsonConstants::JOKE_END)
    }

    /// Build an error response
    pub fn build_error_response(&self, message: &[u8]) -> Response<Full<Bytes>> {
        self.build_json_response(
            JsonConstants::ERROR_PREFIX,
            message,
            JsonConstants::ERROR_END,
        )
    }

    /// Build a generic JSON response with prefix, content, and suffix
    #[inline]
    fn build_json_response(
        &self,
        prefix: &[u8],
        content: &[u8],
        suffix: &[u8],
    ) -> Response<Full<Bytes>> {
        let mut body = Vec::with_capacity(prefix.len() + content.len() + suffix.len());
        body.extend_from_slice(prefix);
        body.extend_from_slice(content);
        body.extend_from_slice(suffix);

        self.build_json_response_from_bytes(Bytes::from(body))
    }

    /// Build a JSON response from pre-assembled bytes
    #[inline]
    fn build_json_response_from_bytes(&self, body: Bytes) -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::OK)
            .header(
                HttpConstants::HEADER_CONTENT_TYPE,
                ContentType::ApplicationJson.as_str(),
            )
            .header(HttpConstants::HEADER_CONTENT_LENGTH, body.len())
            .body(Full::new(body))
            .expect("Failed to build JSON response")
    }
}

/// Image response builder specialization
impl ResponseBuilder<ImageResponseType> {
    /// Build an image response with appropriate MIME type
    pub fn build_image_response(
        &self,
        content: Bytes,
        content_type: ContentType,
    ) -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::OK)
            .header(HttpConstants::HEADER_CONTENT_TYPE, content_type.as_str())
            .header(HttpConstants::HEADER_CONTENT_LENGTH, content.len())
            .header(
                HttpConstants::HEADER_CACHE_CONTROL,
                HttpConstants::CACHE_CONTROL_NO_STORE,
            )
            .body(Full::new(content))
            .expect("Failed to build image response")
    }

    /// Build an image response from filename (auto-detects content type)
    pub fn build_image_response_with_filename(
        &self,
        content: Bytes,
        filename: &FileName,
    ) -> Response<Full<Bytes>> {
        let content_type = ContentType::from_filename(filename);
        self.build_image_response(content, content_type)
    }
}

/// Error response builder specialization
impl ResponseBuilder<ErrorResponseType> {
    /// Build a 404 Not Found response
    pub fn build_not_found_response(&self) -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(
                HttpConstants::HEADER_CONTENT_TYPE,
                ContentType::TextPlain.as_str(),
            )
            .header(
                HttpConstants::HEADER_CONTENT_LENGTH,
                HttpConstants::NOT_FOUND.len(),
            )
            .body(Full::new(Bytes::from_static(HttpConstants::NOT_FOUND)))
            .expect("Failed to build not found response")
    }

    /// Build a generic error response with status code
    pub fn build_error_response_with_status(
        &self,
        status: StatusCode,
        message: &'static str,
    ) -> Response<Full<Bytes>> {
        let body = Bytes::from_static(message.as_bytes());
        Response::builder()
            .status(status)
            .header(
                HttpConstants::HEADER_CONTENT_TYPE,
                ContentType::TextPlain.as_str(),
            )
            .header(HttpConstants::HEADER_CONTENT_LENGTH, body.len())
            .body(Full::new(body))
            .expect("Failed to build error response")
    }
}

/// Convenience type aliases for different response builders
pub type JsonResponse = ResponseBuilder<JsonResponseType>;
pub type ImageResponse = ResponseBuilder<ImageResponseType>;
pub type ErrorResponse = ResponseBuilder<ErrorResponseType>;

/// Global response builder instances (zero-cost)
pub struct ResponseBuilders;

impl ResponseBuilders {
    pub const JSON: JsonResponse = JsonResponse::new();
    pub const IMAGE: ImageResponse = ImageResponse::new();
    pub const ERROR: ErrorResponse = ErrorResponse::new();
}

// forgive me father for I have sinned

/// Response factory trait for creating different types of responses
pub trait Responser {
    /// Create a URL response
    fn url_response(
        resource: ResourceType,
        base_url: &BaseUrl,
        filename: &FileName,
    ) -> Response<Full<Bytes>>;

    /// Create a quote response
    fn quote_response(quote: &Bytes) -> Response<Full<Bytes>>;

    /// Create a joke response
    fn joke_response(joke: &Bytes) -> Response<Full<Bytes>>;

    /// Create an image response
    fn image_response(content: Bytes, filename: &FileName) -> Response<Full<Bytes>>;

    /// Create a not found response
    fn not_found_response() -> Response<Full<Bytes>>;

    /// Create an error response
    fn error_response(message: &[u8]) -> Response<Full<Bytes>>;
}

/// Default response factory implementation using global builders
pub struct DefaultResponser;

impl Responser for DefaultResponser {
    #[inline]
    fn url_response(
        resource: ResourceType,
        base_url: &BaseUrl,
        filename: &FileName,
    ) -> Response<Full<Bytes>> {
        ResponseBuilders::JSON.build_url_response(resource, base_url, filename)
    }

    #[inline]
    fn quote_response(quote: &Bytes) -> Response<Full<Bytes>> {
        ResponseBuilders::JSON.build_quote_response(quote)
    }

    #[inline]
    fn joke_response(joke: &Bytes) -> Response<Full<Bytes>> {
        ResponseBuilders::JSON.build_joke_response(joke)
    }

    #[inline]
    fn image_response(content: Bytes, filename: &FileName) -> Response<Full<Bytes>> {
        ResponseBuilders::IMAGE.build_image_response_with_filename(content, filename)
    }

    #[inline]
    fn not_found_response() -> Response<Full<Bytes>> {
        ResponseBuilders::ERROR.build_not_found_response()
    }

    #[inline]
    fn error_response(message: &[u8]) -> Response<Full<Bytes>> {
        ResponseBuilders::JSON.build_error_response(message)
    }
}

/// Fast response helpers for common cases
pub mod fast {
    use super::*;

    /// Create a Gary URL response quickly
    #[inline]
    pub fn gary_url(base_url: &BaseUrl, filename: &FileName) -> Response<Full<Bytes>> {
        DefaultResponser::url_response(ResourceType::Gary, base_url, filename)
    }

    /// Create a Goober URL response quickly
    #[inline]
    pub fn goober_url(base_url: &BaseUrl, filename: &FileName) -> Response<Full<Bytes>> {
        DefaultResponser::url_response(ResourceType::Goober, base_url, filename)
    }

    /// Create a quote response quickly
    #[inline]
    pub fn quote(content: &Bytes) -> Response<Full<Bytes>> {
        DefaultResponser::quote_response(content)
    }

    /// Create a joke response quickly
    #[inline]
    pub fn joke(content: &Bytes) -> Response<Full<Bytes>> {
        DefaultResponser::joke_response(content)
    }

    /// Create an image response quickly
    #[inline]
    pub fn image(content: Bytes, filename: &FileName) -> Response<Full<Bytes>> {
        DefaultResponser::image_response(content, filename)
    }

    /// Create a not found response quickly
    #[inline]
    pub fn not_found() -> Response<Full<Bytes>> {
        DefaultResponser::not_found_response()
    }

    /// Create an error response quickly
    #[inline]
    pub fn error(message: &[u8]) -> Response<Full<Bytes>> {
        DefaultResponser::error_response(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_response_builder() {
        let builder = JsonResponse::new();
        let base_url = BaseUrl::new("http://example.com");
        let filename = FileName::new_unchecked("test.jpg");

        let response = builder.build_url_response(ResourceType::Gary, &base_url, &filename);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_image_response_builder() {
        let builder = ImageResponse::new();
        let content = Bytes::from("fake image data");
        let filename = FileName::new_unchecked("test.jpg");

        let response = builder.build_image_response_with_filename(content, &filename);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_error_response_builder() {
        let builder = ErrorResponse::new();
        let response = builder.build_not_found_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_response_factory() {
        let base_url = BaseUrl::new("http://example.com");
        let filename = FileName::new_unchecked("test.jpg");
        let quote = Bytes::from("Test quote");

        let url_response =
            DefaultResponser::url_response(ResourceType::Gary, &base_url, &filename);
        assert_eq!(url_response.status(), StatusCode::OK);

        let quote_response = DefaultResponser::quote_response(&quote);
        assert_eq!(quote_response.status(), StatusCode::OK);

        let not_found = DefaultResponser::not_found_response();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_fast_helpers() {
        let base_url = BaseUrl::new("http://example.com");
        let filename = FileName::new_unchecked("test.jpg");
        let quote = Bytes::from("Test quote");

        let gary_response = fast::gary_url(&base_url, &filename);
        assert_eq!(gary_response.status(), StatusCode::OK);

        let goober_response = fast::goober_url(&base_url, &filename);
        assert_eq!(goober_response.status(), StatusCode::OK);

        let quote_response = fast::quote(&quote);
        assert_eq!(quote_response.status(), StatusCode::OK);

        let not_found = fast::not_found();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
    }
}
