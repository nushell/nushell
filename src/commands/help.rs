use crate::commands::PerItemCommand;
use crate::data::{command_dict, TaggedDictBuilder};
use crate::errors::ShellError;
use crate::parser::registry::{self, NamedType, PositionalType};
use crate::prelude::*;

pub struct Help;

impl PerItemCommand for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> registry::Signature {
        Signature::build("help").rest(SyntaxShape::Any, "the name of command(s) to get help on")
    }

    fn usage(&self) -> &str {
        "Display help information about commands."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        _raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        let tag = &call_info.name_tag;

        match call_info.args.nth(0) {
            Some(Tagged {
                item: Value::Primitive(Primitive::String(document)),
                tag,
            }) => {
                let mut help = VecDeque::new();
                if document == "commands" {
                    let mut sorted_names = registry.names();
                    sorted_names.sort();
                    for cmd in sorted_names {
                        let mut short_desc = TaggedDictBuilder::new(tag.clone());
                        let value = command_dict(registry.get_command(&cmd).unwrap(), tag.clone());

                        short_desc.insert("name", cmd);
                        short_desc.insert(
                            "description",
                            value
                                .get_data_by_key("usage".spanned_unknown())
                                .unwrap()
                                .as_string()
                                .unwrap(),
                        );

                        help.push_back(ReturnSuccess::value(short_desc.into_tagged_value()));
                    }
                } else {
                    if let Some(command) = registry.get_command(document) {
                        let mut long_desc = String::new();

                        long_desc.push_str(&command.usage());
                        long_desc.push_str("\n");

                        let signature = command.signature();

                        let mut one_liner = String::new();
                        one_liner.push_str(&signature.name);
                        one_liner.push_str(" ");

                        for positional in &signature.positional {
                            match &positional.0 {
                                PositionalType::Mandatory(name, _m) => {
                                    one_liner.push_str(&format!("<{}> ", name));
                                }
                                PositionalType::Optional(name, _o) => {
                                    one_liner.push_str(&format!("({}) ", name));
                                }
                            }
                        }

                        if signature.rest_positional.is_some() {
                            one_liner.push_str(&format!(" ...args",));
                        }

                        if signature.named.len() > 0 {
                            one_liner.push_str("{flags} ");
                        }

                        long_desc.push_str(&format!("\nUsage:\n  > {}\n", one_liner));

                        if signature.positional.len() > 0 || signature.rest_positional.is_some() {
                            long_desc.push_str("\nparameters:\n");
                            for positional in signature.positional {
                                match positional.0 {
                                    PositionalType::Mandatory(name, _m) => {
                                        long_desc
                                            .push_str(&format!("  <{}> {}\n", name, positional.1));
                                    }
                                    PositionalType::Optional(name, _o) => {
                                        long_desc
                                            .push_str(&format!("  ({}) {}\n", name, positional.1));
                                    }
                                }
                            }
                            if signature.rest_positional.is_some() {
                                long_desc.push_str(&format!(
                                    "  ...args{} {}\n",
                                    if signature.rest_positional.is_some() {
                                        ":"
                                    } else {
                                        ""
                                    },
                                    signature.rest_positional.unwrap().1
                                ));
                            }
                        }
                        if signature.named.len() > 0 {
                            long_desc.push_str("\nflags:\n");
                            for (flag, ty) in signature.named {
                                match ty.0 {
                                    NamedType::Switch => {
                                        long_desc.push_str(&format!(
                                            "  --{}{} {}\n",
                                            flag,
                                            if ty.1.len() > 0 { ":" } else { "" },
                                            ty.1
                                        ));
                                    }
                                    NamedType::Mandatory(m) => {
                                        long_desc.push_str(&format!(
                                            "  --{} <{}> (required parameter){} {}\n",
                                            flag,
                                            m,
                                            if ty.1.len() > 0 { ":" } else { "" },
                                            ty.1
                                        ));
                                    }
                                    NamedType::Optional(o) => {
                                        long_desc.push_str(&format!(
                                            "  --{} <{}>{} {}\n",
                                            flag,
                                            o,
                                            if ty.1.len() > 0 { ":" } else { "" },
                                            ty.1
                                        ));
                                    }
                                }
                            }
                        }

                        help.push_back(ReturnSuccess::value(
                            Value::string(long_desc).tagged(tag.clone()),
                        ));
                    }
                }

                Ok(help.to_output_stream())
            }
            _ => {
                let msg = r#"Welcome to Nushell.

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command

You can also learn more at https://book.nushell.sh"#;

                let mut output_stream = VecDeque::new();

                output_stream.push_back(ReturnSuccess::value(Value::string(msg).tagged(tag)));

                Ok(output_stream.to_output_stream())
            }
        }
    }
}
