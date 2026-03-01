use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error(transparent)]
    #[api_error(transparent)]
    #[api_error(transparent)]
    Transparent(InnerError),
}

#[derive(Debug, thiserror::Error, ApiError)]
#[error("this is an inner error!")]
struct InnerError;

fn main() {}
