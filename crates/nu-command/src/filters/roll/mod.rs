mod roll_;
mod roll_down;
mod roll_left;
mod roll_right;
mod roll_up;

use nu_protocol::{ShellError, Value};
pub use roll_::Roll;
pub use roll_down::RollDown;
pub use roll_left::RollLeft;
pub use roll_right::RollRight;
pub use roll_up::RollUp;

enum VerticalDirection {
    Up,
    Down,
}

fn vertical_rotate_value(
    value: Value,
    by: Option<usize>,
    direction: VerticalDirection,
) -> Result<Value, ShellError> {
    match value {
        Value::List { mut vals, span } => {
            let rotations = by.map(|n| n % vals.len()).unwrap_or(1);
            let values = vals.as_mut_slice();

            match direction {
                VerticalDirection::Up => values.rotate_left(rotations),
                VerticalDirection::Down => values.rotate_right(rotations),
            }

            Ok(Value::List {
                vals: values.to_owned(),
                span,
            })
        }
        _ => Err(ShellError::TypeMismatch("list".to_string(), value.span()?)),
    }
}

enum HorizontalDirection {
    Left,
    Right,
}

fn horizontal_rotate_value(
    value: Value,
    by: &Option<usize>,
    cells_only: bool,
    direction: &HorizontalDirection,
) -> Result<Value, ShellError> {
    match value {
        Value::Record {
            mut cols,
            mut vals,
            span,
        } => {
            let rotations = by.map(|n| n % vals.len()).unwrap_or(1);

            let columns = if cells_only {
                cols
            } else {
                let columns = cols.as_mut_slice();

                match direction {
                    HorizontalDirection::Right => columns.rotate_right(rotations),
                    HorizontalDirection::Left => columns.rotate_left(rotations),
                }

                columns.to_owned()
            };

            let values = vals.as_mut_slice();

            match direction {
                HorizontalDirection::Right => values.rotate_right(rotations),
                HorizontalDirection::Left => values.rotate_left(rotations),
            }

            Ok(Value::Record {
                cols: columns,
                vals: values.to_owned(),
                span,
            })
        }
        Value::List { vals, span } => {
            let values = vals
                .into_iter()
                .map(|value| horizontal_rotate_value(value, by, cells_only, direction))
                .collect::<Result<Vec<Value>, ShellError>>()?;

            Ok(Value::List { vals: values, span })
        }
        _ => Err(ShellError::TypeMismatch(
            "record".to_string(),
            value.span()?,
        )),
    }
}
