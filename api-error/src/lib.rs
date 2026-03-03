// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0
#![warn(clippy::pedantic)]
#![doc = include_str!("../../README.md")]

use std::borrow::Cow;

use http::StatusCode;

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
    use std::borrow::Cow;

    use axum_core::{
        body::Body,
        response::{IntoResponse, Response},
    };
    use http::StatusCode;
    use serde_core::{Serialize, ser::SerializeMap};

    use crate::ApiError;

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
