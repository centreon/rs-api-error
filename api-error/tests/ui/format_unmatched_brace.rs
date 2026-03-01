use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(message = "Missing closing brace {0")]
    Invalid(String),
}

fn main() {}
