// Copyright 2026-Present Centreon
// SPDX-License-Identifier: Apache-2.0

//! The derive must work when the `ApiError` trait is not in scope.

#[derive(Debug, thiserror::Error, api_error::ApiError)]
#[error("teapot")]
#[api_error(status_code = 418)]
struct Teapot;

#[derive(Debug, thiserror::Error, api_error::ApiError)]
enum Wrapper {
    #[error(transparent)]
    #[api_error(transparent)]
    Teapot(Teapot),

    #[error("conflict")]
    #[api_error(status_code = 409)]
    Conflict,
}

#[test]
fn derive_does_not_need_the_trait_in_scope() {
    let err = Wrapper::Teapot(Teapot);
    assert_eq!(api_error::ApiError::status_code(&err), 418);
    assert_eq!(api_error::ApiError::message(&err), "I'm a teapot");

    let err = Wrapper::Conflict;
    assert_eq!(api_error::ApiError::status_code(&err), 409);
    assert_eq!(api_error::ApiError::message(&err), "Conflict");
}
