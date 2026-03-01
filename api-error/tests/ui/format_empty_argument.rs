use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(message = "Empty argument {}")]
    Invalid(String),
}

fn main() {}
