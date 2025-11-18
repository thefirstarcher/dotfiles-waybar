// Waybar Common Library - DRY utilities shared across all modules

pub mod cache;
pub mod error;
pub mod fallback;
pub mod logging;
pub mod output;
pub mod retry;

// Re-export commonly used types
pub use error::{Result, WaybarError};
pub use output::{WaybarOutput, WaybarOutputBuilder};
pub use cache::Cache;
pub use retry::RetryStrategy;
pub use fallback::FallbackData;

// Common constants
pub const CACHE_DIR: &str = ".config/waybar/cache";
pub const LOG_FILE: &str = ".config/waybar/logs/waybar.log";
