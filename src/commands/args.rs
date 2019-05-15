use crate::object::Value;

#[allow(unused)]
#[derive(Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

#[allow(unused)]
#[derive(Debug)]
pub struct LogItem {
    level: LogLevel,
    value: Value,
}
