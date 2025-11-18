use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tracing::{Level, Metadata};
use tracing_subscriber::{fmt, layer::SubscriberExt, registry::LookupSpan, EnvFilter, Layer};

/// Initialize hybrid logging (systemd journal + file for critical)
pub fn init_logging(module_name: &str, log_file: Option<PathBuf>) -> anyhow::Result<()> {
    // Create log directory if file logging is enabled
    if let Some(ref path) = log_file {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }

    // Build the registry
    let registry = tracing_subscriber::registry();

    // Add systemd journal layer or stdout fallback
    #[cfg(target_os = "linux")]
    let registry = {
        if systemd_journal_logger::connected_to_journal() {
            // Note: systemd logger will be used via tracing-journald if needed
            // For now, use stdout as it's simpler and works everywhere
            let stdout_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()));

            registry.with(stdout_layer)
        } else {
            // Fallback to stdout if journal not available
            let stdout_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()));

            registry.with(stdout_layer)
        }
    };

    // Add file layer for critical errors
    if let Some(log_path) = log_file {
        let file_layer = CriticalFileLayer::new(log_path.clone())?;
        let registry = registry.with(file_layer);
        tracing::subscriber::set_global_default(registry)?;
    } else {
        tracing::subscriber::set_global_default(registry)?;
    }

    tracing::info!("Initialized logging for {}", module_name);

    Ok(())
}

/// Custom layer that writes only WARN and ERROR to file
struct CriticalFileLayer {
    log_path: PathBuf,
}

impl CriticalFileLayer {
    fn new(log_path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { log_path })
    }

    fn should_log(&self, metadata: &Metadata) -> bool {
        matches!(*metadata.level(), Level::WARN | Level::ERROR)
    }

    fn write_log(&self, message: String) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", message)?;
        file.flush()?;

        Ok(())
    }
}

impl<S> Layer<S> for CriticalFileLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if self.should_log(event.metadata()) {
            let metadata = event.metadata();
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

            // Build log message
            let mut message = format!(
                "[{}] [{:5}] {}:",
                timestamp,
                metadata.level(),
                metadata.target()
            );

            // Extract message from event
            let mut visitor = MessageVisitor::new();
            event.record(&mut visitor);

            if let Some(ref msg) = visitor.message {
                message.push_str(&format!(" {}", msg));
            }

            // Write to file
            if let Err(e) = self.write_log(message) {
                eprintln!("Failed to write to log file: {}", e);
            }

            // Send notification for errors
            if *metadata.level() == Level::ERROR {
                if let Some(ref msg) = visitor.message {
                    let _ = send_error_notification(metadata.target(), msg);
                }
            }
        }
    }
}

/// Visitor to extract message from tracing event
struct MessageVisitor {
    message: Option<String>,
}

impl MessageVisitor {
    fn new() -> Self {
        Self { message: None }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

/// Send desktop notification for critical errors
fn send_error_notification(module: &str, message: &str) -> anyhow::Result<()> {
    use notify_rust::{Notification, Timeout};

    Notification::new()
        .summary(&format!("Waybar Error: {}", module))
        .body(message)
        .icon("dialog-error")
        .timeout(Timeout::Milliseconds(5000))
        .show()?;

    Ok(())
}

/// Helper macro for logging with context
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*);
    };
}
