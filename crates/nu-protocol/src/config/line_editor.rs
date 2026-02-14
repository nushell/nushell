use super::prelude::*;
use crate as nu_protocol;
use crate::engine::Closure;

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct ExternalHinterConfig {
    pub enable: bool,
    pub closure: Option<Closure>,
}

impl Default for ExternalHinterConfig {
    fn default() -> Self {
        Self {
            enable: true,
            closure: None,
        }
    }
}

impl UpdateFromValue for ExternalHinterConfig {
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

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct ExternalLineEditorConfig {
    pub hinter: ExternalHinterConfig,
}

impl UpdateFromValue for ExternalLineEditorConfig {
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
                "hinter" => self.hinter.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[derive(Clone, Debug, Default, IntoValue, Serialize, Deserialize)]
pub struct LineEditorConfig {
    pub external: ExternalLineEditorConfig,
}

impl UpdateFromValue for LineEditorConfig {
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
                "external" => self.external.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
