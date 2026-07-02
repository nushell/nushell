use std::collections::HashMap;

/// Abstraction over the process environment so that path-resolution logic can
/// be unit-tested without setting real environment variables.
///
/// # Seam
///
/// `SystemEnv` is the one adapter used in production.  `TestEnv` is the second
/// adapter that makes this a *real* seam — callers can inject arbitrary env
/// values during testing.
pub trait EnvAccess {
    fn var(&self, name: &str) -> Result<String, std::env::VarError>;
}

/// Reads from the real process environment via `std::env::var`.
pub struct SystemEnv;

impl EnvAccess for SystemEnv {
    fn var(&self, name: &str) -> Result<String, std::env::VarError> {
        std::env::var(name)
    }
}

/// Reads from an in-memory `HashMap` — useful in tests.
pub struct TestEnv {
    vars: HashMap<String, String>,
}

impl TestEnv {
    pub fn new(vars: HashMap<String, String>) -> Self {
        Self { vars }
    }
}

impl EnvAccess for TestEnv {
    fn var(&self, name: &str) -> Result<String, std::env::VarError> {
        self.vars
            .get(name)
            .cloned()
            .ok_or(std::env::VarError::NotPresent)
    }
}
