use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("some error")]
    #[api_error(transparent)]
    TwoFields(String, i32),
}

fn main() {}
