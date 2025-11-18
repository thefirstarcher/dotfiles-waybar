use thiserror::Error;

/// Common error type for all Waybar modules
#[derive(Error, Debug)]
pub enum WaybarError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Module error: {0}")]
    Module(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

/// Result type alias using WaybarError
pub type Result<T> = std::result::Result<T, WaybarError>;

/// Extension trait for converting Results to fallback values
pub trait ResultExt<T> {
    /// Convert error to fallback value and log the error
    fn or_fallback(self, fallback: T) -> T;

    /// Convert error to fallback value with custom error message
    fn or_fallback_msg(self, fallback: T, msg: &str) -> T;
}

impl<T> ResultExt<T> for Result<T> {
    fn or_fallback(self, fallback: T) -> T {
        self.unwrap_or_else(|e| {
            tracing::warn!("Error occurred, using fallback: {}", e);
            fallback
        })
    }

    fn or_fallback_msg(self, fallback: T, msg: &str) -> T {
        self.unwrap_or_else(|e| {
            tracing::warn!("{}: {}", msg, e);
            fallback
        })
    }
}
