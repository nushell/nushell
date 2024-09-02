use super::{prelude::*, report_invalid_config_key, report_invalid_config_value};
use crate as nu_protocol;

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct DatetimeFormatConfig {
    pub normal: Option<String>,
    pub table: Option<String>,
}

impl UpdateFromValue for DatetimeFormatConfig {
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
                "normal" => match val {
                    Value::Nothing { .. } => self.normal = None,
                    Value::String { val, .. } => self.normal = Some(val.clone()),
                    _ => report_invalid_config_value(
                        "should be null or a string",
                        span,
                        path,
                        errors,
                    ),
                },
                "table" => match val {
                    Value::Nothing { .. } => self.table = None,
                    Value::String { val, .. } => self.table = Some(val.clone()),
                    _ => report_invalid_config_value(
                        "should be null or a string",
                        span,
                        path,
                        errors,
                    ),
                },
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}
