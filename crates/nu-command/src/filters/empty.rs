use nu_engine::command_prelude::*;

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
            for column in &columns {
                let val = val.clone();
                match val.follow_cell_path(&column.members, false) {
                    Ok(Value::Nothing { .. }) => {}
                    Ok(_) => {
                        if negate {
                            return Ok(Value::bool(true, head).into_pipeline_data());
                        } else {
                            return Ok(Value::bool(false, head).into_pipeline_data());
                        }
                    }
                    Err(err) => return Err(err),
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
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ExternalStream { stdout, .. } => match stdout {
                Some(s) => {
                    let bytes = s.into_bytes();

                    match bytes {
                        Ok(s) => {
                            if negate {
                                Ok(Value::bool(!s.item.is_empty(), head).into_pipeline_data())
                            } else {
                                Ok(Value::bool(s.item.is_empty(), head).into_pipeline_data())
                            }
                        }
                        Err(err) => Err(err),
                    }
                }
                None => {
                    if negate {
                        Ok(Value::bool(false, head).into_pipeline_data())
                    } else {
                        Ok(Value::bool(true, head).into_pipeline_data())
                    }
                }
            },
            PipelineData::ListStream(s, ..) => {
                if negate {
                    Ok(Value::bool(s.count() != 0, head).into_pipeline_data())
                } else {
                    Ok(Value::bool(s.count() == 0, head).into_pipeline_data())
                }
            }
            PipelineData::Value(value, ..) => {
                if negate {
                    Ok(Value::bool(!value.is_empty(), head).into_pipeline_data())
                } else {
                    Ok(Value::bool(value.is_empty(), head).into_pipeline_data())
                }
            }
        }
    }
}
