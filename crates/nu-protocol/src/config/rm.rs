use super::{prelude::*, report_invalid_config_key, report_invalid_config_value};
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
        errors: &mut Vec<ShellError>,
    ) {
        let span = value.span();
        let Value::Record { val: record, .. } = value else {
            report_invalid_config_value("should be a record", span, path, errors);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
            match col.as_str() {
                "always_trash" => self.always_trash.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}
