#[derive(Debug, Clone)]
pub struct Report {
    pub message: String,
    pub level: Severity,
    pub context: String,
    pub context2: String,
}

impl Report {
    pub fn new(message: String, level: Severity, context: String, context2: String) -> Self {
        Self {
            message,
            level,
            context,
            context2,
        }
    }

    pub fn message(message: impl Into<String>, level: Severity) -> Self {
        Self::new(message.into(), level, String::new(), String::new())
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message.into(), Severity::Info, String::new(), String::new())
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message.into(), Severity::Err, String::new(), String::new())
    }
}

impl Default for Report {
    fn default() -> Self {
        Self::new(String::new(), Severity::Info, String::new(), String::new())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Info,
    #[allow(dead_code)]
    Warn,
    Err,
}
