use crate::EvaluatedCall;

use super::interface::{make_call_input_from_pipeline_data, PluginExecutionNushellContext};
use super::{create_command, make_plugin_interface};
use crate::protocol::{CallInfo, PluginCall};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nu_engine::eval_block;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{ast::Call, PluginSignature, Signature};
use nu_protocol::{Example, PipelineData, ShellError, Value};

#[doc(hidden)] // Note: not for plugin authors / only used in nu-parser
#[derive(Clone)]
pub struct PluginDeclaration {
    name: String,
    signature: PluginSignature,
    filename: PathBuf,
    shell: Option<PathBuf>,
}

impl PluginDeclaration {
    pub fn new(filename: PathBuf, signature: PluginSignature, shell: Option<PathBuf>) -> Self {
        Self {
            name: signature.sig.name.clone(),
            signature,
            filename,
            shell,
        }
    }
}

impl Command for PluginDeclaration {
    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> Signature {
        self.signature.sig.clone()
    }

    fn usage(&self) -> &str {
        self.signature.sig.usage.as_str()
    }

    fn extra_usage(&self) -> &str {
        self.signature.sig.extra_usage.as_str()
    }

    fn search_terms(&self) -> Vec<&str> {
        self.signature
            .sig
            .search_terms
            .iter()
            .map(|term| term.as_str())
            .collect()
    }

    fn examples(&self) -> Vec<Example> {
        let mut res = vec![];
        for e in self.signature.examples.iter() {
            res.push(Example {
                example: &e.example,
                description: &e.description,
                result: e.result.clone(),
            })
        }
        res
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Call the command with self path
        // Decode information from plugin
        // Create PipelineData
        let source_file = Path::new(&self.filename);
        let mut plugin_cmd = create_command(source_file, self.shell.as_deref());
        // We need the current environment variables for `python` based plugins
        // Or we'll likely have a problem when a plugin is implemented in a virtual Python environment.
        let current_envs = nu_engine::env::env_to_strings(engine_state, stack).unwrap_or_default();
        plugin_cmd.envs(current_envs);

        let mut child = plugin_cmd.spawn().map_err(|err| {
            let decl = engine_state.get_decl(call.decl_id);
            ShellError::GenericError {
                error: format!("Unable to spawn plugin for {}", decl.name()),
                msg: format!("{err}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }
        })?;

        let call_input = make_call_input_from_pipeline_data(&input, &self.name, &self.filename)?;

        // Fetch the configuration for a plugin
        //
        // The `plugin` must match the registered name of a plugin.  For
        // `register nu_plugin_example` the plugin config lookup uses `"example"`
        let config = self
            .filename
            .file_stem()
            .and_then(|file| {
                file.to_string_lossy()
                    .clone()
                    .strip_prefix("nu_plugin_")
                    .map(|name| {
                        nu_engine::get_config(engine_state, stack)
                            .plugins
                            .get(name)
                            .cloned()
                    })
            })
            .flatten()
            .map(|value| {
                let span = value.span();
                match value {
                    Value::Closure { val, .. } => {
                        let input = PipelineData::Empty;

                        let block = engine_state.get_block(val.block_id).clone();
                        let mut stack = stack.captures_to_stack(val.captures);

                        match eval_block(engine_state, &mut stack, &block, input, false, false) {
                            Ok(v) => v.into_value(span),
                            Err(e) => Value::error(e, call.head),
                        }
                    }
                    _ => value.clone(),
                }
            });

        let plugin_call = PluginCall::Run(CallInfo {
            name: self.name.clone(),
            call: EvaluatedCall::try_from_call(call, engine_state, stack)?,
            input: call_input,
            config,
        });

        let context = Arc::new(PluginExecutionNushellContext::new(
            self.filename.clone(),
            self.shell.clone(),
            engine_state,
            stack,
            call,
        ));

        let interface = make_plugin_interface(&mut child, Some(context))?;
        let interface_clone = interface.clone();

        // Write the call and stream(s) on another thread. If we don't start reading immediately,
        // we could block the child from being able to read stdin because it's trying to write
        // something on stdout and its buffer is full.
        std::thread::spawn(move || {
            interface_clone.write_call(plugin_call)?;
            match input {
                PipelineData::Value(_, _) => Ok(()),
                PipelineData::ListStream(list_stream, _) => {
                    interface_clone.write_full_list_stream(list_stream)
                }
                PipelineData::ExternalStream {
                    stdout,
                    stderr,
                    exit_code,
                    ..
                } => interface_clone.write_full_external_stream(stdout, stderr, exit_code),
                PipelineData::Empty => Ok(()),
            }
        });

        // Spawn a thread just to wait for the child.
        std::thread::spawn(move || child.wait());

        // Return the pipeline data from the response
        let response = interface.read_call_response()?;
        interface.make_pipeline_data(response)
    }

    fn is_plugin(&self) -> Option<(&Path, Option<&Path>)> {
        Some((&self.filename, self.shell.as_deref()))
    }
}
