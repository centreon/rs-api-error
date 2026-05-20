// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "axum")]

use std::borrow::Cow;

use api_error::{
    ApiError,
    axum::{ApiErrorResponse, set_error_responder},
};
use axum_core::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::json;

#[derive(Debug, thiserror::Error, ApiError)]
#[error("boom")]
#[api_error(status_code = 418, message = "I'm a teapot")]
struct TeapotError;

#[derive(Debug, thiserror::Error)]
#[error("validation failed")]
struct ValidationError;

impl ApiError for ValidationError {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNPROCESSABLE_ENTITY
    }

    fn message(&self) -> Cow<'_, str> {
        Cow::Borrowed("validation failed")
    }

    fn extended(&self) -> Option<serde_json::Value> {
        Some(json!({ "field": "email" }))
    }
}

fn custom_responder(err: &dyn ApiError) -> Response {
    let mut resp = ApiErrorResponse::new(err).into_response();
    resp.headers_mut()
        .insert("x-custom-responder", "1".parse().unwrap());
    resp
}

#[test]
fn custom_responder_is_used_by_derive() {
    set_error_responder(custom_responder);

    let resp = TeapotError.into_response();
    assert_eq!(resp.status(), 418);
    assert_eq!(resp.headers().get("x-custom-responder").unwrap(), "1");
}

#[test]
fn default_responder_serializes_message_as_json() {
    let body = serde_json::to_string(&ApiErrorResponse::new(&TeapotError)).unwrap();
    assert_eq!(body, r#"{"message":"I'm a teapot"}"#);
}

#[test]
fn default_responder_forwards_status_code() {
    let resp = TeapotError.into_response();
    assert_eq!(resp.status(), 418);
}

#[test]
fn extended_defaults_to_none_for_derived_errors() {
    assert!(TeapotError.extended().is_none());
}

#[test]
fn extended_appears_in_response_body() {
    let body = serde_json::to_string(&ApiErrorResponse::new(&ValidationError)).unwrap();
    assert_eq!(
        body,
        r#"{"message":"validation failed","extended":{"field":"email"}}"#
    );
}

#[test]
fn extended_forwarded_through_reference() {
    let err = ValidationError;
    let by_ref: &ValidationError = &err;
    assert_eq!(by_ref.extended(), Some(json!({ "field": "email" })));
}
