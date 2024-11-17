use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesizeConfig {
    pub metric: bool,
    pub format: String,
}

impl Default for FilesizeConfig {
    fn default() -> Self {
        Self {
            metric: false,
            format: "auto".into(),
        }
    }
}

impl UpdateFromValue for FilesizeConfig {
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
                "metric" => self.metric.update(val, path, errors),
                "format" => self.format.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
