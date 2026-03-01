use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(status_code = 400)]
    #[api_error(status_code = 500)]
    Duplicate,
}

fn main() {}
