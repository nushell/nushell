use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};

use base64::{decode_config, encode_config};

#[derive(Deserialize)]
pub struct Arguments {
    pub rest: Vec<ColumnPath>,
    pub character_set: Option<Tagged<String>>,
    pub encode: Tagged<bool>,
    pub decode: Tagged<bool>,
}

#[derive(Clone)]
pub struct Base64Config {
    pub character_set: String,
    pub action_type: ActionType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActionType {
    Encode,
    Decode,
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
                "character_set",
                SyntaxShape::String,
                "specify the character rules for encoding the input.\n\
                        \tValid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding',\
                        'binhex', 'bcrypt', 'crypt'",
                Some('c'),
            )
            .switch(
                "encode", 
                "encode the input as base64. This is the default behavior if not specified.", 
                Some('e')
            )
            .switch(
                "decode", 
                "decode the input from base64", 
                Some('d'))
            .rest(
                SyntaxShape::ColumnPath,
                "optionally base64 encode / decode data by column paths",
            )
    }

    fn usage(&self) -> &str {
        "base64 encode or decode a value"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Base64 encode a string with default settings",
                example: "echo 'username:password' | hash base64",
                result: Some(vec![
                    UntaggedValue::string("dXNlcm5hbWU6cGFzc3dvcmQ=").into_untagged_value()
                ]),
            },
            Example {
                description: "Base64 encode a string with the binhex character set",
                example: "echo 'username:password' | hash base64 --character_set binhex --encode",
                result: Some(vec![
                    UntaggedValue::string("F@0NEPjJD97kE'&bEhFZEP3").into_untagged_value()
                ]),
            },
            Example {
                description: "Base64 decode a value",
                example: "echo 'dXNlcm5hbWU6cGFzc3dvcmQ=' | hash base64 --decode",
                result: Some(vec![
                    UntaggedValue::string("username:password").into_untagged_value()
                ]),
            },
        ]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = &args.call_info.name_tag.clone();

    let (
        Arguments {
            encode,
            decode,
            character_set,
            rest,
        },
        input,
    ) = args.process().await?;

    if encode.item && decode.item {
        return Ok(OutputStream::one(Err(ShellError::labeled_error(
            "only one of --decode and --encode flags can be used",
            "conflicting flags",
            name_tag,
        ))));
    }

    // Default the action to be encoding if no flags are specified.
    let action_type = if *decode.item() {
        ActionType::Decode
    } else {
        ActionType::Encode
    };

    // Default the character set to standard if the argument is not specified.
    let character_set = match character_set {
        Some(inner_tag) => inner_tag.item().to_string(),
        None => "standard".to_string(),
    };

    let encoding_config = Base64Config {
        character_set,
        action_type,
    };

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

fn action(
    input: &Value,
    base64_config: &Base64Config,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let base64_config_enum: base64::Config = if &base64_config.character_set == "standard" {
                base64::STANDARD
            } else if &base64_config.character_set == "standard-no-padding" {
                base64::STANDARD_NO_PAD
            } else if &base64_config.character_set == "url-safe" {
                base64::URL_SAFE
            } else if &base64_config.character_set == "url-safe-no-padding" {
                base64::URL_SAFE_NO_PAD
            } else if &base64_config.character_set == "binhex" {
                base64::BINHEX
            } else if &base64_config.character_set == "bcrypt" {
                base64::BCRYPT
            } else if &base64_config.character_set == "crypt" {
                base64::CRYPT
            } else {
                return Err(ShellError::labeled_error(
                    "value is not an accepted character set",
                    format!(
                        "{} is not a valid character-set.\nPlease use `help hash base64` to see a list of valid character sets.", 
                        &base64_config.character_set
                    ),
                    tag.into().span,
                ));
            };

            match base64_config.action_type {
                ActionType::Encode => Ok(UntaggedValue::string(encode_config(
                    &s,
                    base64_config_enum,
                ))
                .into_value(tag)),
                ActionType::Decode => {
                    let decode_result = decode_config(&s, base64_config_enum);

                    match decode_result {
                        Ok(decoded_value) => Ok(UntaggedValue::string(
                            std::string::String::from_utf8_lossy(&decoded_value),
                        )
                        .into_value(tag)),
                        Err(_) => Err(ShellError::labeled_error(
                            "value could not be base64 decoded",
                            format!(
                                "invalid base64 input for character set {}",
                                &base64_config.character_set
                            ),
                            tag.into().span,
                        )),
                    }
                }
            }
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
    use super::{action, ActionType, Base64Config};
    use nu_protocol::UntaggedValue;
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn base64_encode_standard() {
        let word = string("username:password");
        let expected = UntaggedValue::string("dXNlcm5hbWU6cGFzc3dvcmQ=").into_untagged_value();

        let actual = action(
            &word,
            &Base64Config {
                character_set: "standard".to_string(),
                action_type: ActionType::Encode,
            },
            Tag::unknown(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_standard_no_padding() {
        let word = string("username:password");
        let expected = UntaggedValue::string("dXNlcm5hbWU6cGFzc3dvcmQ").into_untagged_value();

        let actual = action(
            &word,
            &Base64Config {
                character_set: "standard-no-padding".to_string(),
                action_type: ActionType::Encode,
            },
            Tag::unknown(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_encode_url_safe() {
        let word = string("this is for url");
        let expected = UntaggedValue::string("dGhpcyBpcyBmb3IgdXJs").into_untagged_value();

        let actual = action(
            &word,
            &Base64Config {
                character_set: "url-safe".to_string(),
                action_type: ActionType::Encode,
            },
            Tag::unknown(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn base64_decode_binhex() {
        let word = string("A5\"KC9jRB@IIF'8bF!");
        let expected = UntaggedValue::string("a binhex test").into_untagged_value();

        let actual = action(
            &word,
            &Base64Config {
                character_set: "binhex".to_string(),
                action_type: ActionType::Decode,
            },
            Tag::unknown(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}
