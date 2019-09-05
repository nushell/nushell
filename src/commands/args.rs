use crate::data::Value;

#[derive(Debug)]
pub enum LogLevel {}

#[derive(Debug)]
pub struct LogItem {
    level: LogLevel,
    value: Value,
}
