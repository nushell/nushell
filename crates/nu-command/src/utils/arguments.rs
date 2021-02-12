use indexmap::IndexSet;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, ColumnPath, UntaggedValue, Value};
use nu_value_ext::ValueExt;

/// Commands can be used in block form (passing a block) and
/// in the majority of cases we are also interested in accepting
/// column names along with it.
///
/// This aids with commands that take rest arguments
/// that need to be column names and an optional block as last
/// argument.
pub fn arguments(
    rest: &mut Vec<Value>,
) -> Result<(Vec<ColumnPath>, Option<Box<CapturedBlock>>), ShellError> {
    let last_argument = rest.pop();

    let mut columns: IndexSet<_> = rest.iter().collect();
    let mut column_paths = vec![];

    let mut default = None;

    for argument in columns.drain(..) {
        match &argument.value {
            UntaggedValue::Table(values) => {
                column_paths.extend(collect_as_column_paths(&values)?);
            }
            _ => {
                column_paths.push(argument.as_column_path()?.item);
            }
        }
    }

    match last_argument {
        Some(Value {
            value: UntaggedValue::Block(call),
            ..
        }) => default = Some(call),
        Some(other) => match &other.value {
            UntaggedValue::Table(values) => {
                column_paths.extend(collect_as_column_paths(&values)?);
            }
            _ => {
                column_paths.push(other.as_column_path()?.item);
            }
        },
        None => {}
    };

    Ok((column_paths, default))
}

fn collect_as_column_paths(values: &[Value]) -> Result<Vec<ColumnPath>, ShellError> {
    let mut out = vec![];

    for name in values {
        out.push(name.as_column_path()?.item);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::arguments;
    use nu_test_support::value::*;
    use nu_value_ext::ValueExt;

    #[test]
    fn arguments_test() -> Result<(), Box<dyn std::error::Error>> {
        // cmd name
        let arg1 = string("name");
        let expected = string("name").as_column_path()?.item;

        let (args, _) = arguments(&mut vec![arg1])?;

        assert_eq!(args[0], expected);

        Ok(())
    }

    #[test]
    fn arguments_test_2() -> Result<(), Box<dyn std::error::Error>> {
        // cmd name [type]
        let arg1 = string("name");
        let arg2 = table(&[string("type")]);

        let expected = vec![
            string("name").as_column_path()?.item,
            string("type").as_column_path()?.item,
        ];

        assert_eq!(arguments(&mut vec![arg1, arg2])?.0, expected);

        Ok(())
    }

    #[test]
    fn arguments_test_3() -> Result<(), Box<dyn std::error::Error>> {
        // cmd [name type]
        let arg1 = table(&vec![string("name"), string("type")]);

        let expected = vec![
            string("name").as_column_path()?.item,
            string("type").as_column_path()?.item,
        ];

        assert_eq!(arguments(&mut vec![arg1])?.0, expected);

        Ok(())
    }
}
