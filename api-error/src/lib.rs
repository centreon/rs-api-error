// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0
#![warn(clippy::pedantic)]

//! # Api Error
//!
//! A Rust crate for easily defining API-friendly error types with HTTP status codes and user-facing error messages.
//!
//! ## Usage
//!
//! ### Basic Enum Example
//!
//! ```rust
//! use api_error::ApiError;
//! use http::StatusCode;
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! enum MyError {
//!     #[error("Invalid input")]
//!     #[api_error(status_code = 400, message = "The provided input is invalid")]
//!     InvalidInput,
//!
//!     #[error("Resource not found")]
//!     #[api_error(status_code = 404, message = "The requested resource was not found")]
//!     NotFound,
//!
//!     #[error("Internal error")]
//!     #[api_error(status_code = StatusCode::INTERNAL_SERVER_ERROR)]
//!     Internal,
//! }
//!
//! let err = MyError::InvalidInput;
//! assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
//! assert_eq!(err.message().as_ref(), "The provided input is invalid");
//! assert_eq!(err.to_string(), "Invalid input");  // From thiserror
//! ```
//!
//! ### Enum with Fields and Formatting
//!
//! ```rust
//! use api_error::ApiError;
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! enum AppError {
//!     // Unnamed fields with positional formatting
//!     #[error("Database error: {0}")]
//!     #[api_error(status_code = 500, message = "Database operation failed: {0}")]
//!     Database(String),
//!
//!     // Named fields with named formatting
//!     #[error("Validation failed on {field}")]
//!     #[api_error(status_code = 422, message = "Field `{field}` has invalid value")]
//!     Validation { field: String, value: String },
//! }
//!
//! let err = AppError::Database("Connection timeout".to_string());
//! assert_eq!(err.status_code().as_u16(), 500);
//! assert_eq!(err.message().as_ref(), "Database operation failed: Connection timeout");
//!
//! let err = AppError::Validation {
//!     field: "email".to_string(),
//!     value: "invalid".to_string(),
//! };
//! assert_eq!(err.status_code().as_u16(), 422);
//! assert_eq!(err.message().as_ref(), "Field `email` has invalid value");
//! ```
//!
//! ### Struct Example
//!
//! ```rust
//! use api_error::ApiError;
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! #[error("Authentication failed: {reason}")]
//! #[api_error(status_code = 401, message = "Authentication failed")]
//! struct AuthError {
//!     reason: String,
//! }
//!
//! let err = AuthError {
//!     reason: "Invalid token".to_string(),
//! };
//! assert_eq!(err.status_code().as_u16(), 401);
//! assert_eq!(err.message().as_ref(), "Authentication failed");
//! ```
//!
//! ### Message Inheritance
//!
//! Use `message(inherit)` to use the `Display` implementation as the user-facing message:
//!
//! ```rust
//! use api_error::ApiError;
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! enum MyError {
//!     #[error("User-friendly error message")]
//!     #[api_error(message(inherit), status_code = 400)]
//!     BadRequest,
//! }
//!
//! let err = MyError::BadRequest;
//! assert_eq!(err.message().as_ref(), "User-friendly error message");
//! ```
//!
//! ### Transparent Forwarding
//!
//! Forward both status code and message from an inner error:
//!
//! ```rust
//! use api_error::ApiError;
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! #[error("Database error")]
//! #[api_error(status_code = 503, message = "Service temporarily unavailable")]
//! struct DatabaseError;
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! enum AppError {
//!     #[error(transparent)]
//!     #[api_error(transparent)]
//!     Database(DatabaseError),
//!
//!     #[error("Other error")]
//!     #[api_error(status_code = 500, message = "Internal error")]
//!     Other,
//! }
//!
//! let err = AppError::Database(DatabaseError);
//! assert_eq!(err.status_code().as_u16(), 503);  // Forwarded from DatabaseError
//! assert_eq!(err.message().as_ref(), "Service temporarily unavailable");
//! ```
//!
//! ### Axum Integration
//!
//! With the `axum` feature enabled, `ApiError` types automatically implement `IntoResponse`:
//!
//! ```rust
//! use api_error::ApiError;
//! use axum::{Router, routing::get};
//!
//! #[derive(Debug, thiserror::Error, ApiError)]
//! enum MyApiError {
//!     #[error("Not found")]
//!     #[api_error(status_code = 404, message = "Resource not found")]
//!     NotFound,
//! }
//!
//! async fn handler() -> Result<String, MyApiError> {
//!     Err(MyApiError::NotFound)
//! }
//!
//! let app: Router = Router::new().route("/", get(handler));
//!
//! // Returns JSON response:
//! // Status: 404
//! // Body: {"message": "Resource not found"}
//! ```
//!
//! ### Customizing axum error response format
//!
//! The default response body is `{"message": "<error msg>"}` with the error's
//! HTTP status code. To use a different format, register a custom responder
//! once at startup with [`axum::set_error_responder`]. Every type deriving
//! [`ApiError`] will route through it.
//!
//! ```no_run
//! use api_error::ApiError;
//! use axum_core::response::{IntoResponse, Response};
//! use http::StatusCode;
//! use serde_json::json;
//!
//! fn my_responder(err: &dyn ApiError) -> Response {
//!     let status = err.status_code();
//!     let body = serde_json::to_vec(&json!({
//!         "error": {
//!             "code": status.as_u16(),
//!             "message": err.message(),
//!         }
//!     })).unwrap();
//!     (status, body).into_response()
//! }
//!
//! api_error::axum::set_error_responder(my_responder);
//! ```

// Compile the README's code blocks as doctests. Opt-in via
// `RUSTFLAGS="--cfg readme_doctest" cargo test --all-features` (this is what CI runs).
#[cfg(readme_doctest)]
#[doc = include_str!("../../README.md")]
mod _readme_doctest {}

use std::borrow::Cow;

use http::StatusCode;

#[doc(hidden)]
pub use ::http as __http;

/// Derive macro for implementing [`ApiError`] on enums and structs.
///
/// This macro generates an [`ApiError`] implementation based on
/// `#[api_error(...)]` attributes, removing the need to write
/// `status_code()` and `message()` by hand.
///
/// It is intended to be used together with `thiserror::Error`.
///
/// ---
///
/// # Basic usage
///
/// ```
/// # use api_error::ApiError;
///
/// #[derive(Debug, thiserror::Error, ApiError)]
/// enum MyError {
///     #[error("Internal failure")]
///     #[api_error(status_code = 500, message = "Something went wrong")]
///     Failure,
/// }
/// ```
///
/// ---
///
/// # `#[api_error]` attribute
///
/// The `#[api_error(...)]` attribute may be applied to:
/// - enums
/// - enum variants
/// - structs
///
/// ## `status_code`
///
/// Sets the HTTP status code returned by the generated implementation.
///
/// You can either use the [`StatusCode`] enum or
/// a status code literal:
///
/// ```
/// # use api_error::ApiError;
/// # use http::StatusCode;
/// #[derive(Debug, thiserror::Error, ApiError)]
/// enum MyError {
///     #[api_error(status_code = 400)]
///     #[error("Got error because of A")]
///     ReasonA,
///
///     #[api_error(status_code = StatusCode::CONFLICT)]
///     #[error("Got error because of B")]
///     ReasonB,
/// }
/// assert_eq!(MyError::ReasonB.status_code(), StatusCode::CONFLICT)
/// ```
///
/// If omitted, the status code defaults to
/// `500 Internal Server Error`.
///
/// ---
///
/// ## `message`
///
/// Sets the client-facing error message.
///
/// ```ignore
/// # use api_error::ApiError;
/// # use http::StatusCode;
/// #[api_error(message = "Invalid input")]
/// ```
///
/// The message supports formatting using:
/// - tuple indices (`{0}`, `{1}`, …)
/// - named fields (`{field}`)
///
/// If omitted, the HTTP status reason phrase is used.
///
/// ---
///
/// ## `message(inherit)`
///
/// Uses the type’s `Display` implementation (from `thiserror`)
/// as the API error message.
///
/// ```ignore
/// #[error("Forbidden")]
/// #[api_error(message(inherit))]
/// struct Forbidden;
/// ```
///
/// ---
///
/// ## `transparent`
///
/// Marks the type as a transparent wrapper around another [`ApiError`].
///
/// ```
/// # use api_error::ApiError;
/// #[derive(Debug, thiserror::Error, ApiError)]
/// #[error(transparent)]
/// #[api_error(transparent)]
/// struct Wrapper(InnerError);
///
/// #[derive(Debug, thiserror::Error, ApiError)]
/// #[error("My inner error")]
/// struct InnerError;
/// ```
///
/// ### Rules
///
/// - `transparent` must be used **alone**
/// - all API metadata is delegated to the wrapped error
///
/// ---
///
/// # Multiple attributes
///
/// Multiple `#[api_error]` attributes may be used.
/// When the same field is specified multiple times,
/// the **last occurrence wins**.
///
/// ```rust
/// #[api_error(message = "Initial")]
/// #[api_error(status_code = 202)]
/// #[api_error(message = "Final")]
/// ```
#[cfg(feature = "derive")]
pub use api_error_derive::ApiError;

/// An error that can be returned by a service API.
/// ```
/// # use http::StatusCode;
/// # use api_error::ApiError;
/// # use std::borrow::Cow;
///
/// #[derive(Debug, thiserror::Error)]
/// enum MyServiceErrors {
///     #[error("Database error: {0}")]
///     Db(String),
///     #[error("Authentication error")]
///     Auth,
/// // etc...
/// }
///
/// impl ApiError for MyServiceErrors {
///     fn status_code(&self) -> StatusCode {
///         match self {
///             MyServiceErrors::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
///             MyServiceErrors::Auth => StatusCode::UNAUTHORIZED,
///         }
///     }
///     fn message(&self) -> Cow<'_, str> {
///         match self {
///             MyServiceErrors::Db(_) => "Database error".into(),
///             MyServiceErrors::Auth => "Authentication error".into(),
///         }
///     }
/// }
///
/// assert_eq!(MyServiceErrors::Db("test".to_string()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
/// assert_eq!(MyServiceErrors::Auth.status_code(), StatusCode::UNAUTHORIZED);
pub trait ApiError: std::error::Error {
    /// Returns the HTTP status code associated with the error.
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    /// Returns a human-readable message describing the error.
    /// It can be potentially shown to the user.
    fn message(&self) -> Cow<'_, str> {
        let msg = self
            .status_code()
            .canonical_reason()
            .unwrap_or("Unknown error");

        Cow::Borrowed(msg)
    }
}

/// Custom implementation for axum integration
#[cfg(feature = "axum")]
pub mod axum {
    use std::{borrow::Cow, sync::OnceLock};

    use axum_core::{
        body::Body,
        response::{IntoResponse, Response},
    };
    use http::StatusCode;
    use serde_core::{Serialize, ser::SerializeMap};

    #[doc(hidden)]
    pub use ::axum_core as __axum_core;

    use super::ApiError;

    #[doc(hidden)]
    pub static __ERROR_RESPONDER: OnceLock<ApiErrorResponder> = OnceLock::new();

    /// A function that converts an [`ApiError`] into an axum [`Response`].
    ///
    /// Register one globally with [`set_error_responder`] to customize the
    /// response format produced by types deriving [`ApiError`].
    pub type ApiErrorResponder = fn(&dyn ApiError) -> Response;

    /// Sets a custom [`ApiErrorResponder`] that will be used to convert
    /// [`ApiError`] to a [`Response`].
    ///
    /// For a non-panicking alternative, use [`try_set_error_responder`].
    ///
    /// # Panics
    ///
    /// Panics if the responder is already set.
    pub fn set_error_responder(f: ApiErrorResponder) {
        __ERROR_RESPONDER
            .set(f)
            .expect("an api error responder should be set only once");
    }

    /// Tries to set a custom [`ApiErrorResponder`] that will be used to convert
    /// [`ApiError`] to a [`Response`].
    ///
    /// # Errors
    ///
    /// Returns an error if the responder is already set.
    pub fn try_set_error_responder(f: ApiErrorResponder) -> Result<(), ApiErrorResponder> {
        __ERROR_RESPONDER.set(f)
    }

    /// The default [`ApiErrorResponder`].
    ///
    /// Returns a [`Response`] whose status is [`ApiError::status_code`] and
    /// whose JSON body is:
    ///
    /// ```json
    /// { "message": "<ApiError::message()>" }
    /// ```
    pub fn default_error_responder(api_error: &dyn ApiError) -> Response {
        ApiErrorResponse::new(api_error).into_response()
    }

    pub struct ApiErrorResponse<'a> {
        message: Cow<'a, str>,
        status_code: StatusCode,
    }

    impl<'a> ApiErrorResponse<'a> {
        pub fn new(api_error: &'a dyn ApiError) -> Self {
            Self {
                message: api_error.message(),
                status_code: api_error.status_code(),
            }
        }
    }

    impl Serialize for ApiErrorResponse<'_> {
        fn serialize<S: serde_core::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry("message", &self.message)?;
            map.end()
        }
    }

    impl IntoResponse for ApiErrorResponse<'_> {
        fn into_response(self) -> Response {
            let body =
                serde_json::to_vec(&self).expect("AxumApiError serialization should not fail");

            (self.status_code, Body::from(body)).into_response()
        }
    }
}
