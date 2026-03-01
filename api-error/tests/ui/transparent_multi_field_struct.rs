use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
#[error("some error")]
#[api_error(transparent)]
struct MyError(String, i32);

fn main() {}
