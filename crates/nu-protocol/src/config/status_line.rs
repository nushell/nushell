use super::prelude::*;
use crate as nu_protocol;

/// What the REPL shows on the line under the one being edited. `enable`
/// switches all of it off at once.
#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusLineConfig {
    /// Draw the status line at all.
    pub enable: bool,
    /// The first parse or type error in the line.
    pub errors: bool,
    /// The signature of the command under the cursor.
    pub signature: bool,
    /// The type the pipeline at the cursor outputs.
    #[nu_value(rename = "type")]
    #[serde(rename = "type")]
    pub output_type: bool,
    /// The value of a line built only from const commands.
    pub preview: bool,
}

impl Default for StatusLineConfig {
    fn default() -> Self {
        Self {
            enable: true,
            errors: true,
            signature: true,
            output_type: true,
            preview: true,
        }
    }
}

impl UpdateFromValue for StatusLineConfig {
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
                "errors" => self.errors.update(val, path, errors),
                "signature" => self.signature.update(val, path, errors),
                "type" => self.output_type.update(val, path, errors),
                "preview" => self.preview.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
