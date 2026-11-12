use nu_engine::command_prelude::*;
use nu_engine::env::{convert_env_var, convert_env_vars, ensure_path, is_automatic_env_var, var};
use nu_protocol::engine::env_var_eq;

#[derive(Clone)]
pub struct LoadEnv;

impl Command for LoadEnv {
    fn name(&self) -> &str {
        "load-env"
    }

    fn description(&self) -> &str {
        "Loads an environment update from a record."
    }

    fn extra_description(&self) -> &str {
        "A config record updates the current configuration. Each string value with a `from_string` conversion in $env.ENV_CONVERSIONS is converted, as nushell converts the environment it starts with, and a string PATH becomes a list; `with-env` converts the same way. When a conversion fails, nothing is loaded. A record holding ENV_CONVERSIONS is loaded first and then converts every variable, as assigning it does, so a failed conversion leaves it loaded. With --no-convert, the values are loaded as given."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("load-env")
            .input_output_types(vec![
                (Type::record(), Type::Nothing),
                (Type::Nothing, Type::Nothing),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "no-convert",
                "Load the values as given, without $env.ENV_CONVERSIONS or making PATH a list.",
                None,
            )
            .optional(
                "update",
                SyntaxShape::record(),
                "The record to use for updates.",
            )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg = call.opt(engine_state, stack, 0)?;
        let convert = !call.has_flag(engine_state, stack, "no-convert")?;
        let span = call.head;
        let input_span = input.span().unwrap_or(span);

        let record = if let Some(record) = arg {
            record
        } else if let PipelineData::Value(value, ..) = input
            && let Ok(record) = value.into_record()
        {
            record.into_spanned(input_span)
        } else {
            return Err(ShellError::UnsupportedInput {
                msg: "'load-env' expects a single record".into(),
                input: "value originated from here".into(),
                msg_span: span,
                input_span,
            });
        };

        for (env_var, _) in &record.item {
            if is_automatic_env_var(env_var) {
                return Err(ShellError::AutomaticEnvVarSetManually {
                    envvar_name: env_var.to_owned(),
                    span: record.span,
                });
            }
        }

        let (vars, load) = EnvLoad::prepare(engine_state, stack, record.item, convert)?;

        let mut loaded_config = false;
        for (name, value) in vars {
            if env_var_eq(&name, var::CONFIG) {
                loaded_config = true;
                let name = stack.get_env_var_name(engine_state, &name).into_owned();
                stack.add_env_var(name, value);
            } else {
                stack.add_env_var(name, value);
            }
        }

        if loaded_config {
            stack.update_config(engine_state)?
        }

        load.finish(engine_state, stack)?;

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Load variables from an input stream.",
                example: "{NAME: ABE, AGE: UNKNOWN} | load-env; $env.NAME",
                result: Some(Value::test_string("ABE")),
            },
            Example {
                description: "Load variables from an argument.",
                example: "load-env {NAME: ABE, AGE: UNKNOWN}; $env.NAME",
                result: Some(Value::test_string("ABE")),
            },
            Example {
                description: "Update the configuration from a config record.",
                example: "load-env {config: {show_banner: false}}; $env.config.show_banner",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Load a variable, which its environment conversion turns into a list.",
                example: "$env.ENV_CONVERSIONS = {MY_ENV_VAR: {from_string: { split row ':' }}}; load-env {MY_ENV_VAR: 'foo:bar'}; $env.MY_ENV_VAR",
                result: Some(Value::test_list(vec![
                    Value::test_string("foo"),
                    Value::test_string("bar"),
                ])),
            },
            Example {
                description: "Load a variable as given, without converting it.",
                example: "$env.ENV_CONVERSIONS = {MY_ENV_VAR: {from_string: { split row ':' }}}; load-env --no-convert {MY_ENV_VAR: 'foo:bar'}; $env.MY_ENV_VAR",
                result: Some(Value::test_string("foo:bar")),
            },
        ]
    }
}

/// How `load-env` and `with-env` convert a record they set in the environment, as nushell
/// converts the environment it starts with.
#[derive(Clone, Copy)]
pub(crate) struct EnvLoad {
    /// False with `--no-convert`, which sets the values as given.
    convert: bool,
    /// The record holds ENV_CONVERSIONS, whose conversions apply to every variable once it's set.
    conversions: bool,
    /// The record holds PATH.
    path: bool,
}

impl EnvLoad {
    /// Converts each string value of `record` with its `from_string` closure before any is set,
    /// so a failed conversion sets nothing. `config` stays as it is, and so does every value
    /// without `convert` or when the record holds ENV_CONVERSIONS, which [`EnvLoad::finish`]
    /// applies once it's set.
    pub(crate) fn prepare(
        engine_state: &EngineState,
        stack: &Stack,
        record: Record,
        convert: bool,
    ) -> Result<(Record, Self), ShellError> {
        let holds = |name: &str| record.columns().any(|key| env_var_eq(key, name));
        let load = EnvLoad {
            convert,
            conversions: holds(var::ENV_CONVERSIONS),
            path: holds(var::PATH),
        };
        if !convert || load.conversions {
            return Ok((record, load));
        }
        let record = record
            .into_iter()
            .map(|(name, value)| {
                if env_var_eq(&name, var::CONFIG) {
                    Ok((name, value))
                } else {
                    convert_env_var(engine_state, stack, &name, value).map(|value| (name, value))
                }
            })
            .collect::<Result<Record, ShellError>>()?;
        Ok((record, load))
    }

    /// Runs once the record is set in `stack`: new ENV_CONVERSIONS convert every variable, as
    /// assigning `$env.ENV_CONVERSIONS` does, and otherwise a string PATH becomes a list.
    pub(crate) fn finish(
        self,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<(), ShellError> {
        if !self.convert {
            return Ok(());
        }
        if self.conversions {
            if let Some(conversions) = stack
                .get_env_var(engine_state, var::ENV_CONVERSIONS)
                .cloned()
            {
                convert_env_vars(stack, engine_state, &conversions)?;
            }
        } else if self.path
            && let Some(err) = ensure_path(engine_state, stack)
        {
            return Err(err);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LoadEnv;

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(LoadEnv)
    }
}
