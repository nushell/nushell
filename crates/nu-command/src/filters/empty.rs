use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
use std::io::Read;

pub fn empty(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    negate: bool,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    if !columns.is_empty() {
        for val in input {
            let val = val.unwrap_error()?;
            for column in &columns {
                if !val.follow_cell_path(&column.members)?.is_nothing() {
                    return Ok(Value::bool(negate, head).into_pipeline_data());
                }
            }
        }

        if negate {
            Ok(Value::bool(false, head).into_pipeline_data())
        } else {
            Ok(Value::bool(true, head).into_pipeline_data())
        }
    } else {
        match input {
            PipelineData::Empty => Ok(Value::bool(!negate, head).into_pipeline_data()),
            PipelineData::ByteStream(stream, ..) => {
                let span = stream.span();
                match stream.reader() {
                    Some(reader) => {
                        let is_empty = reader
                            .bytes()
                            .next()
                            .transpose()
                            .map_err(|err| IoError::new(err, span, None))?
                            .is_none();
                        if negate {
                            Ok(Value::bool(!is_empty, head).into_pipeline_data())
                        } else {
                            Ok(Value::bool(is_empty, head).into_pipeline_data())
                        }
                    }
                    None => {
                        if negate {
                            Ok(Value::bool(false, head).into_pipeline_data())
                        } else {
                            Ok(Value::bool(true, head).into_pipeline_data())
                        }
                    }
                }
            }
            PipelineData::ListStream(s, ..) => {
                let empty = match s.into_iter().next() {
                    None => true,
                    Some(val) => {
                        val.unwrap_error()?;
                        false
                    }
                };
                if negate {
                    Ok(Value::bool(!empty, head).into_pipeline_data())
                } else {
                    Ok(Value::bool(empty, head).into_pipeline_data())
                }
            }
            PipelineData::Value(value, ..) => {
                match &value {
                    Value::Error { error, .. } => return Err(*error.clone()),
                    Value::List { vals, .. } => {
                        for val in vals {
                            if let Value::Error { error, .. } = val {
                                return Err(*error.clone());
                            }
                        }
                    }
                    _ => {}
                }
                if negate {
                    Ok(Value::bool(!value.is_empty(), head).into_pipeline_data())
                } else {
                    Ok(Value::bool(value.is_empty(), head).into_pipeline_data())
                }
            }
        }
    }
}
