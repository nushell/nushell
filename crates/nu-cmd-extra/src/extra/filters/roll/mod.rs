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
    let span = value.span();
    match value {
        Value::List { mut vals, .. } => {
            let rotations = by.map(|n| n % vals.len()).unwrap_or(1);
            let values = vals.as_mut_slice();

            match direction {
                VerticalDirection::Up => values.rotate_left(rotations),
                VerticalDirection::Down => values.rotate_right(rotations),
            }

            Ok(Value::list(values.to_owned(), span))
        }
        _ => Err(ShellError::TypeMismatch {
            err_message: "list".to_string(),
            span: value.span(),
        }),
    }
}

enum HorizontalDirection {
    Left,
    Right,
}

fn horizontal_rotate_value(
    value: Value,
    by: Option<usize>,
    cells_only: bool,
    direction: &HorizontalDirection,
) -> Result<Value, ShellError> {
    let span = value.span();
    match value {
        Value::Record {
            val: mut record, ..
        } => {
            let rotations = by.map(|n| n % record.len()).unwrap_or(1);

            if !cells_only {
                match direction {
                    HorizontalDirection::Right => record.cols.rotate_right(rotations),
                    HorizontalDirection::Left => record.cols.rotate_left(rotations),
                }
            };

            match direction {
                HorizontalDirection::Right => record.vals.rotate_right(rotations),
                HorizontalDirection::Left => record.vals.rotate_left(rotations),
            }

            Ok(Value::record(record, span))
        }
        Value::List { vals, .. } => {
            let values = vals
                .into_iter()
                .map(|value| horizontal_rotate_value(value, by, cells_only, direction))
                .collect::<Result<Vec<Value>, ShellError>>()?;

            Ok(Value::list(values, span))
        }
        _ => Err(ShellError::TypeMismatch {
            err_message: "record".to_string(),
            span: value.span(),
        }),
    }
}
