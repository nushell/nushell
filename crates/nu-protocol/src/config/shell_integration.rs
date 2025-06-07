use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellIntegrationConfig {
    pub osc2: bool,
    pub osc7: bool,
    pub osc8: bool,
    pub osc9_9: bool,
    pub osc133: bool,
    pub osc633: bool,
    pub reset_application_mode: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ShellIntegrationConfig {
    fn default() -> Self {
        Self {
            osc2: true,
            osc7: !cfg!(windows),
            osc8: true,
            osc9_9: cfg!(windows),
            osc133: true,
            osc633: true,
            reset_application_mode: true,
        }
    }
}

impl UpdateFromValue for ShellIntegrationConfig {
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
                "osc2" => self.osc2.update(val, path, errors),
                "osc7" => self.osc7.update(val, path, errors),
                "osc8" => self.osc8.update(val, path, errors),
                "osc9_9" => self.osc9_9.update(val, path, errors),
                "osc133" => self.osc133.update(val, path, errors),
                "osc633" => self.osc633.update(val, path, errors),
                "reset_application_mode" => self.reset_application_mode.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
