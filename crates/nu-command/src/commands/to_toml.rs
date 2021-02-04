use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{Primitive, ReturnSuccess, Signature, UnspannedPathMember, UntaggedValue, Value};

pub struct ToTOML;

#[async_trait]
impl WholeStreamCommand for ToTOML {
    fn name(&self) -> &str {
        "to toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to toml")
    }

    fn usage(&self) -> &str {
        "Convert table into .toml text"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_toml(args).await
    }
    // TODO: add an example here. What commands to run to get a Row(Dictionary)?
    // fn examples(&self) -> Vec<Example> {
    //     vec![
    //         Example {
    //             description:
    //                 "Outputs an TOML string representing TOML document",
    //             example: "echo [1 2 3] | to json",
    //             result: Some(vec![Value::from("[1,2,3]")]),
    //         },
    //     ]
    // }
}

// Helper method to recursively convert nu_protocol::Value -> toml::Value
// This shouldn't be called at the top-level
fn helper(v: &Value) -> Result<toml::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => toml::Value::Boolean(*b),
        UntaggedValue::Primitive(Primitive::Filesize(b)) => {
            if let Some(value) = b.to_i64() {
                toml::Value::Integer(value)
            } else {
                return Err(ShellError::labeled_error(
                    "Value too large to write to toml",
                    "value too large for toml",
                    v.tag.span,
                ));
            }
        }
        UntaggedValue::Primitive(Primitive::Duration(i)) => toml::Value::String(i.to_string()),
        UntaggedValue::Primitive(Primitive::Date(d)) => toml::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => {
            toml::Value::String("<End of Stream>".to_string())
        }
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => {
            toml::Value::String("<Beginning of Stream>".to_string())
        }
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            toml::Value::Float(f.tagged(&v.tag).coerce_into("converting to TOML float")?)
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            toml::Value::Integer(i.tagged(&v.tag).coerce_into("converting to TOML integer")?)
        }
        UntaggedValue::Primitive(Primitive::Nothing) => {
            toml::Value::String("<Nothing>".to_string())
        }
        UntaggedValue::Primitive(Primitive::GlobPattern(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => toml::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::FilePath(s)) => {
            toml::Value::String(s.display().to_string())
        }
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => toml::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => Ok(toml::Value::String(string.clone())),
                    UnspannedPathMember::Int(int) => Ok(toml::Value::Integer(
                        int.tagged(&v.tag)
                            .coerce_into("converting to TOML integer")?,
                    )),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),
        UntaggedValue::Table(l) => toml::Value::Array(collect_values(l)?),
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) => toml::Value::String("<Block>".to_string()),
        UntaggedValue::Primitive(Primitive::Range(_)) => toml::Value::String("<Range>".to_string()),
        UntaggedValue::Primitive(Primitive::Binary(b)) => {
            toml::Value::Array(b.iter().map(|x| toml::Value::Integer(*x as i64)).collect())
        }
        UntaggedValue::Row(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), helper(v)?);
            }
            toml::Value::Table(m)
        }
    })
}

/// Converts a nu_protocol::Value into a toml::Value
/// Will return a Shell Error, if the Nu Value is not a valid top-level TOML Value
pub fn value_to_toml_value(v: &Value) -> Result<toml::Value, ShellError> {
    match &v.value {
        UntaggedValue::Row(o) => {
            let mut m = toml::map::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), helper(v)?);
            }
            Ok(toml::Value::Table(m))
        }
        UntaggedValue::Primitive(Primitive::String(s)) => {
            // Attempt to de-serialize the String
            toml::de::from_str(s).map_err(|_| {
                ShellError::labeled_error(
                    format!("{:?} unable to de-serialize string to TOML", s),
                    "invalid TOML",
                    v.tag(),
                )
            })
        }
        _ => Err(ShellError::labeled_error(
            format!("{:?} is not a valid top-level TOML", v.value),
            "invalid TOML",
            v.tag(),
        )),
    }
}

fn collect_values(input: &[Value]) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(helper(value)?);
    }

    Ok(out)
}

async fn to_toml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let name_tag = args.name_tag();
    let name_span = name_tag.span;
    let input: Vec<Value> = args.input.collect().await;

    let to_process_input = match input.len() {
        x if x > 1 => {
            let tag = input[0].tag.clone();
            vec![Value {
                value: UntaggedValue::Table(input),
                tag,
            }]
        }
        1 => input,
        _ => vec![],
    };

    Ok(
        futures::stream::iter(to_process_input.into_iter().map(move |value| {
            let value_span = value.tag.span;
            match value_to_toml_value(&value) {
                Ok(toml_value) => match toml::to_string(&toml_value) {
                    Ok(x) => ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(x)).into_value(&name_tag),
                    ),
                    _ => Err(ShellError::labeled_error_with_secondary(
                        "Expected a table with TOML-compatible structure.tag() from pipeline",
                        "requires TOML-compatible input",
                        name_span,
                        "originates from here".to_string(),
                        value_span,
                    )),
                },
                _ => Err(ShellError::labeled_error(
                    "Expected a table with TOML-compatible structure from pipeline",
                    "requires TOML-compatible input",
                    &name_tag,
                )),
            }
        }))
        .to_output_stream(),
    )
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::*;
    use nu_protocol::Dictionary;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(ToTOML {})?)
    }

    #[test]
    fn test_value_to_toml_value() {
        //
        // Positive Tests
        //

        // Dictionary -> What we do in "crates/nu-cli/src/data/config.rs" to write the config file
        let mut m = indexmap::IndexMap::new();
        m.insert("rust".to_owned(), Value::from("editor"));
        m.insert("is".to_owned(), Value::nothing());
        m.insert(
            "features".to_owned(),
            UntaggedValue::Table(vec![
                UntaggedValue::string("hello").into_untagged_value(),
                UntaggedValue::string("array").into_untagged_value(),
            ])
            .into_untagged_value(),
        );
        let tv = value_to_toml_value(&UntaggedValue::Row(Dictionary::new(m)).into_untagged_value())
            .expect("Expected Ok from valid TOML dictionary");
        assert_eq!(
            tv.get("features"),
            Some(&toml::Value::Array(vec![
                toml::Value::String("hello".to_owned()),
                toml::Value::String("array".to_owned())
            ]))
        );
        // TOML string
        let tv = value_to_toml_value(&Value::from(
            r#"
            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates
            
            [dependencies]
            rustyline = "4.1.0"
            sysinfo = "0.8.4"
            chrono = { version = "0.4.6", features = ["serde"] }
            "#,
        ))
        .expect("Expected Ok from valid TOML string");
        assert_eq!(
            tv.get("title").unwrap(),
            &toml::Value::String("TOML Example".to_owned())
        );
        //
        // Negative Tests
        //
        value_to_toml_value(&Value::from("not_valid"))
            .expect_err("Expected non-valid toml (String) to cause error!");
        value_to_toml_value(&UntaggedValue::Table(vec![Value::from("1")]).into_untagged_value())
            .expect_err("Expected non-valid toml (Table) to cause error!");
    }
}
