use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{ast::Call, span};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Spanned, SyntaxShape, Value,
};
use std::process::{Command as CommandSys, Stdio};

#[derive(Clone)]
pub struct Kill;

impl Command for Kill {
    fn name(&self) -> &str {
        "kill"
    }

    fn usage(&self) -> &str {
        "Kill a process using the process id."
    }

    fn signature(&self) -> Signature {
        let signature = Signature::build("kill")
            .required(
                "pid",
                SyntaxShape::Int,
                "process id of process that is to be killed",
            )
            .rest("rest", SyntaxShape::Int, "rest of processes to kill")
            .switch("force", "forcefully kill the process", Some('f'))
            .switch("quiet", "won't print anything to the console", Some('q'))
            .category(Category::Platform);

        if cfg!(windows) {
            return signature;
        }

        signature.named(
            "signal",
            SyntaxShape::Int,
            "signal decimal number to be sent instead of the default 15 (unsupported on Windows)",
            Some('s'),
        )
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["stop", "end", "close"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pid: i64 = call.req(engine_state, stack, 0)?;
        let rest: Vec<i64> = call.rest(engine_state, stack, 1)?;
        let force: bool = call.has_flag("force");
        let signal: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "signal")?;
        let quiet: bool = call.has_flag("quiet");

        let mut cmd = if cfg!(windows) {
            let mut cmd = CommandSys::new("taskkill");

            if force {
                cmd.arg("/F");
            }

            cmd.arg("/PID");
            cmd.arg(pid.to_string());

            // each pid must written as `/PID 0` otherwise
            // taskkill will act as `killall` unix command
            for id in &rest {
                cmd.arg("/PID");
                cmd.arg(id.to_string());
            }

            cmd
        } else {
            let mut cmd = CommandSys::new("kill");
            if force {
                if let Some(Spanned {
                    item: _,
                    span: signal_span,
                }) = signal
                {
                    return Err(ShellError::IncompatibleParameters {
                        left_message: "force".to_string(),
                        left_span: call
                            .get_named_arg("force")
                            .ok_or_else(|| {
                                ShellError::GenericError(
                                    "Flag error".into(),
                                    "flag force not found".into(),
                                    Some(call.head),
                                    None,
                                    Vec::new(),
                                )
                            })?
                            .span,
                        right_message: "signal".to_string(),
                        right_span: span(&[
                            call.get_named_arg("signal")
                                .ok_or_else(|| {
                                    ShellError::GenericError(
                                        "Flag error".into(),
                                        "flag signal not found".into(),
                                        Some(call.head),
                                        None,
                                        Vec::new(),
                                    )
                                })?
                                .span,
                            signal_span,
                        ]),
                    });
                }
                cmd.arg("-9");
            } else if let Some(signal_value) = signal {
                cmd.arg(format!("-{}", signal_value.item));
            }

            cmd.arg(pid.to_string());

            cmd.args(rest.iter().map(move |id| id.to_string()));

            cmd
        };

        // pipe everything to null
        if quiet {
            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
        }

        let output = cmd.output().map_err(|e| {
            ShellError::GenericError(
                "failed to execute shell command".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?;

        if !quiet && !output.status.success() {
            return Err(ShellError::GenericError(
                "process didn't terminate successfully".into(),
                String::from_utf8(output.stderr).unwrap_or_default(),
                Some(call.head),
                None,
                Vec::new(),
            ));
        }

        let val = String::from(
            String::from_utf8(output.stdout)
                .map_err(|e| {
                    ShellError::GenericError(
                        "failed to convert output to string".into(),
                        e.to_string(),
                        Some(call.head),
                        None,
                        Vec::new(),
                    )
                })?
                .trim_end(),
        );
        if val.is_empty() {
            Ok(Value::Nothing { span: call.head }.into_pipeline_data())
        } else {
            Ok(vec![Value::String {
                val,
                span: call.head,
            }]
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Kill the pid using the most memory",
                example: "ps | sort-by mem | last | kill $in.pid",
                result: None,
            },
            Example {
                description: "Force kill a given pid",
                example: "kill --force 12345",
                result: None,
            },
            Example {
                description: "Send INT signal",
                example: "kill -s 2 12345",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Kill;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Kill {})
    }
}
