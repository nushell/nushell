use nu_protocol::ast::PathMember;
use nu_protocol::{Record, ShellError, Span, Value};

pub fn value_to_json_value(v: &Value) -> Result<nu_json::Value, ShellError> {
    let span = v.span();
    Ok(match v {
        Value::Bool { val, .. } => nu_json::Value::Bool(*val),
        Value::Filesize { val, .. } => nu_json::Value::I64(val.get()),
        Value::Duration { val, .. } => nu_json::Value::I64(*val),
        Value::Date { val, .. } => nu_json::Value::String(val.to_string()),
        Value::Float { val, .. } => nu_json::Value::F64(*val),
        Value::Int { val, .. } => nu_json::Value::I64(*val),
        Value::Nothing { .. } => nu_json::Value::Null,
        Value::String { val, .. } => nu_json::Value::String(val.to_string()),
        Value::Glob { val, .. } => nu_json::Value::String(val.to_string()),
        Value::CellPath { val, .. } => nu_json::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(nu_json::Value::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(nu_json::Value::U64(*val as u64)),
                })
                .collect::<Result<Vec<nu_json::Value>, ShellError>>()?,
        ),

        Value::List { vals, .. } => nu_json::Value::Array(json_list(vals)?),
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Closure { .. } | Value::Range { .. } => nu_json::Value::Null,
        Value::Binary { val, .. } => {
            nu_json::Value::Array(val.iter().map(|x| nu_json::Value::U64(*x as u64)).collect())
        }
        Value::Record { val, .. } => {
            let mut m = nu_json::Map::new();
            for (k, v) in &**val {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            nu_json::Value::Object(m)
        }
        Value::Custom { val, .. } => {
            let collected = val.to_base_value(span)?;
            value_to_json_value(&collected)?
        }
    })
}

pub fn json_list(input: &[Value]) -> Result<Vec<nu_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

pub fn json_to_value(v: nu_json::Value, span: Span) -> Result<Value, ShellError> {
    Ok(match v {
        nu_json::Value::Null => Value::nothing(span),
        nu_json::Value::Bool(val) => Value::bool(val, span),
        nu_json::Value::I64(val) => Value::int(val, span),
        nu_json::Value::U64(val) => {
            if val <= i64::MAX as u64 {
                let val = val as i64;
                Value::int(val, span)
            } else {
                Value::string(format!("{}", val), span)
            }
        }
        nu_json::Value::F64(val) => Value::float(val, span),
        nu_json::Value::String(val) => Value::string(val, span),
        nu_json::Value::Array(vec) => {
            let arr: &mut Vec<Value> = &mut vec![];
            for jval in vec {
                arr.push(json_to_value(jval, span)?);
            }
            Value::list(arr.to_vec(), span)
        }
        nu_json::Value::Object(val) => {
            let mut rec = Record::new();
            for (k, v) in val {
                let value = json_to_value(v, span)?;
                rec.insert(k.clone(), value);
            }
            Value::record(rec, span)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_json::Value as JsonValue;
    use nu_protocol::{Record, Span, Value, ast::PathMember};

    fn span() -> Span {
        Span::test_data()
    }

    #[test]
    fn test_primitives_to_json() {
        let s = span();

        assert_eq!(
            value_to_json_value(&Value::int(42, s)).unwrap(),
            JsonValue::I64(42)
        );
        assert_eq!(
            value_to_json_value(&Value::float(3.14, s)).unwrap(),
            JsonValue::F64(3.14)
        );
        assert_eq!(
            value_to_json_value(&Value::bool(true, s)).unwrap(),
            JsonValue::Bool(true)
        );
        assert_eq!(
            value_to_json_value(&Value::nothing(s)).unwrap(),
            JsonValue::Null
        );
        assert_eq!(
            value_to_json_value(&Value::string("abc", s)).unwrap(),
            JsonValue::String("abc".into())
        );
    }

    #[test]
    fn test_list_to_json() {
        let s = span();
        let vals = vec![Value::int(1, s), Value::string("two", s)];
        let json = value_to_json_value(&Value::list(vals, s)).unwrap();
        assert_eq!(
            json,
            JsonValue::Array(vec![JsonValue::I64(1), JsonValue::String("two".into())])
        );
    }

    #[test]
    fn test_record_to_json() {
        let s = span();
        let mut rec = Record::new();
        rec.insert("a", Value::int(1, s));
        rec.insert("b", Value::bool(false, s));

        let json = value_to_json_value(&Value::record(rec.clone(), s)).unwrap();
        let mut expected = nu_json::Map::new();
        expected.insert("a".into(), JsonValue::I64(1));
        expected.insert("b".into(), JsonValue::Bool(false));

        assert_eq!(json, JsonValue::Object(expected));
    }

    #[test]
    fn test_cellpath_to_json() {
        let s = span();
        let cellpath = Value::cell_path(
            nu_protocol::ast::CellPath {
                members: vec![
                    PathMember::string(
                        "foo".into(),
                        false,
                        nu_protocol::casing::Casing::Insensitive,
                        s,
                    ),
                    PathMember::int(42, false, s),
                ],
            },
            s,
        );

        let json = value_to_json_value(&cellpath).unwrap();
        assert_eq!(
            json,
            JsonValue::Array(vec![JsonValue::String("foo".into()), JsonValue::U64(42),])
        );
    }

    #[test]
    fn test_binary_to_json() {
        let s = span();
        let bytes = vec![1u8, 2u8, 3u8];
        let json = value_to_json_value(&Value::binary(bytes.clone(), s)).unwrap();
        assert_eq!(
            json,
            JsonValue::Array(vec![
                JsonValue::U64(1),
                JsonValue::U64(2),
                JsonValue::U64(3),
            ])
        );
    }

    #[test]
    fn test_json_to_value_primitives() {
        let s = span();

        assert_eq!(
            json_to_value(JsonValue::I64(123), s).unwrap(),
            Value::int(123, s)
        );
        assert_eq!(
            json_to_value(JsonValue::F64(2.5), s).unwrap(),
            Value::float(2.5, s)
        );
        assert_eq!(
            json_to_value(JsonValue::Bool(true), s).unwrap(),
            Value::bool(true, s)
        );
        assert_eq!(
            json_to_value(JsonValue::Null, s).unwrap(),
            Value::nothing(s)
        );
        assert_eq!(
            json_to_value(JsonValue::String("abc".into()), s).unwrap(),
            Value::string("abc", s)
        );
    }

    #[test]
    fn test_json_to_value_array() {
        let s = span();
        let jarr = JsonValue::Array(vec![JsonValue::I64(1), JsonValue::String("two".into())]);
        let val = json_to_value(jarr, s).unwrap();

        match val {
            Value::List { vals, .. } => {
                assert_eq!(vals.len(), 2);
                assert_eq!(vals[0], Value::int(1, s));
                assert_eq!(vals[1], Value::string("two", s));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_json_to_value_object() {
        let s = span();
        let mut obj = nu_json::Map::new();
        obj.insert("x".into(), JsonValue::I64(5));
        obj.insert("y".into(), JsonValue::Bool(true));

        let val = json_to_value(JsonValue::Object(obj.clone()), s).unwrap();

        match val {
            Value::Record { val: rec, .. } => {
                assert_eq!(rec.get("x"), Some(&Value::int(5, s)));
                assert_eq!(rec.get("y"), Some(&Value::bool(true, s)));
            }
            _ => panic!("Expected record"),
        }
    }

    #[test]
    fn test_u64_overflow_handling() {
        let s = span();
        let big = (i64::MAX as u64) + 1000;
        let val = json_to_value(JsonValue::U64(big), s).unwrap();

        match val {
            Value::String { val, .. } => assert_eq!(val, big.to_string()),
            _ => panic!("Expected string fallback for overflow"),
        };
    }

    #[test]
    fn test_roundtrip_value_json_value() {
        let s = span();
        let mut rec = Record::new();
        rec.insert("num", Value::int(7, s));
        rec.insert("msg", Value::string("ok", s));
        rec.insert("flag", Value::bool(true, s));
        let original = Value::record(rec.clone(), s);

        let json = value_to_json_value(&original).unwrap();
        let roundtrip = json_to_value(json, s).unwrap();

        assert_eq!(original, roundtrip);
    }
}
