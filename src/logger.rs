use worker::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Debug,
}

impl LogLevel {
    pub fn from_header(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            _ => LogLevel::Info,
        }
    }

    pub fn should_log_debug(&self) -> bool {
        matches!(self, LogLevel::Debug)
    }
}

/// Log at INFO level (always displayed)
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        worker::console_log!("[INFO] {}", format!($($arg)*))
    };
}

/// Log at DEBUG level (only when debug mode enabled)
#[macro_export]
macro_rules! log_debug {
    ($level:expr, $($arg:tt)*) => {
        if $level.should_log_debug() {
            worker::console_log!("[DEBUG] {}", format!($($arg)*))
        }
    };
}

/// Log errors (always displayed)
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        worker::console_log!("[ERROR] {}", format!($($arg)*))
    };
}
