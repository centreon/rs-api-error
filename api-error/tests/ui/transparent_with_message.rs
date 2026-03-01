use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(transparent, message = "Cannot be combined")]
    Transparent(InnerError),
}

#[derive(Debug, thiserror::Error, ApiError)]
#[error("this is an inner error!")]
struct InnerError;

fn main() {}
