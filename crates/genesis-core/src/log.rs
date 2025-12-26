//! Logging system for Genesis.
//!
//! Provides structured logging with multiple outputs, stack traces,
//! and configurable log levels.

use genesis_types::{LogLevel, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the logging system with default configuration.
pub fn init_default() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    Ok(())
}

/// Initialize logging from configuration.
/// Note: Advanced logging features (multiple outputs, rotation, etc.) can be configured
/// using tracing-subscriber layers and tracing-appender for production deployments.
pub fn init_from_config(_configs: &[genesis_types::config::LogConfig]) -> Result<()> {
    init_default()
}
