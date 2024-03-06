use std::sync::Arc;

use nu_protocol::{
    ast::RangeInclusion, engine::Closure, record, CustomValue, Range, ShellError, Span, Value,
};

use crate::{
    plugin::PluginSource,
    protocol::test_util::{
        expected_test_custom_value, test_plugin_custom_value, test_plugin_custom_value_with_source,
        TestCustomValue,
    },
};

use super::PluginCustomValue;

#[test]
fn serialize_deserialize() -> Result<(), ShellError> {
    let original_value = TestCustomValue(32);
    let span = Span::test_data();
    let serialized = PluginCustomValue::serialize_from_custom_value(&original_value, span)?;
    assert_eq!(original_value.value_string(), serialized.name());
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
fn add_source_at_root() -> Result<(), ShellError> {
    let mut val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source(&mut val, &source);

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

fn check_range_custom_values(
    val: &Value,
    mut f: impl FnMut(&str, &dyn CustomValue) -> Result<(), ShellError>,
) -> Result<(), ShellError> {
    let range = val.as_range()?;
    for (name, val) in [
        ("from", &range.from),
        ("incr", &range.incr),
        ("to", &range.to),
    ] {
        let custom_value = val
            .as_custom_value()
            .unwrap_or_else(|_| panic!("{name} not custom value"));
        f(name, custom_value)?;
    }
    Ok(())
}

#[test]
fn add_source_nested_range() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_range(Range {
        from: orig_custom_val.clone(),
        incr: orig_custom_val.clone(),
        to: orig_custom_val.clone(),
        inclusion: RangeInclusion::Inclusive,
    });
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source(&mut val, &source);

    check_range_custom_values(&val, |name, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("{name} not PluginCustomValue"));
        assert_eq!(
            Some(Arc::as_ptr(&source)),
            plugin_custom_value.source.as_ref().map(Arc::as_ptr),
            "{name} source not set correctly"
        );
        Ok(())
    })
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
fn add_source_nested_record() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_record(record! {
        "foo" => orig_custom_val.clone(),
        "bar" => orig_custom_val.clone(),
    });
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source(&mut val, &source);

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
fn add_source_nested_list() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_list(vec![orig_custom_val.clone(), orig_custom_val.clone()]);
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source(&mut val, &source);

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
fn add_source_nested_closure() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_closure(Closure {
        block_id: 0,
        captures: vec![(0, orig_custom_val.clone()), (1, orig_custom_val.clone())],
    });
    let source = Arc::new(PluginSource::new_fake("foo"));
    PluginCustomValue::add_source(&mut val, &source);

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
    let mut ok_val = Value::custom_value(Box::new(test_plugin_custom_value_with_source()), span);
    let mut native_val = Value::custom_value(Box::new(TestCustomValue(32)), span);
    let mut foreign_val = {
        let mut val = test_plugin_custom_value();
        val.source = Some(Arc::new(PluginSource::new_fake("other")));
        Value::custom_value(Box::new(val), span)
    };
    let source = PluginSource::new_fake("test");

    PluginCustomValue::verify_source(&mut ok_val, &source).expect("ok_val should be verified ok");

    for (val, src_plugin) in [(&mut native_val, None), (&mut foreign_val, Some("other"))] {
        let error = PluginCustomValue::verify_source(val, &source).expect_err(&format!(
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

#[test]
fn verify_source_nested_range() -> Result<(), ShellError> {
    let native_val = Value::test_custom_value(Box::new(TestCustomValue(32)));
    let source = PluginSource::new_fake("test");
    for (name, mut val) in [
        (
            "from",
            Value::test_range(Range {
                from: native_val.clone(),
                incr: Value::test_nothing(),
                to: Value::test_nothing(),
                inclusion: RangeInclusion::RightExclusive,
            }),
        ),
        (
            "incr",
            Value::test_range(Range {
                from: Value::test_nothing(),
                incr: native_val.clone(),
                to: Value::test_nothing(),
                inclusion: RangeInclusion::RightExclusive,
            }),
        ),
        (
            "to",
            Value::test_range(Range {
                from: Value::test_nothing(),
                incr: Value::test_nothing(),
                to: native_val.clone(),
                inclusion: RangeInclusion::RightExclusive,
            }),
        ),
    ] {
        PluginCustomValue::verify_source(&mut val, &source)
            .expect_err(&format!("error not generated on {name}"));
    }

    let mut ok_range = Value::test_range(Range {
        from: Value::test_nothing(),
        incr: Value::test_nothing(),
        to: Value::test_nothing(),
        inclusion: RangeInclusion::RightExclusive,
    });
    PluginCustomValue::verify_source(&mut ok_range, &source)
        .expect("ok_range should not generate error");

    Ok(())
}

#[test]
fn verify_source_nested_record() -> Result<(), ShellError> {
    let native_val = Value::test_custom_value(Box::new(TestCustomValue(32)));
    let source = PluginSource::new_fake("test");
    for (name, mut val) in [
        (
            "first element foo",
            Value::test_record(record! {
                "foo" => native_val.clone(),
                "bar" => Value::test_nothing(),
            }),
        ),
        (
            "second element bar",
            Value::test_record(record! {
                "foo" => Value::test_nothing(),
                "bar" => native_val.clone(),
            }),
        ),
    ] {
        PluginCustomValue::verify_source(&mut val, &source)
            .expect_err(&format!("error not generated on {name}"));
    }

    let mut ok_record = Value::test_record(record! {"foo" => Value::test_nothing()});
    PluginCustomValue::verify_source(&mut ok_record, &source)
        .expect("ok_record should not generate error");

    Ok(())
}

#[test]
fn verify_source_nested_list() -> Result<(), ShellError> {
    let native_val = Value::test_custom_value(Box::new(TestCustomValue(32)));
    let source = PluginSource::new_fake("test");
    for (name, mut val) in [
        (
            "first element",
            Value::test_list(vec![native_val.clone(), Value::test_nothing()]),
        ),
        (
            "second element",
            Value::test_list(vec![Value::test_nothing(), native_val.clone()]),
        ),
    ] {
        PluginCustomValue::verify_source(&mut val, &source)
            .expect_err(&format!("error not generated on {name}"));
    }

    let mut ok_list = Value::test_list(vec![Value::test_nothing()]);
    PluginCustomValue::verify_source(&mut ok_list, &source)
        .expect("ok_list should not generate error");

    Ok(())
}

#[test]
fn verify_source_nested_closure() -> Result<(), ShellError> {
    let native_val = Value::test_custom_value(Box::new(TestCustomValue(32)));
    let source = PluginSource::new_fake("test");
    for (name, mut val) in [
        (
            "first capture",
            Value::test_closure(Closure {
                block_id: 0,
                captures: vec![(0, native_val.clone()), (1, Value::test_nothing())],
            }),
        ),
        (
            "second capture",
            Value::test_closure(Closure {
                block_id: 0,
                captures: vec![(0, Value::test_nothing()), (1, native_val.clone())],
            }),
        ),
    ] {
        PluginCustomValue::verify_source(&mut val, &source)
            .expect_err(&format!("error not generated on {name}"));
    }

    let mut ok_closure = Value::test_closure(Closure {
        block_id: 0,
        captures: vec![(0, Value::test_nothing())],
    });
    PluginCustomValue::verify_source(&mut ok_closure, &source)
        .expect("ok_closure should not generate error");

    Ok(())
}

#[test]
fn serialize_in_root() -> Result<(), ShellError> {
    let span = Span::new(4, 10);
    let mut val = Value::custom_value(Box::new(expected_test_custom_value()), span);
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
fn serialize_in_range() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(TestCustomValue(-1)));
    let mut val = Value::test_range(Range {
        from: orig_custom_val.clone(),
        incr: orig_custom_val.clone(),
        to: orig_custom_val.clone(),
        inclusion: RangeInclusion::Inclusive,
    });
    PluginCustomValue::serialize_custom_values_in(&mut val)?;

    check_range_custom_values(&val, |name, custom_value| {
        let plugin_custom_value: &PluginCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("{name} not PluginCustomValue"));
        assert_eq!(
            "TestCustomValue",
            plugin_custom_value.name(),
            "{name} name not set correctly"
        );
        Ok(())
    })
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
    let mut val = Value::custom_value(Box::new(test_plugin_custom_value()), span);
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
fn deserialize_in_range() -> Result<(), ShellError> {
    let orig_custom_val = Value::test_custom_value(Box::new(test_plugin_custom_value()));
    let mut val = Value::test_range(Range {
        from: orig_custom_val.clone(),
        incr: orig_custom_val.clone(),
        to: orig_custom_val.clone(),
        inclusion: RangeInclusion::Inclusive,
    });
    PluginCustomValue::deserialize_custom_values_in(&mut val)?;

    check_range_custom_values(&val, |name, custom_value| {
        let test_custom_value: &TestCustomValue = custom_value
            .as_any()
            .downcast_ref()
            .unwrap_or_else(|| panic!("{name} not TestCustomValue"));
        assert_eq!(
            expected_test_custom_value(),
            *test_custom_value,
            "{name} not deserialized correctly"
        );
        Ok(())
    })
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
