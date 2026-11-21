use std::time::SystemTime;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand, SimplePluginCommand};
use nu_protocol::{
    Category, DateTime, Example, LabeledError, Record, Signature, Span, Value as NuValue,
    jiff::tz::Offset, record,
};
use plist::{Date as PlistDate, Dictionary, Value as PlistValue};

use crate::FormatCmdsPlugin;

pub struct FromPlist;

impl SimplePluginCommand for FromPlist {
    type Plugin = FormatCmdsPlugin;

    fn name(&self) -> &str {
        "from plist"
    }

    fn description(&self) -> &str {
        "Convert plist to Nushell values"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: r#"'<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
        <key>a</key>
        <integer>3</integer>
</dict>
</plist>' | from plist"#,
            description: "Convert a table into a plist file",
            result: Some(NuValue::test_record(record!( "a" => NuValue::test_int(3)))),
        }]
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self)).category(Category::Formats)
    }

    fn run(
        &self,
        _plugin: &FormatCmdsPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        match input {
            NuValue::String { val, .. } => {
                let plist = plist::from_bytes(val.as_bytes())
                    .map_err(|e| build_label_error(format!("{e}"), input.span()))?;
                let converted = convert_plist_value(&plist, call.head)?;
                Ok(converted)
            }
            NuValue::Binary { val, .. } => {
                let plist = plist::from_bytes(val)
                    .map_err(|e| build_label_error(format!("{e}"), input.span()))?;
                let converted = convert_plist_value(&plist, call.head)?;
                Ok(converted)
            }
            _ => Err(build_label_error(
                format!("Invalid input, must be string not: {input:?}"),
                call.head,
            )),
        }
    }
}

fn build_label_error(msg: impl Into<String>, span: Span) -> LabeledError {
    LabeledError::new("Could not load plist").with_label(msg, span)
}

fn convert_plist_value(plist_val: &PlistValue, span: Span) -> Result<NuValue, LabeledError> {
    match plist_val {
        PlistValue::String(s) => Ok(NuValue::string(s.to_owned(), span)),
        PlistValue::Boolean(b) => Ok(NuValue::bool(*b, span)),
        PlistValue::Real(r) => Ok(NuValue::float(*r, span)),
        PlistValue::Date(d) => convert_date(d)
            .map(|date| NuValue::date(date, span))
            .ok_or_else(|| build_label_error("plist date is out of range", span)),
        PlistValue::Integer(i) => {
            let signed = i
                .as_signed()
                .ok_or_else(|| build_label_error(format!("Cannot convert {i} to i64"), span))?;
            Ok(NuValue::int(signed, span))
        }
        PlistValue::Uid(uid) => Ok(NuValue::float(uid.get() as f64, span)),
        PlistValue::Data(data) => Ok(NuValue::binary(data.to_owned(), span)),
        PlistValue::Array(arr) => Ok(NuValue::list(convert_array(arr, span)?, span)),
        PlistValue::Dictionary(dict) => Ok(convert_dict(dict, span)?),
        _ => Ok(NuValue::nothing(span)),
    }
}

fn convert_dict(dict: &Dictionary, span: Span) -> Result<NuValue, LabeledError> {
    let cols: Vec<String> = dict.keys().cloned().collect();
    let vals: Result<Vec<NuValue>, LabeledError> = dict
        .values()
        .map(|v| convert_plist_value(v, span))
        .collect();
    Ok(NuValue::record(
        Record::from_raw_cols_vals(cols, vals?, span, span)?,
        span,
    ))
}

fn convert_array(plist_array: &[PlistValue], span: Span) -> Result<Vec<NuValue>, LabeledError> {
    plist_array
        .iter()
        .map(|v| convert_plist_value(v, span))
        .collect()
}

/// The plist date at UTC, or `None` outside years -262143 to 262142.
pub fn convert_date(plist_date: &PlistDate) -> Option<DateTime> {
    // In the docs the plist date object is listed as a utc timestamp, so this
    // conversion should be fine
    let plist_sys_time: SystemTime = plist_date.to_owned().into();
    DateTime::from_system_time(plist_sys_time).map(|date| date.with_offset(Offset::UTC))
}

#[cfg(test)]
mod test {
    use super::*;
    use plist::Uid;
    use std::time::SystemTime;

    use nu_plugin_test_support::PluginTest;
    use nu_protocol::ShellError;

    #[test]
    fn test_convert_string() {
        let plist_val = PlistValue::String("hello".to_owned());
        let result = convert_plist_value(&plist_val, Span::test_data());
        assert_eq!(
            result,
            Ok(NuValue::string("hello".to_owned(), Span::test_data()))
        );
    }

    #[test]
    fn test_convert_boolean() {
        let plist_val = PlistValue::Boolean(true);
        let result = convert_plist_value(&plist_val, Span::test_data());
        assert_eq!(result, Ok(NuValue::bool(true, Span::test_data())));
    }

    #[test]
    fn test_convert_real() {
        let plist_val = PlistValue::Real(3.5);
        let result = convert_plist_value(&plist_val, Span::test_data());
        assert_eq!(result, Ok(NuValue::float(3.5, Span::test_data())));
    }

    #[test]
    fn test_convert_integer() {
        let plist_val = PlistValue::Integer(42.into());
        let result = convert_plist_value(&plist_val, Span::test_data());
        assert_eq!(result, Ok(NuValue::int(42, Span::test_data())));
    }

    #[test]
    fn test_convert_uid() {
        let v = 12345678_u64;
        let uid = Uid::new(v);
        let plist_val = PlistValue::Uid(uid);
        let result = convert_plist_value(&plist_val, Span::test_data());
        assert_eq!(result, Ok(NuValue::float(v as f64, Span::test_data())));
    }

    #[test]
    fn test_convert_data() {
        let data = vec![0x41, 0x42, 0x43];
        let plist_val = PlistValue::Data(data.clone());
        let result = convert_plist_value(&plist_val, Span::test_data());
        assert_eq!(result, Ok(NuValue::binary(data, Span::test_data())));
    }

    #[test]
    fn test_convert_date() {
        let epoch = SystemTime::UNIX_EPOCH;
        let plist_date = epoch.into();

        let datetime = convert_date(&plist_date).unwrap().to_civil().unwrap();
        assert_eq!(1970, datetime.year());
        assert_eq!(1, datetime.month());
        assert_eq!(1, datetime.day());
    }

    // jiff's timestamps stop at 9999-12-30T22:00Z; plist dates reach the end of the year.
    #[test]
    fn test_convert_date_on_the_last_day_of_9999() {
        let last: DateTime = "9999-12-31T23:59:59Z".parse().unwrap();
        let plist_date = last.to_system_time().unwrap().into();
        assert_eq!(convert_date(&plist_date), Some(last));
    }

    #[test]
    fn test_convert_dict() {
        let mut dict = Dictionary::new();
        dict.insert("a".to_string(), PlistValue::String("c".to_string()));
        dict.insert("b".to_string(), PlistValue::String("d".to_string()));
        let nu_dict = convert_dict(&dict, Span::test_data()).unwrap();
        assert_eq!(
            nu_dict,
            NuValue::record(
                Record::from_raw_cols_vals(
                    vec!["a".to_string(), "b".to_string()],
                    vec![
                        NuValue::string("c".to_string(), Span::test_data()),
                        NuValue::string("d".to_string(), Span::test_data())
                    ],
                    Span::test_data(),
                    Span::test_data(),
                )
                .expect("failed to create record"),
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_convert_array() {
        let arr = vec![
            PlistValue::String("a".into()),
            PlistValue::String("b".into()),
        ];
        let nu_arr = convert_array(&arr, Span::test_data()).unwrap();
        assert_eq!(
            nu_arr,
            vec![
                NuValue::string("a".to_string(), Span::test_data()),
                NuValue::string("b".to_string(), Span::test_data())
            ]
        );
    }

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        let plugin = FormatCmdsPlugin {};
        let cmd = FromPlist {};

        let mut plugin_test = PluginTest::new("polars", plugin.into())?;
        plugin_test.test_command_examples(&cmd)
    }
}
