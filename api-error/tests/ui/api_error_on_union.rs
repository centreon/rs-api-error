use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
union MyError {
    field: i32,
}

fn main() {}
