use super::Options;

use nu_command::commands::{loglevels, testbins, NuSignature as Nu};
use nu_command::commands::{Autoview, Pivot, Table, Version as NuVersion};
use nu_engine::{whole_stream_command, EvaluationContext};
use nu_errors::ShellError;
use nu_protocol::hir::{ClassifiedCommand, InternalCommand, NamedValue};
use nu_protocol::UntaggedValue;
use nu_source::Tag;

pub struct NuParser {
    context: EvaluationContext,
}

pub trait OptionsParser {
    fn parse(&self, input: &str) -> Result<Options, ShellError>;
    fn context(&self) -> &EvaluationContext;
}

impl NuParser {
    pub fn new() -> Self {
        let context = EvaluationContext::basic();
        context.add_commands(vec![
            whole_stream_command(Nu {}),
            whole_stream_command(NuVersion {}),
            whole_stream_command(Autoview {}),
            whole_stream_command(Pivot {}),
            whole_stream_command(Table {}),
        ]);

        Self { context }
    }
}

impl OptionsParser for NuParser {
    fn context(&self) -> &EvaluationContext {
        &self.context
    }

    fn parse(&self, input: &str) -> Result<Options, ShellError> {
        let options = Options::default();
        let (lite_result, _err) = nu_parser::lex(input, 0, nu_parser::NewlineMode::Normal);
        let (lite_result, _err) = nu_parser::parse_block(lite_result);

        let (parsed, err) = nu_parser::classify_block(&lite_result, &self.context.scope);

        if let Some(reason) = err {
            return Err(reason.into());
        }

        match parsed.block[0].pipelines[0].list[0] {
            ClassifiedCommand::Internal(InternalCommand { ref args, .. }) => {
                if let Some(ref params) = args.named {
                    params.iter().for_each(|(k, v)| {
                        let value = match v {
                            NamedValue::AbsentSwitch => {
                                Some(UntaggedValue::from(false).into_untagged_value())
                            }
                            NamedValue::PresentSwitch(span) => {
                                Some(UntaggedValue::from(true).into_value(Tag::from(span)))
                            }
                            NamedValue::AbsentValue => None,
                            NamedValue::Value(span, exprs) => {
                                let value = nu_engine::evaluate_baseline_expr(exprs, &self.context)
                                    .expect("value");
                                Some(value.value.into_value(Tag::from(span)))
                            }
                        };

                        let value =
                            value
                                .map(|v| match k.as_ref() {
                                    "testbin" => {
                                        if let Ok(name) = v.as_string() {
                                            if testbins().iter().any(|n| name == *n) {
                                                Some(v)
                                            } else {
                                                Some(
                                                    UntaggedValue::Error(
                                                        ShellError::untagged_runtime_error(
                                                            format!("{} is not supported.", name),
                                                        ),
                                                    )
                                                    .into_value(v.tag),
                                                )
                                            }
                                        } else {
                                            Some(v)
                                        }
                                    }
                                    "loglevel" => {
                                        if let Ok(name) = v.as_string() {
                                            if loglevels().iter().any(|n| name == *n) {
                                                Some(v)
                                            } else {
                                                Some(
                                                    UntaggedValue::Error(
                                                        ShellError::untagged_runtime_error(
                                                            format!("{} is not supported.", name),
                                                        ),
                                                    )
                                                    .into_value(v.tag),
                                                )
                                            }
                                        } else {
                                            Some(v)
                                        }
                                    }
                                    _ => Some(v),
                                })
                                .flatten();

                        if let Some(value) = value {
                            options.put(&k, value);
                        }
                    });
                }

                let mut positional_args = vec![];

                if let Some(positional) = &args.positional {
                    for pos in positional {
                        let result = nu_engine::evaluate_baseline_expr(pos, &self.context)?;
                        positional_args.push(result);
                    }
                }

                if !positional_args.is_empty() {
                    options.put(
                        "args",
                        UntaggedValue::Table(positional_args).into_untagged_value(),
                    );
                }
            }
            ClassifiedCommand::Error(ref reason) => {
                return Err(reason.clone().into());
            }
            _ => return Err(ShellError::untagged_runtime_error("unrecognized command")),
        }

        Ok(options)
    }
}
