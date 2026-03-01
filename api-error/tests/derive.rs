// Copyright 2025-Present Centreon
// SPDX-License-Identifier: Apache-2.0

use api_error::ApiError;
use axum_core::response::IntoResponse;
use http::StatusCode;

#[test]
fn api_error_basic_enum() {
    #[derive(Debug, thiserror::Error, ApiError)]
    enum MyError {
        #[error("My first error")]
        #[api_error(status_code = 400, message = "A foo error occurred")]
        Foo,

        #[error("My second error")]
        #[api_error(status_code = 403, message = "A bar error occurred")]
        Bar,

        #[error("My third error")]
        #[api_error(status_code = StatusCode::IM_A_TEAPOT, message = "A baz error occurred")]
        Baz,

        #[error("My no message error")]
        Nada,
    }

    let err = MyError::Foo;
    assert_eq!(err.status_code(), 400);
    assert_eq!(err.message().as_ref(), "A foo error occurred");
    assert_eq!(err.to_string(), "My first error");
    assert_eq!(err.into_response().status(), 400);

    let err = MyError::Bar;
    assert_eq!(err.status_code(), 403);
    assert_eq!(err.message().as_ref(), "A bar error occurred");
    assert_eq!(err.to_string(), "My second error");
    assert_eq!(err.into_response().status(), 403);

    let err = MyError::Baz;
    assert_eq!(err.status_code(), 418);
    assert_eq!(err.message().as_ref(), "A baz error occurred");
    assert_eq!(err.to_string(), "My third error");
    assert_eq!(err.into_response().status(), 418);

    let err = MyError::Nada;
    assert_eq!(err.status_code(), 500);
    assert_eq!(err.message().as_ref(), "Internal Server Error");
    assert_eq!(err.to_string(), "My no message error");
    assert_eq!(err.into_response().status(), 500);
}

#[test]
fn api_error_complex_enum() {
    #[derive(Debug, thiserror::Error, ApiError)]
    enum MyError {
        #[error("My first error {1}")]
        #[api_error(status_code = 400, message = "A foo error, {1} occurred")]
        Foo(String, usize),

        #[error("My second error")]
        #[api_error(status_code = 403, message = "A bar error {detail} occurred")]
        Bar { detail: String, code: i32 },

        #[error("My third error")]
        #[api_error(status_code = StatusCode::IM_A_TEAPOT, message = "A baz error occurred")]
        Baz,

        #[error("My inherited error")]
        #[api_error(message(inherit), status_code = StatusCode::INTERNAL_SERVER_ERROR)]
        Inherited,

        #[error(transparent)]
        #[api_error(transparent)]
        Transparent(InnerError),

        #[error("My inherited error")]
        #[api_error(message = "Hey")]
        #[api_error(status_code = StatusCode::PROCESSING)]
        MultiAttrs,

        #[error("My no message error")]
        #[api_error(status_code = StatusCode::BAD_GATEWAY)]
        NoMessage,
    }

    #[derive(Debug, thiserror::Error, ApiError)]
    enum InnerError {
        #[error("Inner error")]
        #[api_error(message(inherit), status_code = 402)]
        Inner,
    }

    let err = MyError::Foo("FOO".to_string(), 42);
    assert_eq!(err.status_code(), 400);
    assert_eq!(err.message().as_ref(), "A foo error, 42 occurred");
    assert_eq!(err.to_string(), "My first error 42");
    assert_eq!(err.into_response().status(), 400);

    let err = MyError::Bar {
        detail: "DETAILS".to_string(),
        code: 420,
    };

    assert_eq!(err.status_code(), 403);
    assert_eq!(err.message().as_ref(), "A bar error DETAILS occurred");
    assert_eq!(err.to_string(), "My second error");
    assert_eq!(err.into_response().status(), 403);

    let err = MyError::Baz;
    assert_eq!(err.status_code(), 418);
    assert_eq!(err.message().as_ref(), "A baz error occurred");
    assert_eq!(err.to_string(), "My third error");
    assert_eq!(err.into_response().status(), 418);

    let err = MyError::Inherited;
    assert_eq!(err.status_code(), 500);
    assert_eq!(err.message().as_ref(), err.to_string());
    assert_eq!(err.into_response().status(), 500);

    let err = MyError::Transparent(InnerError::Inner);
    assert_eq!(err.status_code(), 402);
    assert_eq!(err.message().as_ref(), "Inner error");
    assert_eq!(err.to_string(), "Inner error");
    assert_eq!(err.into_response().status(), 402);

    let err = MyError::MultiAttrs;
    assert_eq!(err.status_code(), 102);
    assert_eq!(err.message().as_ref(), "Hey");
    assert_eq!(err.into_response().status(), 102);

    let err = MyError::NoMessage;
    assert_eq!(err.status_code(), 502);
    assert_eq!(err.message().as_ref(), "Bad Gateway");
    assert_eq!(err.into_response().status(), 502);
}

#[test]
fn api_error_struct() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("My Error {0}")]
    #[api_error(message = "A struct error occurred: {1}", status_code = 499)]
    struct MyErrorStruct(String, usize);

    let err = MyErrorStruct("DETAIL".to_string(), 123);
    assert_eq!(err.status_code(), 499);
    assert_eq!(err.message().as_ref(), "A struct error occurred: 123");
    assert_eq!(err.to_string(), "My Error DETAIL");
    assert_eq!(err.into_response().status(), 499);
}

#[test]
fn api_error_struct_named_fields() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("Invalid input: {field}")]
    #[api_error(
        status_code = 422,
        message = "Field `{field}` is invalid (code {code})"
    )]
    struct ValidationError {
        field: String,
        code: u32,
    }

    let err = ValidationError {
        field: "email".to_string(),
        code: 1001,
    };

    assert_eq!(err.status_code(), 422);
    assert_eq!(
        err.message().as_ref(),
        "Field `email` is invalid (code 1001)"
    );
    assert_eq!(err.to_string(), "Invalid input: email");
    assert_eq!(err.into_response().status(), 422);
}
#[test]
fn api_error_struct_default_status_code() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("Something went wrong")]
    #[api_error(message = "Unexpected error")]
    struct DefaultStatusError;

    let err = DefaultStatusError;

    assert_eq!(err.status_code(), 500);
    assert_eq!(err.message().as_ref(), "Unexpected error");
    assert_eq!(err.to_string(), "Something went wrong");
    assert_eq!(err.into_response().status(), 500);
}
#[test]
fn api_error_struct_no_message_fallback() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("Upstream failure")]
    #[api_error(status_code = StatusCode::SERVICE_UNAVAILABLE)]
    struct NoMessageStruct;

    let err = NoMessageStruct;

    assert_eq!(err.status_code(), 503);
    assert_eq!(err.message().as_ref(), "Service Unavailable");
    assert_eq!(err.to_string(), "Upstream failure");
    assert_eq!(err.into_response().status(), 503);
}
#[test]
fn api_error_struct_message_inherit() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("Inherited struct error")]
    #[api_error(message(inherit), status_code = 409)]
    struct InheritedStructError;

    let err = InheritedStructError;

    assert_eq!(err.status_code(), 409);
    assert_eq!(err.message().as_ref(), "Inherited struct error");
    assert_eq!(err.to_string(), "Inherited struct error");
    assert_eq!(err.into_response().status(), 409);
}
#[test]
fn api_error_struct_multiple_attributes() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("Multi attr struct")]
    #[api_error(message = "First message")]
    #[api_error(status_code = StatusCode::ACCEPTED)]
    struct MultiAttrStruct;

    let err = MultiAttrStruct;

    assert_eq!(err.status_code(), 202);
    assert_eq!(err.message().as_ref(), "First message");
    assert_eq!(err.to_string(), "Multi attr struct");
    assert_eq!(err.into_response().status(), 202);
}
#[test]
fn api_error_struct_fully_transparent() {
    #[derive(Debug, thiserror::Error, ApiError)]
    #[error("Base error")]
    #[api_error(status_code = 401, message = "Unauthorized")]
    struct BaseError;

    #[derive(Debug, thiserror::Error, ApiError)]
    #[error(transparent)]
    #[api_error(transparent)]
    struct TransparentWrapper<E: ApiError>(E);

    let err = TransparentWrapper(BaseError);

    assert_eq!(err.status_code(), 401);
    assert_eq!(err.message().as_ref(), "Unauthorized");
    assert_eq!(err.to_string(), "Base error");
    assert_eq!(err.into_response().status(), 401);
}
