use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(message = "First message")]
    #[api_error(message = "Second message")]
    Duplicate,
}

fn main() {}
