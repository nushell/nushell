use nu_protocol::Value;

#[derive(Debug)]
pub enum LogLevel {}

#[derive(Debug)]
pub struct LogItem {
    level: LogLevel,
    value: Value,
}
