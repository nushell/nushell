use nu_engine::{command_prelude::*, get_eval_expression};
use nu_plugin_protocol::{CallInfo, EvaluatedCall};
use nu_protocol::{PluginIdentity, PluginSignature, engine::CommandType};
use std::sync::Arc;

use crate::{GetPlugin, PluginExecutionCommandContext, PluginSource};

/// The command declaration proxy used within the engine for all plugin commands.
#[derive(Clone)]
pub struct PluginDeclaration {
    name: String,
    signature: PluginSignature,
    source: PluginSource,
}

impl PluginDeclaration {
    pub fn new(plugin: Arc<dyn GetPlugin>, signature: PluginSignature) -> Self {
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

    fn description(&self) -> &str {
        self.signature.sig.description.as_str()
    }

    fn extra_description(&self) -> &str {
        self.signature.sig.extra_description.as_str()
    }

    fn search_terms(&self) -> Vec<&str> {
        self.signature
            .sig
            .search_terms
            .iter()
            .map(|term| term.as_str())
            .collect()
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
        let engine_config = stack.get_config(engine_state);

        // Get, or start, the plugin.
        let plugin = self
            .source
            .persistent(None)
            .and_then(|p| {
                // Set the garbage collector config from the local config before running
                p.set_gc_config(engine_config.plugin_gc.get(p.identity().name()));
                p.get_plugin(Some((engine_state, stack)))
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
        let mut context = PluginExecutionCommandContext::new(
            self.source.identity.clone(),
            engine_state,
            stack,
            call,
        );

        plugin.run(
            CallInfo {
                name: self.name.clone(),
                call: evaluated_call,
                input,
            },
            &mut context,
        )
    }

    fn command_type(&self) -> CommandType {
        CommandType::Plugin
    }

    fn plugin_identity(&self) -> Option<&PluginIdentity> {
        Some(&self.source.identity)
    }
}
