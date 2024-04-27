use nu_protocol::{CustomValue, IntoSpanned, ShellError, Spanned, Value};

/// Do something with all [`CustomValue`]s recursively within a `Value`. This is not limited to
/// plugin custom values.
///
/// `LazyRecord`s will be collected to plain values for completeness.
pub fn with_custom_values_in<E>(
    value: &mut Value,
    mut f: impl FnMut(Spanned<&mut (dyn CustomValue + '_)>) -> Result<(), E>,
) -> Result<(), E>
where
    E: From<ShellError>,
{
    value.recurse_mut(&mut |value| {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => {
                // Operate on a CustomValue.
                f(val.as_mut().into_spanned(span))
            }
            _ => Ok(()),
        }
    })
}

#[test]
fn find_custom_values() {
    use crate::protocol::test_util::test_plugin_custom_value;
    use nu_protocol::{engine::Closure, record};

    let mut cv = Value::test_custom_value(Box::new(test_plugin_custom_value()));

    let mut value = Value::test_record(record! {
        "bare" => cv.clone(),
        "list" => Value::test_list(vec![
            cv.clone(),
            Value::test_int(4),
        ]),
        "closure" => Value::test_closure(
            Closure {
                block_id: 0,
                captures: vec![(0, cv.clone()), (1, Value::test_string("foo"))]
            }
        ),
    });

    // Do with_custom_values_in, and count the number of custom values found
    let mut found = 0;
    with_custom_values_in::<ShellError>(&mut value, |_| {
        found += 1;
        Ok(())
    })
    .expect("error");
    assert_eq!(3, found, "found in value");

    // Try it on bare custom value too
    found = 0;
    with_custom_values_in::<ShellError>(&mut cv, |_| {
        found += 1;
        Ok(())
    })
    .expect("error");
    assert_eq!(1, found, "bare custom value didn't work");
}
