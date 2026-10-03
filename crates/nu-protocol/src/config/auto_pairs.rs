use super::prelude::*;
use crate as nu_protocol;

/// Configuration for automatic pair insertion in the line editor (`$env.config.auto_pairs`).
#[derive(Clone, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPairsConfig {
    pub enable: bool,
}

impl UpdateFromValue for AutoPairsConfig {
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
                "enable" => self.enable.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
