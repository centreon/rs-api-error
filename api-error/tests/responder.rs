// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "axum")]

use api_error::{
    ApiError,
    axum::{ApiErrorResponse, set_error_responder},
};
use axum_core::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error, ApiError)]
#[error("boom")]
#[api_error(status_code = 418, message = "I'm a teapot")]
struct TeapotError;

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
