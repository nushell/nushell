use std::{cell::RefCell, collections::HashSet, fmt, marker::PhantomData};

use nu_protocol::{
    CustomValue, FromValue, IntoValue, Record, ShellError, Span, Type,
    engine::{Closure, EngineState},
};
use serde::{
    Serialize, Serializer,
    de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::{self, SerializeMap, SerializeSeq},
};

use crate::Value as JsonValue;
use nu_protocol::Value as NuValue;

impl IntoValue for JsonValue {
    fn into_value(self, span: Span) -> NuValue {
        match self {
            JsonValue::String(s) => NuValue::string(s, span),
            JsonValue::Bool(b) => NuValue::bool(b, span),
            JsonValue::F64(f) => NuValue::float(f, span),
            JsonValue::I64(i) => NuValue::int(i, span),
            JsonValue::Null => NuValue::nothing(span),
            JsonValue::U64(u) => match i64::try_from(u) {
                Ok(i) => NuValue::int(i, span),
                Err(_) => NuValue::float(u as f64, span),
            },
            JsonValue::Array(array) => NuValue::list(
                array.into_iter().map(|x| x.into_value(span)).collect(),
                span,
            ),
            JsonValue::Object(k) => NuValue::record(
                k.into_iter()
                    .map(|(k, v)| (k, v.into_value(span)))
                    .collect(),
                span,
            ),
        }
    }
}

/// Deserializes JSON or Hjson straight into a nushell [`NuValue`] with `span`.
///
/// The result is the same as deserializing a [`JsonValue`] (with the `preserve_order` feature,
/// which keeps keys in input order) and calling [`IntoValue::into_value`] on it, without building
/// the [`JsonValue`] tree in between. Use it with [`from_slice_seed`](crate::from_slice_seed) or
/// any other serde deserializer.
#[derive(Clone, Copy)]
pub struct NuValueSeed {
    pub span: Span,
}

impl<'de> DeserializeSeed<'de> for NuValueSeed {
    type Value = NuValue;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<NuValue, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for NuValueSeed {
    type Value = NuValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a json value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<NuValue, E> {
        Ok(NuValue::bool(value, self.span))
    }

    fn visit_i64<E>(self, value: i64) -> Result<NuValue, E> {
        Ok(NuValue::int(value, self.span))
    }

    fn visit_u64<E>(self, value: u64) -> Result<NuValue, E> {
        Ok(JsonValue::U64(value).into_value(self.span))
    }

    fn visit_f64<E>(self, value: f64) -> Result<NuValue, E> {
        Ok(NuValue::float(value, self.span))
    }

    fn visit_str<E>(self, value: &str) -> Result<NuValue, E> {
        Ok(NuValue::string(value, self.span))
    }

    fn visit_string<E>(self, value: String) -> Result<NuValue, E> {
        Ok(NuValue::string(value, self.span))
    }

    fn visit_none<E>(self) -> Result<NuValue, E> {
        Ok(NuValue::nothing(self.span))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<NuValue, D::Error> {
        self.deserialize(deserializer)
    }

    fn visit_unit<E>(self) -> Result<NuValue, E> {
        Ok(NuValue::nothing(self.span))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<NuValue, A::Error> {
        let mut vals = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(val) = seq.next_element_seed(self)? {
            vals.push(val);
        }
        Ok(NuValue::list(vals, self.span))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<NuValue, A::Error> {
        let mut pairs = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(pair) = map.next_entry_seed(PhantomData::<String>, self)? {
            pairs.push(pair);
        }
        if has_duplicate_keys(pairs.iter().map(|(key, _)| key.as_str())) {
            pairs = keep_last_of_each_key(pairs);
        }
        Ok(NuValue::record(Record::from_iter(pairs), self.span))
    }
}

/// Whether any key appears twice, in O(n log n) so wide objects stay cheap.
fn has_duplicate_keys<'a>(keys: impl ExactSizeIterator<Item = &'a str>) -> bool {
    if keys.len() < 2 {
        return false;
    }
    let mut keys: Vec<&str> = keys.collect();
    keys.sort_unstable();
    keys.windows(2).any(|w| w[0] == w[1])
}

/// Drops repeated keys the way the [`Map`](crate::Map) behind [`JsonValue::Object`] does: a key
/// keeps its last value and sits where it last appeared (`{a: 1, b: 2, a: 3}` gives `{b: 2, a: 3}`).
fn keep_last_of_each_key(pairs: Vec<(String, NuValue)>) -> Vec<(String, NuValue)> {
    let keep: Vec<bool> = {
        let mut seen = HashSet::with_capacity(pairs.len());
        let mut keep: Vec<bool> = pairs
            .iter()
            .rev()
            .map(|(key, _)| seen.insert(key.as_str()))
            .collect();
        keep.reverse();
        keep
    };
    pairs
        .into_iter()
        .zip(keep)
        .filter_map(|(pair, keep)| keep.then_some(pair))
        .collect()
}

#[derive(Clone, Copy)]
enum SerializeMode<'e> {
    Simple,
    Coercing { engine_state: &'e EngineState },
}

/// Writes `value` as JSON with `write` (such as [`to_string`](crate::to_string) or
/// [`to_string_with_indent`](crate::to_string_with_indent)) without building a [`JsonValue`]
/// first.
///
/// The output and errors are the same as converting with [`JsonValue::from_value`] (or
/// [`JsonValue::from_value_serialized`] when `engine_state` is given, which turns closures into
/// their source) and then writing that. The outer error is the conversion error, the inner one
/// the serializer's.
pub fn write_nu_value<T>(
    value: &NuValue,
    engine_state: Option<&EngineState>,
    write: impl FnOnce(&NuValueJson<'_>) -> crate::Result<T>,
) -> Result<crate::Result<T>, ShellError> {
    let mode = match engine_state {
        Some(engine_state) => SerializeMode::Coercing { engine_state },
        None => SerializeMode::Simple,
    };
    let error = RefCell::new(None);
    let written = write(&NuValueJson {
        value,
        mode,
        error: &error,
    });
    match error.into_inner() {
        Some(err) => Err(err),
        None => Ok(written),
    }
}

/// A borrowed nushell value that serializes as JSON; see [`write_nu_value`].
pub struct NuValueJson<'a> {
    value: &'a NuValue,
    mode: SerializeMode<'a>,
    /// The first conversion error. The serializer only carries a message, so the error itself
    /// waits here for [`write_nu_value`].
    error: &'a RefCell<Option<ShellError>>,
}

impl NuValueJson<'_> {
    fn child<'b>(&'b self, value: &'b NuValue) -> NuValueJson<'b> {
        NuValueJson {
            value,
            mode: self.mode,
            error: self.error,
        }
    }

    /// Keeps `err` for [`write_nu_value`] and stops the serializer.
    fn fail<E: ser::Error>(&self, err: ShellError) -> E {
        let msg = err.to_string();
        self.error.borrow_mut().get_or_insert(err);
        E::custom(msg)
    }
}

impl Serialize for NuValueJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let span = self.value.span();
        match self.value {
            NuValue::Bool { val, .. } => serializer.serialize_bool(*val),
            NuValue::Int { val, .. } => serializer.serialize_i64(*val),
            NuValue::Float { val, .. } => serializer.serialize_f64(*val),
            NuValue::String { val, .. } => serializer.serialize_str(val),
            NuValue::Glob { val, .. } => serializer.serialize_str(&val.to_string()),
            NuValue::Filesize { val, .. } => serializer.serialize_i64(val.get()),
            NuValue::Duration { val, .. } => serializer.serialize_i64(*val),
            NuValue::Date { val, .. } => serializer.serialize_str(&val.to_rfc3339()),
            NuValue::Range { val, .. } => serializer.serialize_str(&val.to_string()),
            NuValue::Record { val, .. } => {
                if has_duplicate_keys(val.columns().map(String::as_str)) {
                    // Rare: let the map behind `JsonValue::Object` pick which keys stay.
                    return match JsonValue::from_value_impl(self.value.clone(), self.mode) {
                        Ok(json) => json.serialize(serializer),
                        Err(err) => Err(self.fail(err)),
                    };
                }
                let mut map = serializer.serialize_map(Some(val.len()))?;
                for (key, value) in val.iter() {
                    map.serialize_key(key)?;
                    map.serialize_value(&self.child(value))?;
                }
                map.end()
            }
            NuValue::List { vals, .. } => {
                let mut seq = serializer.serialize_seq(Some(vals.len()))?;
                for value in vals.iter() {
                    seq.serialize_element(&self.child(value))?;
                }
                seq.end()
            }
            NuValue::Closure { val, .. } => {
                match closure_into_json_value((**val).clone(), self.mode, span) {
                    Ok(json) => json.serialize(serializer),
                    Err(err) => Err(self.fail(err)),
                }
            }
            NuValue::Error { error, .. } => Err(self.fail((**error).clone())),
            NuValue::Binary { val, .. } => {
                let mut seq = serializer.serialize_seq(Some(val.len()))?;
                for byte in val {
                    seq.serialize_element(&u64::from(*byte))?;
                }
                seq.end()
            }
            NuValue::CellPath { val, .. } => serializer.serialize_str(&val.to_string()),
            NuValue::Custom { val, .. } => match val.to_base_value(span) {
                Ok(base) => self.child(&base).serialize(serializer),
                Err(err) => Err(self.fail(err)),
            },
            NuValue::Nothing { .. } => serializer.serialize_unit(),
        }
    }
}

impl FromValue for JsonValue {
    fn from_value(v: NuValue) -> Result<Self, ShellError> {
        Self::from_value_impl(v, SerializeMode::Simple)
    }
}

impl JsonValue {
    pub fn from_value_serialized(
        v: NuValue,
        engine_state: &EngineState,
    ) -> Result<Self, ShellError> {
        Self::from_value_impl(v, SerializeMode::Coercing { engine_state })
    }

    fn from_value_impl(v: NuValue, mode: SerializeMode<'_>) -> Result<Self, ShellError> {
        let span = v.span();
        Ok(match v {
            NuValue::Bool { val, .. } => JsonValue::Bool(val),
            NuValue::Int { val, .. } => JsonValue::I64(val),
            NuValue::Float { val, .. } => JsonValue::F64(val),
            NuValue::String { val, .. } => JsonValue::String(val),
            NuValue::Glob { val, .. } => JsonValue::String(val.to_string()),
            NuValue::Filesize { val, .. } => JsonValue::I64(val.get()),
            NuValue::Duration { val, .. } => JsonValue::I64(val),
            NuValue::Date { val, .. } => JsonValue::String(val.to_rfc3339()),
            NuValue::Range { val, .. } => JsonValue::String(val.to_string()),
            NuValue::Record { val, .. } => record_into_json_value(val.into_owned(), mode)?,
            NuValue::List { vals, .. } => list_into_json_value(vals, mode)?,
            NuValue::Closure { val, .. } => closure_into_json_value(*val, mode, span)?,
            NuValue::Error { error, .. } => return Err(*error),
            NuValue::Binary { val, .. } => binary_into_json_value(&val),
            NuValue::CellPath { val, .. } => JsonValue::String(val.to_string()),
            NuValue::Custom { val, .. } => custom_into_json_value(val, mode, span)?,
            NuValue::Nothing { .. } => JsonValue::Null,
        })
    }
}

fn record_into_json_value(
    record: Record,
    mode: SerializeMode<'_>,
) -> Result<JsonValue, ShellError> {
    Ok(JsonValue::Object(
        record
            .into_iter()
            .map(|(k, v)| JsonValue::from_value_impl(v, mode).map(|v| (k, v)))
            .collect::<Result<_, _>>()?,
    ))
}

fn list_into_json_value(
    vals: impl IntoIterator<Item = NuValue>,
    mode: SerializeMode<'_>,
) -> Result<JsonValue, ShellError> {
    Ok(JsonValue::Array(
        vals.into_iter()
            .map(|v| JsonValue::from_value_impl(v, mode))
            .collect::<Result<_, _>>()?,
    ))
}

fn closure_into_json_value(
    val: Closure,
    mode: SerializeMode<'_>,
    span: Span,
) -> Result<JsonValue, ShellError> {
    match mode {
        SerializeMode::Simple => Err(ShellError::CantConvert {
            to_type: Type::String.to_string(),
            from_type: Type::Closure.to_string(),
            span,
            help: None,
        }),
        SerializeMode::Coercing { engine_state } => Ok(JsonValue::String(
            val.coerce_into_string(engine_state, span)?.to_string(),
        )),
    }
}

fn binary_into_json_value(val: &[u8]) -> JsonValue {
    JsonValue::Array(
        val.iter()
            .copied()
            .map(|b| JsonValue::U64(b as u64))
            .collect(),
    )
}

fn custom_into_json_value(
    val: Box<dyn CustomValue>,
    mode: SerializeMode,
    span: Span,
) -> Result<JsonValue, ShellError> {
    JsonValue::from_value_impl(val.to_base_value(span)?, mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_slice, from_slice_seed, to_string, to_string_raw};
    use nu_protocol::record;

    /// Debug output keeps record key order, which `==` on records ignores.
    fn same(a: &NuValue, b: &NuValue) {
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn seed_matches_json_value_tree() {
        let span = Span::test_data();
        let inputs = [
            r#"{"a": 1, "b": [true, null, "x"], "c": {"d": -0, "e": 1.5e3}}"#,
            "[0, -1, 9223372036854775807, 9223372036854775808, 18446744073709551616, -9223372036854775809]",
            r#"{"a": 1, "b": 2, "a": 3, "b": 4, "a": 5, "c": {"x": 1, "x": 2}}"#,
            "// comment\n{\n  key: quoteless value\n  list: [1, 2,]\n}",
            "braceless: 1\nother: two",
            r#""é😀""#,
            "[]",
            "{}",
        ];
        for input in inputs {
            let direct = from_slice_seed(input.as_bytes(), NuValueSeed { span })
                .expect("valid input parses");
            let tree = from_slice::<JsonValue>(input.as_bytes())
                .expect("valid input parses")
                .into_value(span);
            same(&direct, &tree);
        }
    }

    #[test]
    fn seed_keeps_last_duplicate_at_its_last_position() {
        let span = Span::test_data();
        let value = from_slice_seed(br#"{"a": 1, "b": 2, "a": 3}"#, NuValueSeed { span })
            .expect("valid input parses");
        same(
            &value,
            &NuValue::test_record(record! {
                "b" => NuValue::test_int(2),
                "a" => NuValue::test_int(3),
            }),
        );
    }

    #[test]
    fn seed_reports_the_same_error() {
        let span = Span::test_data();
        for input in ["[1, 2", r#"{"a" 1}"#, "[1] x"] {
            let direct = from_slice_seed(input.as_bytes(), NuValueSeed { span })
                .expect_err("invalid input fails");
            let tree = from_slice::<JsonValue>(input.as_bytes()).expect_err("invalid input fails");
            assert_eq!(direct.to_string(), tree.to_string());
        }
    }

    #[test]
    fn write_matches_json_value_tree() {
        let span = Span::test_data();
        let mut with_duplicates = Record::new();
        with_duplicates.push("a", NuValue::test_int(1));
        with_duplicates.push("b", NuValue::test_int(2));
        with_duplicates.push("a", NuValue::test_int(3));
        let values = [
            NuValue::test_record(record! {
                "a" => NuValue::test_int(1),
                "b" => NuValue::test_list(vec![NuValue::test_float(-0.0), NuValue::test_nothing()]),
                "c" => NuValue::test_string("é\n\"😀"),
            }),
            NuValue::test_binary(vec![0, 255, 16]),
            NuValue::test_filesize(1024),
            NuValue::test_duration(-5),
            NuValue::test_glob("*.rs"),
            NuValue::test_list(vec![NuValue::record(with_duplicates, span)]),
            NuValue::test_list(vec![]),
            NuValue::test_record(Record::new()),
        ];
        for value in values {
            let direct = write_nu_value(&value, None, |json| to_string(json))
                .expect("converts")
                .expect("serializes");
            let tree = to_string(&JsonValue::from_value(value.clone()).expect("converts"))
                .expect("serializes");
            assert_eq!(direct, tree);
            let direct = write_nu_value(&value, None, |json| to_string_raw(json))
                .expect("converts")
                .expect("serializes");
            let tree = to_string_raw(&JsonValue::from_value(value).expect("converts"))
                .expect("serializes");
            assert_eq!(direct, tree);
        }
    }

    #[test]
    fn write_returns_the_conversion_error() {
        let span = Span::test_data();
        let error = ShellError::NushellFailed { msg: "boom".into() };
        let value = NuValue::test_list(vec![
            NuValue::test_int(1),
            NuValue::error(error.clone(), span),
        ]);
        let direct = write_nu_value(&value, None, |json| to_string(json))
            .expect_err("the error value is returned");
        let tree = JsonValue::from_value(value).expect_err("the error value is returned");
        assert_eq!(format!("{direct:?}"), format!("{tree:?}"));
    }
}
