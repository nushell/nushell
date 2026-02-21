use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct ClipConfig {
    pub resident_mode: bool,
    pub default_raw: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ClipConfig {
    fn default() -> Self {
        Self {
            resident_mode: cfg!(target_os = "linux"),
            default_raw: false,
        }
    }
}

impl UpdateFromValue for ClipConfig {
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
                "resident_mode" => self.resident_mode.update(val, path, errors),
                "default_raw" => self.default_raw.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
