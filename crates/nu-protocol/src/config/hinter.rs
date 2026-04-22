use super::prelude::*;
use crate as nu_protocol;
use crate::engine::Closure;

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct HinterConfig {
    pub closure: Option<Closure>,
}

impl UpdateFromValue for HinterConfig {
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
                "closure" => match val {
                    Value::Nothing { .. } => self.closure = None,
                    Value::Closure { val, .. } => self.closure = Some(val.as_ref().clone()),
                    _ => errors.type_mismatch(path, Type::custom("closure or nothing"), val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
