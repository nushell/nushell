use std::path::Path;

use nu_command::{Ls, LsEntryMapper};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{engine::Command, Example, LabeledError, PipelineData, ShellError, Span, Value};

use crate::SELinuxPlugin;

#[derive(Clone)]
pub struct SELinuxLs {
    pub ls: Ls,
}

impl PluginCommand for SELinuxLs {
    type Plugin = SELinuxPlugin;

    fn name(&self) -> &str {
        "selinux ls"
    }

    fn description(&self) -> &str {
        self.ls.description()
    }

    fn search_terms(&self) -> Vec<&str> {
        self.ls.search_terms()
    }

    fn signature(&self) -> nu_protocol::Signature {
        let mut signature = self.ls.signature().switch(
            "context",
            "Get the SELinux security context for each entry, if available",
            Some('Z'),
        );
        signature.name = "selinux ls".into();
        signature
    }

    fn run(
        &self,
        _plugin: &SELinuxPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let security_context = call.has_flag("context")?;

        let get_signals = &|| engine.signals().clone();
        let has_flag = &|flag: &str| call.has_flag(flag);
        let has_pattern_arg = call.has_positional_args;
        let pattern_arg = call.rest(0)?;
        let cwd = engine.get_current_dir()?.into();
        let call_head = call.head;
        let map_entry: &LsEntryMapper = &move |path, record| {
            match record {
                Value::Record { val, internal_span } if security_context => {
                    let mut val = val.into_owned();
                    val.push(
                        "security_context",
                        security_context_value(path, call_head)
                            .unwrap_or(Value::nothing(call_head)), // TODO: consider report_shell_warning
                    );

                    Value::record(val, internal_span)
                }
                _ => record,
            }
        };
        let data = Ls::run_ls(
            call_head,
            get_signals,
            has_flag,
            has_pattern_arg,
            pattern_arg,
            cwd,
            map_entry,
        )?;
        Ok(data)
    }

    fn examples(&self) -> Vec<Example> {
        self.ls.examples()
    }
}

fn security_context_value(_path: &Path, span: Span) -> Result<Value, ShellError> {
    #[cfg(not(target_os = "linux"))]
    return Ok(Value::nothing(span));

    #[cfg(target_os = "linux")]
    {
        use selinux;
        match selinux::SecurityContext::of_path(_path, false, false)
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?
        {
            Some(con) => {
                let bytes = con.as_bytes();
                Ok(Value::string(
                    String::from_utf8_lossy(&bytes[0..bytes.len().saturating_sub(1)]),
                    span,
                ))
            }
            None => Ok(Value::nothing(span)),
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use crate::SELinuxPlugin;

    use nu_command::{All, Each, External, First, Join, Lines, SplitColumn, StrTrim};
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::{engine::Command, ShellError, Value};
    use std::{env, sync::Arc};

    #[test]
    fn returns_correct_security_context() -> Result<(), ShellError> {
        let plugin: Arc<SELinuxPlugin> = Arc::new(SELinuxPlugin {});
        let mut plugin_test = PluginTest::new("selinux", plugin)?;

        let engine_state = plugin_test.engine_state_mut();
        engine_state.add_env_var("PWD".to_owned(), Value::test_string("/"));
        engine_state.add_env_var("PATH".into(), Value::test_string(env::var("PATH").unwrap()));

        let deps: Vec<Box<dyn Command>> = vec![
            Box::new(External),
            Box::new(Lines),
            Box::new(Each),
            Box::new(StrTrim),
            Box::new(SplitColumn),
            Box::new(First),
            Box::new(Join),
            Box::new(All),
        ];
        for decl in deps {
            plugin_test.add_decl(decl)?;
        }
        let input = "
            ^ls -Z / | lines | each { |e| $e | str trim | split column ' ' 'coreutils_scontext' 'name' | first } \
            | join (selinux ls -sZ /) name \
            | all { |e|
                let valid = $e.coreutils_scontext == '?' or $e.security_context == $e.coreutils_scontext
                if not $valid {
                    error make { msg: $'For entry ($e.name) expected ($e.coreutils_scontext), got ($e.security_context)' }
                }
            }";
        plugin_test.eval(input)?;
        Ok(())
    }
}
