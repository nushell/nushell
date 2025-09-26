use super::prelude::*;

pub const DEFAULT_PIPELINE_BUFFER_SIZE: usize = 8192;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineConfig {
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
}

const fn default_buffer_size() -> usize {
    DEFAULT_PIPELINE_BUFFER_SIZE
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_PIPELINE_BUFFER_SIZE,
        }
    }
}

impl PipelineConfig {
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

impl IntoValue for PipelineConfig {
    fn into_value(self, span: Span) -> Value {
        record! {
            "buffer_size" => Value::int(
                i64::try_from(self.buffer_size).unwrap_or(i64::MAX),
                span
            )
        }
        .into_value(span)
    }
}

impl UpdateFromValue for PipelineConfig {
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
                "buffer_size" => match val.as_int() {
                    Ok(v) if v > 0 => {
                        if let Ok(v) = usize::try_from(v) {
                            self.buffer_size = v;
                        } else {
                            errors.invalid_value(path, "a usize value", val);
                        }
                    }
                    Ok(_) => errors.invalid_value(path, "an int greater than 0", val),
                    Err(_) => errors.type_mismatch(path, Type::Int, val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
