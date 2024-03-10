use super::{PersistentPlugin, PluginExecutionCommandContext, PluginSource};
use crate::protocol::{CallInfo, EvaluatedCall};
use std::sync::Arc;

use nu_engine::get_eval_expression;

use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{ast::Call, PluginSignature, Signature};
use nu_protocol::{Example, PipelineData, PluginIdentity, RegisteredPlugin, ShellError};

#[doc(hidden)] // Note: not for plugin authors / only used in nu-parser
#[derive(Clone)]
pub struct PluginDeclaration {
    name: String,
    signature: PluginSignature,
    source: PluginSource,
}

impl PluginDeclaration {
    pub fn new(plugin: &Arc<PersistentPlugin>, signature: PluginSignature) -> Self {
        Self {
            name: signature.sig.name.clone(),
            signature,
            source: PluginSource::new(plugin),
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
        let eval_expression = get_eval_expression(engine_state);

        // Create the EvaluatedCall to send to the plugin first - it's best for this to fail early,
        // before we actually try to run the plugin command
        let evaluated_call =
            EvaluatedCall::try_from_call(call, engine_state, stack, eval_expression)?;

        // Get the engine config
        let engine_config = nu_engine::get_config(engine_state, stack);

        // Get, or start, the plugin.
        let plugin = self
            .source
            .persistent(None)
            .and_then(|p| {
                // Set the garbage collector config from the local config before running
                p.set_gc_config(engine_config.plugin_gc.get(p.identity().name()));
                p.get(|| {
                    // We need the current environment variables for `python` based plugins. Or
                    // we'll likely have a problem when a plugin is implemented in a virtual Python
                    // environment.
                    nu_engine::env::env_to_strings(engine_state, stack)
                })
            })
            .map_err(|err| {
                let decl = engine_state.get_decl(call.decl_id);
                ShellError::GenericError {
                    error: format!("Unable to spawn plugin for `{}`", decl.name()),
                    msg: err.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                }
            })?;

        // Create the context to execute in - this supports engine calls and custom values
        let context = Arc::new(PluginExecutionCommandContext::new(
            self.source.identity.clone(),
            engine_state,
            stack,
            call,
        ));

        plugin.run(
            CallInfo {
                name: self.name.clone(),
                call: evaluated_call,
                input,
            },
            context,
        )
    }

    fn is_plugin(&self) -> bool {
        true
    }

    fn plugin_identity(&self) -> Option<&PluginIdentity> {
        Some(&self.source.identity)
    }
}
