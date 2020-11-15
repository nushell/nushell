use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};

use base64::{encode, encode_config};

#[derive(Deserialize)]
pub struct Arguments {
    pub character_type: Option<Tagged<String>>,
    pub encode: Tagged<bool>,
    pub decode: Tagged<bool>,
    pub rest: Vec<ColumnPath>,
}

pub struct SubCommand;


#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "hash base64"
    }

    fn signature(&self) -> Signature {
        Signature::build("hash base64")
            .named(
                "character-type",
                SyntaxShape::String,
                "specify the character rules for encoding the input",
                Some('c'),
            )
            .switch(
                "encode",
                "encode the input as base64",
                Some('p'),
            )
            .switch(
                "dencode",
                "decode the input from base64",
                Some('d'),
            )
            .rest(
                SyntaxShape::ColumnPath,
                "optionally base64 encode / decode data by column paths",
            )
    }

    fn usage(&self) -> &str {
        "base64 encode or decode a value"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Base64 encode a string",
            example: "my_username:my_password | hash base64",
            result: Some(vec![
                UntaggedValue::string("dXNlcm5hbWU6cGFzc3dvcmQ=").into_untagged_value()
            ]),
        }]
    }
}

#[derive(Clone)]
pub struct EncodingConfig(String);

async fn operate(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let (Arguments { 
        encode,
        decode,
        character_type,
        rest, 
        }, 
        input) = args.process(&registry).await?;

    if encode.item && decode.item {
        return Ok(OutputStream::one(Err(ShellError::labeled_error(
            "only one of --decode and --encode can be used",
            "conflicting flags",
            name,
        ))));
    }

    let encoding_inner = match character_type {
        Some(inner_tag) => inner_tag.item().to_string().clone(),
        None => "standard".to_string()
    };

    let encoding_config = EncodingConfig(encoding_inner);
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &encoding_config, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let config = encoding_config.clone();
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &config, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, encoding: &EncodingConfig, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let encoding_config = &encoding.0;

            if encoding_config == "standard" {
                return Ok(UntaggedValue::string(encode_config(&s, base64::STANDARD)).into_value(tag));
            }

            else if encoding_config == "standard-no-padding" {
                return Ok(UntaggedValue::string(encode_config(&s, base64::STANDARD_NO_PAD)).into_value(tag));
            }

            else if encoding_config == "url-safe" {
                return Ok(UntaggedValue::string(encode_config(&s, base64::URL_SAFE)).into_value(tag));
            }

            else {
                return Err(ShellError::labeled_error(
                    "value is not an accepted character set",
                    format!("character-set {}", encoding_config),
                    tag.into().span,
                ));
            }

            Ok(UntaggedValue::string(encode(&s)).into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{action, EncodingConfig};
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn base64_encode_standard() {
        let word = string("username:password");
        let expected = UntaggedValue::string("dXNlcm5hbWU6cGFzc3dvcmQ=").into_untagged_value();

        let actual = action(&word, &EncodingConfig("standard".to_string()), Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn base64_encode_standard_no_padding() {
        let word = string("username:password");
        let expected = UntaggedValue::string("dXNlcm5hbWU6cGFzc3dvcmQ").into_untagged_value();

        let actual = action(&word, &EncodingConfig("standard-no-padding".to_string()), Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn base64_encode_url_safe() {
        let word = string("this is for url");
        let expected = UntaggedValue::string("dGhpcyBpcyBmb3IgdXJs").into_untagged_value();

        let actual = action(&word, &EncodingConfig("url-safe".to_string()), Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
