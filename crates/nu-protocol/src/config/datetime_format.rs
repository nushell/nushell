use super::prelude::*;
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
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "normal" => match val {
                    Value::Nothing { .. } => self.normal = None,
                    Value::String { val, .. } => self.normal = Some(val.clone()),
                    _ => errors.type_mismatch(path, Type::custom("string or nothing"), val),
                },
                "table" => match val {
                    Value::Nothing { .. } => self.table = None,
                    Value::String { val, .. } => self.table = Some(val.clone()),
                    _ => errors.type_mismatch(path, Type::custom("string or nothing"), val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
