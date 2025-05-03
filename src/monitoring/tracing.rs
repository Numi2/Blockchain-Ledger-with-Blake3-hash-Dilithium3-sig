use tracing::{Level, subscriber::set_global_default, Subscriber};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, fmt, Registry};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Log level
    pub level: String,
    
    /// Enable file logging
    pub file_logging: bool,
    
    /// Log directory
    pub log_dir: String,
    
    /// Log file prefix
    pub log_file_prefix: String,
    
    /// Log rolling period
    pub log_rolling: String,
    
    /// Enable JSON formatting
    pub json_format: bool,
    
    /// Enable console logging
    pub console_logging: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_logging: true,
            log_dir: "logs".to_string(),
            log_file_prefix: "world-ledger".to_string(),
            log_rolling: "daily".to_string(),
            json_format: false,
            console_logging: true,
        }
    }
}

/// Initialize the tracing system
pub fn init_tracing(config: &TracingConfig) -> Result<(), String> {
    // Convert log level string to Level
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .map_err(|e| format!("Invalid log level: {}", e))?;
    
    // Set up log capture
    LogTracer::init()
        .map_err(|e| format!("Failed to initialize log tracer: {}", e))?;
    
    // Create a registry that will hold the layers
    let mut layers = Vec::new();
    
    // Add console logging if enabled
    if config.console_logging {
        let fmt_layer = fmt::Layer::default();
        layers.push(Box::new(fmt_layer));
    }
    
    // Add file logging if enabled
    if config.file_logging {
        // Create log directory if it doesn't exist
        if !Path::new(&config.log_dir).exists() {
            fs::create_dir_all(&config.log_dir)
                .map_err(|e| format!("Failed to create log directory: {}", e))?;
        }
        
        // Set up rotation
        let rotation = match config.log_rolling.as_str() {
            "hourly" => Rotation::HOURLY,
            "daily" => Rotation::DAILY,
            "never" => Rotation::NEVER,
            _ => return Err(format!("Invalid log rotation: {}", config.log_rolling)),
        };
        
        // Create file appender
        let file_appender = RollingFileAppender::new(
            rotation,
            &config.log_dir,
            &config.log_file_prefix,
        );
        
        // Create a layer for file logging
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        let file_layer = fmt::Layer::default().with_writer(non_blocking);
        
        // Add layer to registry
        layers.push(Box::new(file_layer));
    }
    
    // Create the subscriber
    let subscriber = Registry::default()
        .with(filter_layer);
    
    // Set the global default
    set_global_default(subscriber)
        .map_err(|e| format!("Failed to set global default subscriber: {}", e))?;
    
    Ok(())
}

/// A span represents a period of time in the execution of a program
#[derive(Debug, Clone)]
pub struct TracingSpan {
    /// Span name
    pub name: String,
    
    /// Is the span active
    pub active: bool,
}

impl TracingSpan {
    /// Create a new tracing span
    pub fn new(name: &str) -> Self {
        let span = tracing::info_span!(name);
        let _entered = span.enter();
        
        Self {
            name: name.to_string(),
            active: true,
        }
    }
    
    /// End the span
    pub fn end(self) {
        // Span will be dropped when this function returns
        drop(self);
    }
}

/// Record an event at info level
pub fn record_info(message: &str) {
    tracing::info!("{}", message);
}

/// Record an event at error level
pub fn record_error(message: &str) {
    tracing::error!("{}", message);
}

/// Record an event at debug level
pub fn record_debug(message: &str) {
    tracing::debug!("{}", message);
}

/// Record an event at warn level
pub fn record_warn(message: &str) {
    tracing::warn!("{}", message);
}

/// Record an event at trace level
pub fn record_trace(message: &str) {
    tracing::trace!("{}", message);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.file_logging);
        assert_eq!(config.log_dir, "logs");
    }
} 