use super::PluginCustomValue;
use crate::{
    plugin::PluginSource,
    protocol::test_util::{
        expected_test_custom_value, test_plugin_custom_value, test_plugin_custom_value_with_source,
        TestCustomValue,
    },
};
use nu_protocol::{engine::Closure, record, CustomValue, IntoSpanned, ShellError, Span, Value};
use std::sync::Arc;

#[test]
fn serialize_deserialize() -> Result<(), ShellError> {
    let original_value = TestCustomValue(32);
    let span = Span::test_data();
    let serialized = PluginCustomValue::serialize_from_custom_value(&original_value, span)?;
    assert_eq!(original_value.type_name(), serialized.name());
    assert!(serialized.source.is_none());
    let deserialized = serialized.deserialize_to_custom_value(span)?;
    let downcasted = deserialized
        .as_any()
        .downcast_ref::<TestCustomValue>()
        .expect("failed to downcast: not TestCustomValue");
    assert_eq!(original_value, *downcasted);
    Ok(())
}

#[test]
fn expected_serialize_output() -> Result<(), ShellError> {
    let original_value = expected_test_custom_value();
    let span = Span::test_data();
    let serialized = PluginCustomValue::serialize_from_custom_value(&original_value, span)?;
    assert_eq!(
        test_plugin_custom_value().data(),
        serialized.data(),
        "The bincode configuration is probably different from what we expected. \
            Fix test_plugin_custom_value() to match it"
    );
    Ok(())
}

#[test]
fn add_source_in_at_root() -> Result<(), ShellError> {
    let mut val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source_in(&mut val, &source)?;

    let custom_value = val.as_custom_value()?;
    let plugin_custom_value: &PluginCustomValue = custom_value
        .as_any()
        .downcast_ref()
        .expect("not PluginCustomValue");
    assert_eq!(
        Some(Arc::as_ptr(&source)),
        plugin_custom_value.source.as_ref().map(Arc::as_ptr)
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
    PluginCustomValue::add_source_in(&mut val, &source)?;

    check_record_custom_values(&val, &["foo", "bar"], |key, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("'{key}' not PluginCustomValue"));
        assert_eq!(
            Some(Arc::as_ptr(&source)),
            plugin_custom_value.source.as_ref().map(Arc::as_ptr),
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
    PluginCustomValue::add_source_in(&mut val, &source)?;

    check_list_custom_values(&val, 0..=1, |index, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not PluginCustomValue"));
        assert_eq!(
            Some(Arc::as_ptr(&source)),
            plugin_custom_value.source.as_ref().map(Arc::as_ptr),
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
        block_id: 0,
        captures: vec![(0, orig_custom_val.clone()), (1, orig_custom_val.clone())],
    });
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source_in(&mut val, &source)?;

    check_closure_custom_values(&val, 0..=1, |index, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not PluginCustomValue"));
        assert_eq!(
            Some(Arc::as_ptr(&source)),
            plugin_custom_value.source.as_ref().map(Arc::as_ptr),
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
    let foreign_val = {
        let mut val = test_plugin_custom_value();
        val.source = Some(Arc::new(PluginSource::new_fake("other")));
        val
    };
    let source = PluginSource::new_fake("test");

    PluginCustomValue::verify_source((&ok_val as &dyn CustomValue).into_spanned(span), &source)
        .expect("ok_val should be verified ok");

    for (val, src_plugin) in [
        (&native_val as &dyn CustomValue, None),
        (&foreign_val as &dyn CustomValue, Some("other")),
    ] {
        let error = PluginCustomValue::verify_source(val.into_spanned(span), &source).expect_err(
            &format!("a custom value from {src_plugin:?} should result in an error"),
        );
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

#[test]
fn serialize_in_root() -> Result<(), ShellError> {
    let span = Span::new(4, 10);
    let mut val = Value::custom(Box::new(expected_test_custom_value()), span);
    PluginCustomValue::serialize_custom_values_in(&mut val)?;

    assert_eq!(span, val.span());

    let custom_value = val.as_custom_value()?;
    if let Some(plugin_custom_value) = custom_value.as_any().downcast_ref::<PluginCustomValue>() {
        assert_eq!("TestCustomValue", plugin_custom_value.name());
        assert_eq!(
            test_plugin_custom_value().data(),
            plugin_custom_value.data()
        );
        assert!(plugin_custom_value.source.is_none());
    } else {
        panic!("Failed to downcast to PluginCustomValue");
    }
    Ok(())
}

#[test]
fn serialize_in_record() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(TestCustomValue(32)));
    let mut val = Value::test_record(record! {
        "foo" => orig_custom_val.clone(),
        "bar" => orig_custom_val.clone(),
    });
    PluginCustomValue::serialize_custom_values_in(&mut val)?;

    check_record_custom_values(&val, &["foo", "bar"], |key, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("'{key}' not PluginCustomValue"));
        assert_eq!(
            "TestCustomValue",
            plugin_custom_value.name(),
            "'{key}' name not set correctly"
        );
        Ok(())
    })
}

#[test]
fn serialize_in_list() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(TestCustomValue(24)));
    let mut val = Value::test_list(vec![orig_custom_val.clone(), orig_custom_val.clone()]);
    PluginCustomValue::serialize_custom_values_in(&mut val)?;

    check_list_custom_values(&val, 0..=1, |index, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not PluginCustomValue"));
        assert_eq!(
            "TestCustomValue",
            plugin_custom_value.name(),
            "[{index}] name not set correctly"
        );
        Ok(())
    })
}

#[test]
fn serialize_in_closure() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(TestCustomValue(24)));
    let mut val = Value::test_closure(Closure {
        block_id: 0,
        captures: vec![(0, orig_custom_val.clone()), (1, orig_custom_val.clone())],
    });
    PluginCustomValue::serialize_custom_values_in(&mut val)?;

    check_closure_custom_values(&val, 0..=1, |index, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not PluginCustomValue"));
        assert_eq!(
            "TestCustomValue",
            plugin_custom_value.name(),
            "[{index}] name not set correctly"
        );
        Ok(())
    })
}

#[test]
fn deserialize_in_root() -> Result<(), ShellError> {
    let span = Span::new(4, 10);
    let mut val = Value::custom(Box::new(test_plugin_custom_value()), span);
    PluginCustomValue::deserialize_custom_values_in(&mut val)?;

    assert_eq!(span, val.span());

    let custom_value = val.as_custom_value()?;
    if let Some(test_custom_value) = custom_value.as_any().downcast_ref::<TestCustomValue>() {
        assert_eq!(expected_test_custom_value(), *test_custom_value);
    } else {
        panic!("Failed to downcast to TestCustomValue");
    }
    Ok(())
}

#[test]
fn deserialize_in_record() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_record(record! {
        "foo" => orig_custom_val.clone(),
        "bar" => orig_custom_val.clone(),
    });
    PluginCustomValue::deserialize_custom_values_in(&mut val)?;

    check_record_custom_values(&val, &["foo", "bar"], |key, custom_value| {
        let test_custom_value: &TestCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("'{key}' not TestCustomValue"));
        assert_eq!(
            expected_test_custom_value(),
            *test_custom_value,
            "{key} not deserialized correctly"
        );
        Ok(())
    })
}

#[test]
fn deserialize_in_list() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_list(vec![orig_custom_val.clone(), orig_custom_val.clone()]);
    PluginCustomValue::deserialize_custom_values_in(&mut val)?;

    check_list_custom_values(&val, 0..=1, |index, custom_value| {
        let test_custom_value: &TestCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not TestCustomValue"));
        assert_eq!(
            expected_test_custom_value(),
            *test_custom_value,
            "[{index}] name not deserialized correctly"
        );
        Ok(())
    })
}

#[test]
fn deserialize_in_closure() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_closure(Closure {
        block_id: 0,
        captures: vec![(0, orig_custom_val.clone()), (1, orig_custom_val.clone())],
    });
    PluginCustomValue::deserialize_custom_values_in(&mut val)?;

    check_closure_custom_values(&val, 0..=1, |index, custom_value| {
        let test_custom_value: &TestCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("[{index}] not TestCustomValue"));
        assert_eq!(
            expected_test_custom_value(),
            *test_custom_value,
            "[{index}] name not deserialized correctly"
        );
        Ok(())
    })
}
