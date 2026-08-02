use nu_protocol::{ShellError, Value};
use nu_test_support::prelude::*;

#[test]
fn into_cell_path_with_negative_number_errors_out() -> Result {
    let val: Value = test().run("(-2) | into cell-path")?;
    let Value::Error { error, .. } = val else {
        panic!("expected Value::Error, got {val:?}");
    };

    match *error {
        ShellError::CantConvert {
            to_type, from_type, ..
        } => {
            assert_eq!(to_type, "cell path");
            assert_eq!(from_type, "negative number");
            Ok(())
        }
        err => Err(err.into()),
    }
}
