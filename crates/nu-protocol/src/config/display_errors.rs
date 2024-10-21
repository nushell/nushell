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

impl UpdateFromValue for DisplayErrors {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "exit_code" => self.exit_code.update(val, path, errors),
                "termination_signal" => self.termination_signal.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
