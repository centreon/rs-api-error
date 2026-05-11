# Api Error

A Rust crate for easily defining API-friendly error types with HTTP status codes and user-facing error messages.

## Purpose

When building HTTP APIs, you often need to:
1. Convert internal errors into appropriate HTTP status codes
2. Provide user-friendly error messages (different from internal error messages)
3. Maintain consistency across your error handling

`api-error` provides a derive macro that automatically implements the `ApiError` trait for your error types, making it easy to:
- Define custom HTTP status codes for each error variant
- Specify user-facing error messages with formatting
- Integrate seamlessly with Axum (optional feature)
- Forward errors transparently through your error hierarchy

## Usage

### Basic Enum Example

```rust
use api_error::ApiError;
use http::StatusCode;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("Invalid input")]
    #[api_error(status_code = 400, message = "The provided input is invalid")]
    InvalidInput,

    #[error("Resource not found")]
    #[api_error(status_code = 404, message = "The requested resource was not found")]
    NotFound,

    #[error("Internal error")]
    #[api_error(status_code = StatusCode::INTERNAL_SERVER_ERROR)]
    Internal,
}

let err = MyError::InvalidInput;
assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
assert_eq!(err.message().as_ref(), "The provided input is invalid");
assert_eq!(err.to_string(), "Invalid input");  // From thiserror
```

### Enum with Fields and Formatting

```rust
use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum AppError {
    // Unnamed fields with positional formatting
    #[error("Database error: {0}")]
    #[api_error(status_code = 500, message = "Database operation failed: {0}")]
    Database(String),

    // Named fields with named formatting
    #[error("Validation failed on {field}")]
    #[api_error(status_code = 422, message = "Field `{field}` has invalid value")]
    Validation { field: String, value: String },
}

let err = AppError::Database("Connection timeout".to_string());
assert_eq!(err.status_code().as_u16(), 500);
assert_eq!(err.message().as_ref(), "Database operation failed: Connection timeout");

let err = AppError::Validation {
    field: "email".to_string(),
    value: "invalid".to_string(),
};
assert_eq!(err.status_code().as_u16(), 422);
assert_eq!(err.message().as_ref(), "Field `email` has invalid value");
```

### Struct Example

```rust
use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
#[error("Authentication failed: {reason}")]
#[api_error(status_code = 401, message = "Authentication failed")]
struct AuthError {
    reason: String,
}

let err = AuthError {
    reason: "Invalid token".to_string(),
};
assert_eq!(err.status_code().as_u16(), 401);
assert_eq!(err.message().as_ref(), "Authentication failed");
```

### Message Inheritance

Use `message(inherit)` to use the `Display` implementation as the user-facing message:

```rust
use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
enum MyError {
    #[error("User-friendly error message")]
    #[api_error(message(inherit), status_code = 400)]
    BadRequest,
}

let err = MyError::BadRequest;
assert_eq!(err.message().as_ref(), "User-friendly error message");
```

### Transparent Forwarding

Forward both status code and message from an inner error:

```rust
use api_error::ApiError;

#[derive(Debug, thiserror::Error, ApiError)]
#[error("Database error")]
#[api_error(status_code = 503, message = "Service temporarily unavailable")]
struct DatabaseError;

#[derive(Debug, thiserror::Error, ApiError)]
enum AppError {
    #[error(transparent)]
    #[api_error(transparent)]
    Database(DatabaseError),
    
    #[error("Other error")]
    #[api_error(status_code = 500, message = "Internal error")]
    Other,
}

let err = AppError::Database(DatabaseError);
assert_eq!(err.status_code().as_u16(), 503);  // Forwarded from DatabaseError
assert_eq!(err.message().as_ref(), "Service temporarily unavailable");
```

### Axum Integration

With the `axum` feature enabled, `ApiError` types automatically implement `IntoResponse`:

```rust
use api_error::ApiError;
use axum::{Router, routing::get};

#[derive(Debug, thiserror::Error, ApiError)]
enum MyApiError {
    #[error("Not found")]
    #[api_error(status_code = 404, message = "Resource not found")]
    NotFound,
}

async fn handler() -> Result<String, MyApiError> {
    Err(MyApiError::NotFound)
}

let app: Router = Router::new().route("/", get(handler));

// Returns JSON response:
// Status: 404
// Body: {"message": "Resource not found"}
```

### Customizing axum error response format

The default response body is `{"message": "<error msg>"}` with the error's HTTP
status code. To use a different format, register a custom responder once at
startup with `api_error::axum::set_error_responder`. Every type deriving
`ApiError` will route through it.

```rust
use api_error::ApiError;
use axum::{Json, response::{IntoResponse, Response}};
use serde_json::json;

fn my_responder(err: &dyn ApiError) -> Response {
    let status = err.status_code();
    let body = json!({
        "error": {
            "code": status.as_u16(),
            "message": err.message(),
        }
    });
    (status, Json(body)).into_response()
}

fn main() {
    api_error::axum::set_error_responder(my_responder);
    // ... build router and serve
}
```
