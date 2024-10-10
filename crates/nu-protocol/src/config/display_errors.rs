use super::prelude::*;
use crate as nu_protocol;
use crate::ShellError;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayErrors {
    pub exit_code: bool,
    pub termination_signal: bool,
}

impl DisplayErrors {
    pub fn should_show(&self, error: &ShellError) -> bool {
        match error {
            ShellError::NonZeroExitCode { .. } => self.exit_code,
            #[cfg(unix)]
            ShellError::TerminatedBySignal { .. } => self.termination_signal,
            _ => true,
        }
    }
}

impl Default for DisplayErrors {
    fn default() -> Self {
        Self {
            exit_code: false,
            termination_signal: true,
        }
    }
}
