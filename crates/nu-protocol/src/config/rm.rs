use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmConfig {
    pub always_trash: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for RmConfig {
    fn default() -> Self {
        Self {
            always_trash: false,
        }
    }
}

impl UpdateFromValue for RmConfig {
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
                "always_trash" => self.always_trash.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
