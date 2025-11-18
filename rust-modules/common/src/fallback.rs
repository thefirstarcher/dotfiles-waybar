use crate::output::WaybarOutput;
use serde::{Deserialize, Serialize};

/// Trait for types that can provide fallback data
pub trait FallbackData: Sized {
    /// Return fallback value when operation fails
    fn fallback() -> Self;

    /// Return fallback with custom message
    fn fallback_with_msg(msg: impl Into<String>) -> Self;
}

/// Standard fallback data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fallback<T> {
    pub value: T,
    pub is_fallback: bool,
    pub error_msg: Option<String>,
}

impl<T> Fallback<T> {
    /// Create normal (non-fallback) data
    pub fn ok(value: T) -> Self {
        Self {
            value,
            is_fallback: false,
            error_msg: None,
        }
    }

    /// Create fallback data with error message
    pub fn error(value: T, msg: impl Into<String>) -> Self {
        Self {
            value,
            is_fallback: true,
            error_msg: Some(msg.into()),
        }
    }

    /// Check if this is fallback data
    pub fn is_fallback(&self) -> bool {
        self.is_fallback
    }

    /// Get the value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get error message if any
    pub fn error_msg(&self) -> Option<&str> {
        self.error_msg.as_deref()
    }
}

/// Implement FallbackData for WaybarOutput
impl FallbackData for WaybarOutput {
    fn fallback() -> Self {
        Self::new("N/A")
    }

    fn fallback_with_msg(msg: impl Into<String>) -> Self {
        WaybarOutput::builder()
            .text("⚠")
            .tooltip(format!("Error: {}", msg.into()))
            .class("error")
            .build()
    }
}

/// Implement FallbackData for common types
impl FallbackData for String {
    fn fallback() -> Self {
        "N/A".to_string()
    }

    fn fallback_with_msg(msg: impl Into<String>) -> Self {
        format!("Error: {}", msg.into())
    }
}

impl FallbackData for f64 {
    fn fallback() -> Self {
        0.0
    }

    fn fallback_with_msg(_msg: impl Into<String>) -> Self {
        0.0
    }
}

impl FallbackData for u64 {
    fn fallback() -> Self {
        0
    }

    fn fallback_with_msg(_msg: impl Into<String>) -> Self {
        0
    }
}

impl FallbackData for i64 {
    fn fallback() -> Self {
        0
    }

    fn fallback_with_msg(_msg: impl Into<String>) -> Self {
        0
    }
}

impl FallbackData for bool {
    fn fallback() -> Self {
        false
    }

    fn fallback_with_msg(_msg: impl Into<String>) -> Self {
        false
    }
}

/// Extension trait for Result to easily convert to fallback
pub trait ResultFallbackExt<T: FallbackData> {
    /// Convert error to fallback value
    fn or_fallback(self) -> T;

    /// Convert error to fallback with wrapped result
    fn to_fallback(self) -> Fallback<T>;
}

impl<T: FallbackData, E: std::fmt::Display> ResultFallbackExt<T> for Result<T, E> {
    fn or_fallback(self) -> T {
        self.unwrap_or_else(|e| {
            tracing::warn!("Using fallback due to error: {}", e);
            T::fallback()
        })
    }

    fn to_fallback(self) -> Fallback<T> {
        match self {
            Ok(value) => Fallback::ok(value),
            Err(e) => {
                let msg = e.to_string();
                tracing::warn!("Error occurred: {}", msg);
                Fallback::error(T::fallback(), msg)
            }
        }
    }
}
