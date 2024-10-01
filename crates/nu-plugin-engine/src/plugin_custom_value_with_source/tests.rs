use std::sync::Arc;

use nu_plugin_protocol::test_util::{test_plugin_custom_value, TestCustomValue};
use nu_protocol::{
    engine::Closure, record, BlockId, CustomValue, IntoSpanned, ShellError, Span, Value, VarId,
};

use crate::{
    test_util::test_plugin_custom_value_with_source, PluginCustomValueWithSource, PluginSource,
};

use super::WithSource;

#[test]
fn add_source_in_at_root() -> Result<(), ShellError> {
    let mut val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValueWithSource::add_source_in(&mut val, &source)?;

    let custom_value = val.as_custom_value()?;
    let plugin_custom_value: &PluginCustomValueWithSource = custom_value
        .as_any()
        .downcast_ref()
        .expect("not PluginCustomValueWithSource");
    assert_eq!(
        Arc::as_ptr(&source),
        Arc::as_ptr(&plugin_custom_value.source)
    );
    Ok(())
}

fn check_record_custom_values(
    val: &Value,
    keys: &[&str],
    mut f: impl FnMut(&str, &dyn CustomValue) -> Result<(), ShellError>,
) -> Result<(), ShellError> {
    let record = val.as_record()?;
    for key in keys {
        let val = record
            .get(key)
            .unwrap_or_else(|| panic!("record does not contain '{key}'"));
        let custom_value = val
            .as_custom_value()
            .unwrap_or_else(|_| panic!("'{key}' not custom value"));
        f(key, custom_value)?;
    }
    Ok(())
}

#[test]
fn add_source_in_nested_record() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_record(record! {
        "foo" => orig_custom_val.clone(),
        "bar" => orig_custom_val.clone(),
    });
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValueWithSource::add_source_in(&mut val, &source)?;

    check_record_custom_values(&val, &["foo", "bar"], |key, custom_value| {
        let plugin_custom_value: &PluginCustomValueWithSource = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("'{key}' not PluginCustomValueWithSource"));
        assert_eq!(
            Arc::as_ptr(&source),
            Arc::as_ptr(&plugin_custom_value.source),
            "'{key}' source not set correctly"
        );
        Ok(())
    })
}

fn check_list_custom_values(
    val: &Value,
    indices: impl IntoIterator<Item = usize>,
    mut f: impl FnMut(usize, &dyn CustomValue) -> Result<(), ShellError>,
) -> Result<(), ShellError> {
    let list = val.as_list()?;
    for index in indices {
        let val = list
            .get(index)
            .unwrap_or_else(|| panic!("[{index}] not present in list"));
        let custom_value = val
            .as_custom_value()
            .unwrap_or_else(|_| panic!("[{index}] not custom value"));
        f(index, custom_value)?;
    }
    Ok(())
}

#[test]
fn add_source_in_nested_list() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_list(vec![orig_custom_val.clone(), orig_custom_val.clone()]);
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValueWithSource::add_source_in(&mut val, &source)?;

    check_list_custom_values(&val, 0..=1, |index, custom_value| {
        let plugin_custom_value: &PluginCustomValueWithSource = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not PluginCustomValueWithSource"));
        assert_eq!(
            Arc::as_ptr(&source),
            Arc::as_ptr(&plugin_custom_value.source),
            "[{index}] source not set correctly"
        );
        Ok(())
    })
}

fn check_closure_custom_values(
    val: &Value,
    indices: impl IntoIterator<Item = usize>,
    mut f: impl FnMut(usize, &dyn CustomValue) -> Result<(), ShellError>,
) -> Result<(), ShellError> {
    let closure = val.as_closure()?;
    for index in indices {
        let val = closure
            .captures
            .get(index)
            .unwrap_or_else(|| panic!("[{index}] not present in closure"));
        let custom_value = val
            .1
            .as_custom_value()
            .unwrap_or_else(|_| panic!("[{index}] not custom value"));
        f(index, custom_value)?;
    }
    Ok(())
}

#[test]
fn add_source_in_nested_closure() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_closure(Closure {
        block_id: BlockId::new(0),
        captures: vec![
            (VarId::new(0), orig_custom_val.clone()),
            (VarId::new(1), orig_custom_val.clone()),
        ],
    });
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValueWithSource::add_source_in(&mut val, &source)?;

    check_closure_custom_values(&val, 0..=1, |index, custom_value| {
        let plugin_custom_value: &PluginCustomValueWithSource = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not PluginCustomValueWithSource"));
        assert_eq!(
            Arc::as_ptr(&source),
            Arc::as_ptr(&plugin_custom_value.source),
            "[{index}] source not set correctly"
        );
        Ok(())
    })
}

#[test]
fn verify_source_error_message() -> Result<(), ShellError> {
    let span = Span::new(5, 7);
    let ok_val = test_plugin_custom_value_with_source();
    let native_val = TestCustomValue(32);
    let foreign_val =
        test_plugin_custom_value().with_source(Arc::new(PluginSource::new_fake("other")));
    let source = PluginSource::new_fake("test");

    PluginCustomValueWithSource::verify_source_of_custom_value(
        (&ok_val as &dyn CustomValue).into_spanned(span),
        &source,
    )
    .expect("ok_val should be verified ok");

    for (val, src_plugin) in [
        (&native_val as &dyn CustomValue, None),
        (&foreign_val as &dyn CustomValue, Some("other")),
    ] {
        let error = PluginCustomValueWithSource::verify_source_of_custom_value(
            val.into_spanned(span),
            &source,
        )
        .expect_err(&format!(
            "a custom value from {src_plugin:?} should result in an error"
        ));
        if let ShellError::CustomValueIncorrectForPlugin {
            name,
            span: err_span,
            dest_plugin,
            src_plugin: err_src_plugin,
        } = error
        {
            assert_eq!("TestCustomValue", name, "error.name from {src_plugin:?}");
            assert_eq!(span, err_span, "error.span from {src_plugin:?}");
            assert_eq!("test", dest_plugin, "error.dest_plugin from {src_plugin:?}");
            assert_eq!(src_plugin, err_src_plugin.as_deref(), "error.src_plugin");
        } else {
            panic!("the error returned should be CustomValueIncorrectForPlugin");
        }
    }

    Ok(())
}
