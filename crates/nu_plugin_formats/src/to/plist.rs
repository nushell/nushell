use crate::FormatCmdsPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand, SimplePluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, Record, Signature, Span, Value as NuValue, jiff::tz::Offset,
};
use plist::Value as PlistValue;

pub(crate) struct IntoPlist;

impl SimplePluginCommand for IntoPlist {
    type Plugin = FormatCmdsPlugin;

    fn name(&self) -> &str {
        "to plist"
    }

    fn description(&self) -> &str {
        "Convert Nu values into plist"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "{ a: 3 } | to plist",
            description: "Convert a table into a plist file",
            result: None,
        }]
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .switch("binary", "Output plist in binary format.", Some('b'))
            .category(Category::Formats)
    }

    fn run(
        &self,
        _plugin: &FormatCmdsPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        let plist_val = convert_nu_value(input)?;
        let mut out = Vec::new();
        if call.has_flag("binary")? {
            plist::to_writer_binary(&mut out, &plist_val)
                .map_err(|e| build_label_error(format!("{e}"), input.span()))?;
            Ok(NuValue::binary(out, input.span()))
        } else {
            plist::to_writer_xml(&mut out, &plist_val)
                .map_err(|e| build_label_error(format!("{e}"), input.span()))?;
            Ok(NuValue::string(
                String::from_utf8(out)
                    .map_err(|e| build_label_error(format!("{e}"), input.span()))?,
                input.span(),
            ))
        }
    }
}

fn build_label_error(msg: String, span: Span) -> LabeledError {
    LabeledError::new("Cannot convert plist").with_label(msg, span)
}

fn convert_nu_value(nu_val: &NuValue) -> Result<PlistValue, LabeledError> {
    let span = Span::test_data();
    match nu_val {
        NuValue::String { val, .. } => Ok(PlistValue::String(val.to_owned())),
        NuValue::Bool { val, .. } => Ok(PlistValue::Boolean(*val)),
        NuValue::Float { val, .. } => Ok(PlistValue::Real(*val)),
        NuValue::Int { val, .. } => Ok(PlistValue::Integer((*val).into())),
        NuValue::Binary { val, .. } => Ok(PlistValue::Data(val.to_vec())),
        NuValue::Record { val, .. } => convert_nu_dict(val),
        NuValue::List { vals, .. } => Ok(PlistValue::Array(
            vals.iter()
                .map(convert_nu_value)
                .collect::<Result<_, _>>()?,
        )),
        // The plist crate writes every date, binary ones too, as RFC 3339, and panics on a
        // year outside 0 to 9999.
        NuValue::Date { val, .. } if (0..=9999).contains(&val.with_offset(Offset::UTC).year()) => {
            val.to_system_time()
                .map(|time| PlistValue::Date(time.into()))
                .ok_or_else(|| build_label_error(format!("{val} is out of range"), span))
        }
        NuValue::Date { val, .. } => Err(build_label_error(
            format!("{val} is out of range: plist dates are years 0 to 9999"),
            span,
        )),
        NuValue::Filesize { val, .. } => Ok(PlistValue::Integer(val.get().into())),
        _ => Err(build_label_error(
            format!("{nu_val:?} is not convertible"),
            span,
        )),
    }
}

fn convert_nu_dict(record: &Record) -> Result<PlistValue, LabeledError> {
    Ok(PlistValue::Dictionary(
        record
            .iter()
            .map(|(k, v)| convert_nu_value(v).map(|v| (k.to_owned(), v)))
            .collect::<Result<_, _>>()?,
    ))
}

#[cfg(test)]
mod test {

    use nu_plugin_test_support::PluginTest;
    use nu_protocol::ShellError;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        let plugin = FormatCmdsPlugin {};
        let cmd = IntoPlist {};

        let mut plugin_test = PluginTest::new("polars", plugin.into())?;
        plugin_test.test_command_examples(&cmd)
    }

    // The plist crate panics on a date before year 0, so those are errors.
    #[test]
    fn dates_before_year_0_are_errors() {
        let date = |text: &str| NuValue::test_date(text.parse().unwrap());
        // Windows' clock starts in 1601, so earlier years can't become a plist date there.
        assert!(convert_nu_value(&date("1601-01-01T00:00:00Z")).is_ok());
        assert!(convert_nu_value(&date("9999-12-30T22:00:00Z")).is_ok());
        assert!(convert_nu_value(&date("-0001-12-31T23:59:59Z")).is_err());
    }
}
