use super::{prelude::*, report_invalid_config_key, report_invalid_config_value};
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
                "metric" => self.metric.update(val, path, errors),
                "format" => self.format.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}
