#[derive(Debug, Clone)]
pub struct Report {
    pub message: String,
    pub level: Severity,
    pub context1: String,
    pub context2: String,
    pub context3: String,
}

impl Report {
    pub fn new(message: String, level: Severity, ctx1: String, ctx2: String, ctx3: String) -> Self {
        Self {
            message,
            level,
            context1: ctx1,
            context2: ctx2,
            context3: ctx3,
        }
    }

    pub fn message(message: impl Into<String>, level: Severity) -> Self {
        Self::new(
            message.into(),
            level,
            String::new(),
            String::new(),
            String::new(),
        )
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::message(message.into(), Severity::Info)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::message(message.into(), Severity::Success)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::message(message.into(), Severity::Err)
    }
}

impl Default for Report {
    fn default() -> Self {
        Self::new(
            String::new(),
            Severity::Info,
            String::new(),
            String::new(),
            String::new(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Info,
    Success,
    Warn,
    Err,
}
