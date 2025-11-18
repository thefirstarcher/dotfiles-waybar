use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Standard Waybar JSON output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaybarOutput {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u32>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl WaybarOutput {
    /// Create a new output with just text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            alt: None,
            tooltip: None,
            class: None,
            percentage: None,
            extra: HashMap::new(),
        }
    }

    /// Create a builder for more complex output
    pub fn builder() -> WaybarOutputBuilder {
        WaybarOutputBuilder::default()
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> crate::Result<String> {
        serde_json::to_string(self).map_err(Into::into)
    }

    /// Convert to pretty JSON string
    pub fn to_json_pretty(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self).map_err(Into::into)
    }

    /// Print JSON to stdout (for Waybar consumption)
    pub fn print(&self) {
        if let Ok(json) = self.to_json() {
            println!("{}", json);
        } else {
            tracing::error!("Failed to serialize Waybar output");
        }
    }
}

/// Builder for WaybarOutput with fluent API
#[derive(Default)]
pub struct WaybarOutputBuilder {
    text: String,
    alt: Option<String>,
    tooltip: Option<String>,
    class: Vec<String>,
    percentage: Option<u32>,
    extra: HashMap<String, Value>,
}

impl WaybarOutputBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class.push(class.into());
        self
    }

    pub fn classes(mut self, classes: Vec<String>) -> Self {
        self.class.extend(classes);
        self
    }

    pub fn percentage(mut self, percentage: u32) -> Self {
        self.percentage = Some(percentage.min(100));
        self
    }

    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    pub fn build(self) -> WaybarOutput {
        WaybarOutput {
            text: self.text,
            alt: self.alt,
            tooltip: self.tooltip,
            class: if self.class.is_empty() {
                None
            } else {
                Some(self.class)
            },
            percentage: self.percentage,
            extra: self.extra,
        }
    }
}

/// Helper function to create simple error output
pub fn error_output(msg: impl Into<String>) -> WaybarOutput {
    WaybarOutput::builder()
        .text("⚠")
        .tooltip(msg)
        .class("error")
        .build()
}

/// Helper function to create loading output
pub fn loading_output(msg: impl Into<String>) -> WaybarOutput {
    WaybarOutput::builder()
        .text("...")
        .tooltip(msg)
        .class("loading")
        .build()
}
