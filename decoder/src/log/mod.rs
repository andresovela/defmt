//! This module provides interoperability utilities between [`defmt`] and the [`log`] crate.
//!
//! If you are implementing a custom defmt decoding tool, this module can make it easier to
//! integrate it with logs produced with the [`log`] crate.
//!
//! [`log`]: https://crates.io/crates/log
//! [`defmt`]: https://crates.io/crates/defmt

mod format;
mod json_logger;
mod stdout_logger;

use log::{Level, LevelFilter, Metadata, Record};
use serde::{Deserialize, Serialize};

use std::fmt;

use self::{json_logger::JsonLogger, stdout_logger::StdoutLogger};
use crate::Frame;

const DEFMT_TARGET_MARKER: &str = "defmt@";

/// Logs a defmt frame using the `log` facade.
pub fn log_defmt(
    frame: &Frame<'_>,
    file: Option<&str>,
    line: Option<u32>,
    module_path: Option<&str>,
) {
    let timestamp = frame
        .display_timestamp()
        .map(|ts| ts.to_string())
        .unwrap_or_default();

    let level = frame.level().map(|level| match level {
        crate::Level::Trace => Level::Trace,
        crate::Level::Debug => Level::Debug,
        crate::Level::Info => Level::Info,
        crate::Level::Warn => Level::Warn,
        crate::Level::Error => Level::Error,
    });

    let target = format!(
        "{}{}",
        DEFMT_TARGET_MARKER,
        serde_json::to_value(Payload { timestamp, level }).unwrap()
    );

    log::logger().log(
        &Record::builder()
            .args(format_args!("{}", frame.display_message()))
            // .level(level) // no need to set the level, since it is transferred via payload
            .target(&target)
            .module_path(module_path)
            .file(file)
            .line(line)
            .build(),
    );
}

/// Determines whether `metadata` belongs to a log record produced by [`log_defmt`].
pub fn is_defmt_frame(metadata: &Metadata) -> bool {
    metadata.target().starts_with(DEFMT_TARGET_MARKER)
}

/// A `log` record representing a defmt log frame.
pub struct DefmtRecord<'a> {
    log_record: &'a Record<'a>,
    payload: Payload,
}

#[derive(Deserialize, Serialize)]
struct Payload {
    level: Option<Level>,
    timestamp: String,
}

impl<'a> DefmtRecord<'a> {
    /// If `record` was produced by [`log_defmt`], returns the corresponding `DefmtRecord`.
    pub fn new(log_record: &'a Record<'a>) -> Option<Self> {
        let target = log_record.metadata().target();
        target
            .strip_prefix(DEFMT_TARGET_MARKER)
            .map(|payload| Self {
                log_record,
                payload: serde_json::from_str(payload).expect("malformed 'payload'"),
            })
    }

    /// Returns the formatted defmt timestamp.
    pub fn timestamp(&self) -> &str {
        self.payload.timestamp.as_str()
    }

    pub fn level(&self) -> Option<Level> {
        self.payload.level
    }

    pub fn args(&self) -> &fmt::Arguments<'a> {
        self.log_record.args()
    }

    pub fn module_path(&self) -> Option<&'a str> {
        self.log_record.module_path()
    }

    pub fn file(&self) -> Option<&'a str> {
        self.log_record.file()
    }

    pub fn line(&self) -> Option<u32> {
        self.log_record.line()
    }
}

/// Initializes a `log` sink that handles defmt frames.
///
/// Defmt frames will be printed to stdout, other logs to stderr.
///
/// The caller has to provide a `should_log` closure that determines whether a log record should be
/// printed.
///
/// An optional `log_format` string can be provided to format the way
/// logs are printed. A format string could look as follows:
/// "{t} [{L}] Location<{f}:{l}> {s}"
///
/// The arguments between curly braces are placeholders for log metadata.
/// The following arguments are supported:
/// - {t} : log timestamp
/// - {f} : file name (e.g. "main.rs")
/// - {F} : file path (e.g. "home/app/main.rs")
/// - {m} : module path (e.g. "foo::bar::some_function")
/// - {l} : line number
/// - {L} : log level (e.g. "INFO", "DEBUG", etc)
/// - {s} : the actual log
///
/// For example, with the log format shown above, a log would look like this:
/// "23124 [INFO] Location<main.rs:23> Hello, world!"
pub fn init_logger(
    log_format: Option<&str>,
    json: bool,
    should_log: impl Fn(&Metadata) -> bool + Sync + Send + 'static,
) {
    log::set_boxed_logger(match json {
        false => StdoutLogger::new(log_format, should_log),
        true => {
            JsonLogger::print_schema_version();
            JsonLogger::new(log_format, should_log)
        }
    })
    .unwrap();
    log::set_max_level(LevelFilter::Trace);
}
