use super::{config_update_string_enum, prelude::*};
use crate::{DisplayFilesize, Filesize, FilesizeUnit};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesizeFormatUnit {
    Metric,
    Binary,
    Unit(FilesizeUnit),
}

impl From<FilesizeUnit> for FilesizeFormatUnit {
    fn from(unit: FilesizeUnit) -> Self {
        Self::Unit(unit)
    }
}

impl FromStr for FilesizeFormatUnit {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "metric" => Ok(Self::Metric),
            "binary" => Ok(Self::Binary),
            _ => {
                if let Ok(unit) = s.parse() {
                    Ok(Self::Unit(unit))
                } else {
                    Err("'metric', 'binary', 'B', 'kB', 'MB', 'GB', 'TB', 'PB', 'EB', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', or 'EiB'")
                }
            }
        }
    }
}

impl IntoValue for FilesizeFormatUnit {
    fn into_value(self, span: Span) -> Value {
        match self {
            FilesizeFormatUnit::Metric => "metric",
            FilesizeFormatUnit::Binary => "binary",
            FilesizeFormatUnit::Unit(unit) => unit.as_str(),
        }
        .into_value(span)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesizeConfig {
    pub unit: FilesizeFormatUnit,
    pub precision: Option<usize>,
}

impl FilesizeConfig {
    pub fn display(&self, filesize: Filesize) -> DisplayFilesize {
        let unit = match self.unit {
            FilesizeFormatUnit::Metric => filesize.largest_metric_unit(),
            FilesizeFormatUnit::Binary => filesize.largest_binary_unit(),
            FilesizeFormatUnit::Unit(unit) => unit,
        };
        filesize.display(unit).precision(self.precision)
    }
}

impl Default for FilesizeConfig {
    fn default() -> Self {
        Self {
            unit: FilesizeFormatUnit::Metric,
            precision: Some(1),
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
                "unit" => config_update_string_enum(&mut self.unit, val, path, errors),
                "precision" => match *val {
                    Value::Nothing { .. } => self.precision = None,
                    Value::Int { val, .. } if val >= 0 => self.precision = Some(val as usize),
                    Value::Int { .. } => errors.invalid_value(path, "a non-negative integer", val),
                    _ => errors.type_mismatch(path, Type::custom("int or nothing"), val),
                },
                "format" | "metric" => {
                    // TODO: remove after next release
                    errors.deprecated_option(path, "set $env.config.filesize.unit", val.span())
                }
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

impl IntoValue for FilesizeConfig {
    fn into_value(self, span: Span) -> Value {
        record! {
            "unit" => self.unit.into_value(span),
            "precision" => self.precision.map(|x| x as i64).into_value(span),
        }
        .into_value(span)
    }
}
