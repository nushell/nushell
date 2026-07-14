use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use nu_system::build_kill_command;
use std::process::Stdio;

#[derive(Clone)]
pub struct Kill;

impl Command for Kill {
    fn name(&self) -> &str {
        "kill"
    }

    fn description(&self) -> &str {
        "Kill a process using its process ID."
    }

    fn signature(&self) -> Signature {
        let signature = Signature::build("kill")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .rest(
                "pid",
                SyntaxShape::Int,
                "Process ids of processes that are to be killed.",
            )
            .switch("force", "Forcefully kill the process.", Some('f'))
            .switch("quiet", "Won't print anything to the console.", Some('q'))
            .category(Category::Platform);

        if cfg!(windows) {
            return signature;
        }

        signature.named(
            "signal",
            SyntaxShape::Int,
            "Signal decimal number to be sent instead of the default 15 (unsupported on Windows).",
            Some('s'),
        )
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["stop", "end", "close", "taskkill"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pids: Vec<Spanned<i64>> = call.rest(engine_state, stack, 0)?;
        let force: bool = call.has_flag(engine_state, stack, "force")?;
        let signal: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "signal")?;
        let quiet: bool = call.has_flag(engine_state, stack, "quiet")?;

        if cfg!(unix)
            && signal.is_none()
            && let Some((span, signal_number)) = pids.iter().find_map(|pid| {
                inferred_signal_number(engine_state.get_span_contents(pid.span), pid.item)
                    .map(|signal_number| (pid.span, signal_number))
            })
        {
            return Err(ShellError::IncorrectValue {
                msg: format!(
                    "negative pid shorthand is not supported; use `kill -s {signal_number} <pid>`"
                ),
                val_span: span,
                call_span: call.head,
            });
        }

        if pids.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "pid".to_string(),
                span: call.arguments_span(),
            });
        }

        if cfg!(unix)
            && let (
                true,
                Some(Spanned {
                    item: _,
                    span: signal_span,
                }),
            ) = (force, signal)
        {
            return Err(ShellError::IncompatibleParameters {
                left_message: "force".to_string(),
                left_span: call
                    .get_flag_span(stack, "force")
                    .expect("Had flag force, but didn't have span for flag"),
                right_message: "signal".to_string(),
                right_span: Span::merge(
                    call.get_flag_span(stack, "signal")
                        .expect("Had flag signal, but didn't have span for flag"),
                    signal_span,
                ),
            });
        };

        let mut cmd = build_kill_command(
            force,
            pids.iter().copied().map(|spanned| spanned.item),
            signal.map(|spanned| spanned.item as u32),
        );

        // pipe everything to null
        if quiet {
            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
        }

        let output = cmd.output().map_err(|e| {
            ShellError::Generic(GenericError::new(
                "failed to execute shell command",
                e.to_string(),
                call.head,
            ))
        })?;

        if !quiet && !output.status.success() {
            return Err(ShellError::Generic(GenericError::new(
                "process didn't terminate successfully",
                String::from_utf8(output.stderr).unwrap_or_default(),
                call.head,
            )));
        }

        let mut output = String::from_utf8(output.stdout).map_err(|e| {
            ShellError::Generic(GenericError::new(
                "failed to convert output to string",
                e.to_string(),
                call.head,
            ))
        })?;

        output.truncate(output.trim_end().len());

        if output.is_empty() {
            Ok(Value::nothing(call.head).into_pipeline_data())
        } else {
            Ok(Value::string(output, call.head).into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Kill the pid using the most memory.",
                example: "ps | sort-by mem | last | kill $in.pid",
                result: None,
            },
            Example {
                description: "Force kill a given pid.",
                example: "kill --force 12345",
                result: None,
            },
            #[cfg(not(target_os = "windows"))]
            Example {
                description: "Send INT signal.",
                example: "kill -s 2 12345",
                result: None,
            },
        ]
    }
}

fn inferred_signal_number(raw: &[u8], pid: i64) -> Option<u64> {
    if pid < 0 {
        return Some(pid.unsigned_abs());
    }

    let stripped = std::str::from_utf8(raw).ok()?.strip_prefix('-')?;

    if stripped.is_empty() || !stripped.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    stripped.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(Kill)
    }

    #[test]
    fn inferred_signal_number_detects_negative_pid_shorthand() {
        assert_eq!(inferred_signal_number(b"-0", 0), Some(0));
        assert_eq!(inferred_signal_number(b"-00", 0), Some(0));
        assert_eq!(inferred_signal_number(b"-9", -9), Some(9));
        assert_eq!(inferred_signal_number(b"0", 0), None);
        assert_eq!(inferred_signal_number(b"9", 9), None);
    }
}
