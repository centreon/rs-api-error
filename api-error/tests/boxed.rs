// Copyright 2026-Present Centreon
// SPDX-License-Identifier: Apache-2.0

use api_error::ApiError;
use http::StatusCode;

#[derive(Debug, thiserror::Error, ApiError)]
#[error("cannot store the file")]
#[api_error(status_code = 507, message = "No space left")]
struct StoreError;

#[derive(Debug, thiserror::Error, ApiError)]
enum Pointers {
    #[error("{0}")]
    #[api_error(transparent)]
    SendSync(Box<dyn ApiError + Send + Sync>),

    #[error("{0}")]
    #[api_error(transparent)]
    Send(Box<dyn ApiError + Send>),

    #[error("{0}")]
    #[api_error(transparent)]
    Local(Box<dyn ApiError>),

    #[error("{0}")]
    #[api_error(transparent)]
    Sized(Box<StoreError>),
}

#[derive(Debug, thiserror::Error, ApiError)]
#[error("{0}")]
#[api_error(transparent)]
struct PointerStruct(Box<dyn ApiError + Send + Sync>);

#[test]
fn transparent_reaches_the_error_behind_a_pointer() {
    let errors: [&dyn ApiError; 5] = [
        &Pointers::SendSync(Box::new(StoreError)),
        &Pointers::Send(Box::new(StoreError)),
        &Pointers::Local(Box::new(StoreError)),
        &Pointers::Sized(Box::new(StoreError)),
        &PointerStruct(Box::new(StoreError)),
    ];

    for err in errors {
        assert_eq!(err.status_code(), StatusCode::INSUFFICIENT_STORAGE);
        assert_eq!(err.message().as_ref(), "No space left");
        assert_eq!(err.to_string(), "cannot store the file");
    }
}

#[cfg(feature = "axum")]
mod axum {
    use std::borrow::Cow;

    use api_error::ApiError;
    use axum_core::response::IntoResponse;
    use serde_json::json;

    use super::{PointerStruct, StoreError};

    #[derive(Debug, thiserror::Error)]
    #[error("validation failed")]
    struct ValidationError;

    impl ApiError for ValidationError {
        fn message(&self) -> Cow<'_, str> {
            Cow::Borrowed("validation failed")
        }

        fn extended(&self) -> Option<serde_json::Value> {
            Some(json!({ "field": "email" }))
        }
    }

    #[test]
    fn transparent_forwards_extended_behind_a_pointer() {
        let err = PointerStruct(Box::new(ValidationError));

        assert_eq!(err.extended(), Some(json!({ "field": "email" })));
    }

    #[test]
    fn responds_like_the_error_behind_a_pointer() {
        let err = PointerStruct(Box::new(StoreError));

        assert_eq!(err.into_response().status(), 507);
    }
}
