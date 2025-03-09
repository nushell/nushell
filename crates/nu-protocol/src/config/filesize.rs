use super::prelude::*;
use crate::{Filesize, FilesizeFormatter, FilesizeUnitFormat, FormattedFilesize};
use nu_utils::get_system_locale;

impl IntoValue for FilesizeUnitFormat {
    fn into_value(self, span: Span) -> Value {
        self.as_str().into_value(span)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesizeConfig {
    pub unit: FilesizeUnitFormat,
    pub show_unit: bool,
    pub precision: Option<usize>,
}

impl FilesizeConfig {
    pub fn formatter(&self) -> FilesizeFormatter {
        FilesizeFormatter::new()
            .unit(self.unit)
            .show_unit(self.show_unit)
            .precision(self.precision)
            .locale(get_system_locale()) // TODO: cache this somewhere or pass in as argument
    }

    pub fn format(&self, filesize: Filesize) -> FormattedFilesize {
        self.formatter().format(filesize)
    }
}

impl Default for FilesizeConfig {
    fn default() -> Self {
        Self {
            unit: FilesizeUnitFormat::Metric,
            show_unit: true,
            precision: Some(1),
        }
    }
}

impl From<FilesizeConfig> for FilesizeFormatter {
    fn from(config: FilesizeConfig) -> Self {
        config.formatter()
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
                "unit" => {
                    if let Ok(str) = val.as_str() {
                        match str.parse() {
                            Ok(unit) => self.unit = unit,
                            Err(_) => errors.invalid_value(path, "'metric', 'binary', 'B', 'kB', 'MB', 'GB', 'TB', 'PB', 'EB', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', or 'EiB'", val),
                        }
                    } else {
                        errors.type_mismatch(path, Type::String, val)
                    }
                }
                "show_unit" => self.show_unit.update(val, path, errors),
                "precision" => match *val {
                    Value::Nothing { .. } => self.precision = None,
                    Value::Int { val, .. } if val >= 0 => self.precision = Some(val as usize),
                    Value::Int { .. } => errors.invalid_value(path, "a non-negative integer", val),
                    _ => errors.type_mismatch(path, Type::custom("int or nothing"), val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

impl IntoValue for FilesizeConfig {
    fn into_value(self, span: Span) -> Value {
        record! {
            "unit" => self.unit.into_value(span),
            "show_unit" => self.show_unit.into_value(span),
            "precision" => self.precision.map(|x| x as i64).into_value(span),
        }
        .into_value(span)
    }
}
