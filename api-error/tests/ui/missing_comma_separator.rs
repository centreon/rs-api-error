use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(message = "Error" status_code = 400)]
    Invalid,
}

fn main() {}
