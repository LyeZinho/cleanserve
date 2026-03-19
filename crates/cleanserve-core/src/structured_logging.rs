//! Structured Logging Module
//!
//! Provides JSON-formatted logging for production environments.
//! Integrates with Datadog, ELK, and other log aggregation systems.

use serde::Serialize;
use std::collections::HashMap;
use tracing::Level;

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<tracing::Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::TRACE => LogLevel::Trace,
            Level::DEBUG => LogLevel::Debug,
            Level::INFO => LogLevel::Info,
            Level::WARN => LogLevel::Warn,
            Level::ERROR => LogLevel::Error,
        }
    }
}

/// Request context for logging
#[derive(Debug, Clone, Serialize)]
pub struct RequestLogContext {
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub duration_ms: u64,
    pub bytes_sent: usize,
    pub remote_addr: String,
}

impl RequestLogContext {
    pub fn new(method: &str, uri: &str, remote_addr: &str) -> Self {
        Self {
            method: method.to_string(),
            uri: uri.to_string(),
            status: 0,
            duration_ms: 0,
            bytes_sent: 0,
            remote_addr: remote_addr.to_string(),
        }
    }

    pub fn with_response(mut self, status: u16, duration_ms: u64, bytes_sent: usize) -> Self {
        self.status = status;
        self.duration_ms = duration_ms;
        self.bytes_sent = bytes_sent;
        self
    }
}

/// Application context for logging
#[derive(Debug, Clone, Serialize)]
pub struct AppLogContext {
    pub service: String,
    pub version: String,
    pub environment: String,
    pub hostname: String,
}

impl AppLogContext {
    pub fn new(service: &str, version: &str) -> Self {
        Self {
            service: service.to_string(),
            version: version.to_string(),
            environment: std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
            hostname: hostname(),
        }
    }
}

/// Structured log entry
#[derive(Debug, Serialize)]
pub struct StructuredLog {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestLogContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorContext>,
}

#[derive(Debug, Serialize)]
pub struct ErrorContext {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// Get hostname
fn hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Lightweight timestamp
fn timestamp() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let secs = now.as_secs();
    let millis = now.subsec_millis();

    format!("{}.{:03}Z", secs, millis)
}

/// Initialize structured JSON logging for production
pub fn init_json_logging(service: &str, version: &str, _level: Level) {
    // Set up JSON format subscriber
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(_level)
        .json()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    tracing::info!(
        service = service,
        version = version,
        environment = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string()),
        "Logging initialized"
    );
}

/// Initialize development logging (human-readable)
pub fn init_dev_logging() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

/// Log a request (convenience function)
pub fn log_request(ctx: &RequestLogContext) {
    tracing::info!(
        method = %ctx.method,
        uri = %ctx.uri,
        status = ctx.status,
        duration_ms = ctx.duration_ms,
        bytes_sent = ctx.bytes_sent,
        remote_addr = %ctx.remote_addr,
        "HTTP Request"
    );
}

/// Log an error with context
pub fn log_error(
    message: &str,
    error: Option<&(dyn std::error::Error + 'static)>,
    custom: Option<HashMap<String, serde_json::Value>>,
) {
    let error_ctx = error.map(|e| ErrorContext {
        message: e.to_string(),
        stack: None, // backtrace() requires RUST_BACKTRACE
    });

    let log = StructuredLog {
        timestamp: timestamp(),
        level: "error".to_string(),
        message: message.to_string(),
        target: "cleanserve".to_string(),
        line: None,
        file: None,
        request: None,
        error: error_ctx,
    };

    // Write directly to stdout in JSON format
    if let Ok(json) = serde_json::to_string(&log) {
        eprintln!("{}", json);
    }

    // Also use tracing for structured output
    tracing::error!(error = ?error, custom = ?custom, "{}", message);
}
