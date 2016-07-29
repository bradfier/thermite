extern crate log;
extern crate time;

use log::{LogRecord, LogLevel, LogMetadata, LogLevelFilter, SetLoggerError};

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            let now = time::now();
            let nowfmt = now.rfc3339();
            println!("{} {}\t{}", nowfmt, record.level(), record.args());
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(Logger)
    })
}
