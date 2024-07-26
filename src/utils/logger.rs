use chrono::{DateTime, Local, Utc};

pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

pub fn log(level: LogLevel, message: &str) {
    let now: DateTime<Utc> = Utc::now();
    let local_now = now.with_timezone(&Local);

    let timestamp = local_now.format("%Y-%m-%d %H:%M:%S");

    let log_level_str = match level {
        LogLevel::Info => "\x1b[32mINFO\x1b[0m",
        LogLevel::Warning => "\x1b[33mWARNING\x1b[0m",
        LogLevel::Error => "\x1b[31mERROR\x1b[0m",
        LogLevel::Debug => "\x1b[34mDEBUG\x1b[0m",
    };

    println!("[{}] {} {}", timestamp, log_level_str, message);
}

mod logger {
    #![macro_use]

    #[macro_export]
    macro_rules! info {
    ($($arg:tt)*) => {
        $crate::utils::logger::log($crate::utils::logger::LogLevel::Info, &format!($($arg)*));
    };
}

    #[macro_export]
    macro_rules! warning {
    ($($arg:tt)*) => {
        $crate::utils::logger::log($crate::utils::logger::LogLevel::Warning, &format!($($arg)*));
    };
}

    #[macro_export]
    macro_rules! error {
    ($($arg:tt)*) => {
        $crate::utils::logger::log($crate::utils::logger::LogLevel::Error, &format!($($arg)*));
    };
}

    #[macro_export]
    macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::utils::logger::log($crate::utils::logger::LogLevel::Debug, &format!($($arg)*));
    };
}

}
