use crate::commands::command::{CallInfo, Sink, SinkCommandArgs, UnevaluatedCallInfo};
use crate::parser::{
    hir,
    registry::{self, CommandConfig},
    Span,
};
use crate::prelude::*;

use derive_new::new;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpanSource {
    Url(String),
    File(String),
    Source(Text),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMap(HashMap<Uuid, SpanSource>);

impl SourceMap {
    pub fn insert(&mut self, uuid: Uuid, span_source: SpanSource) {
        self.0.insert(uuid, span_source);
    }

    pub fn get(&self, uuid: &Uuid) -> Option<&SpanSource> {
        self.0.get(uuid)
    }

    pub fn new() -> SourceMap {
        SourceMap(HashMap::new())
    }
}

#[derive(Clone, new)]
pub struct CommandRegistry {
    #[new(value = "Arc::new(Mutex::new(IndexMap::default()))")]
    registry: Arc<Mutex<IndexMap<String, Arc<dyn Command>>>>,
}

impl CommandRegistry {
    crate fn empty() -> CommandRegistry {
        CommandRegistry {
            registry: Arc::new(Mutex::new(IndexMap::default())),
        }
    }

    fn get_config(&self, name: &str) -> Option<CommandConfig> {
        let registry = self.registry.lock().unwrap();

        registry.get(name).map(|c| c.config())
    }

    fn get_command(&self, name: &str) -> Option<Arc<dyn Command>> {
        let registry = self.registry.lock().unwrap();

        registry.get(name).map(|c| c.clone())
    }

    fn has(&self, name: &str) -> bool {
        let registry = self.registry.lock().unwrap();

        registry.contains_key(name)
    }

    fn insert(&mut self, name: impl Into<String>, command: Arc<dyn Command>) {
        let mut registry = self.registry.lock().unwrap();
        registry.insert(name.into(), command);
    }

    crate fn names(&self) -> Vec<String> {
        let mut registry = self.registry.lock().unwrap();
        registry.keys().cloned().collect()
    }
}

#[derive(Clone)]
pub struct Context {
    registry: CommandRegistry,
    sinks: IndexMap<String, Arc<dyn Sink>>,
    crate source_map: SourceMap,
    crate host: Arc<Mutex<dyn Host + Send>>,
    crate env: Arc<Mutex<Environment>>,
}

impl Context {
    crate fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    crate fn basic() -> Result<Context, Box<dyn Error>> {
        Ok(Context {
            registry: CommandRegistry::new(),
            sinks: indexmap::IndexMap::new(),
            source_map: SourceMap::new(),
            host: Arc::new(Mutex::new(crate::env::host::BasicHost)),
            env: Arc::new(Mutex::new(Environment::basic()?)),
        })
    }

    pub fn add_commands(&mut self, commands: Vec<Arc<dyn Command>>) {
        for command in commands {
            self.registry.insert(command.name().to_string(), command);
        }
    }

    pub fn add_sinks(&mut self, sinks: Vec<Arc<dyn Sink>>) {
        for sink in sinks {
            self.sinks.insert(sink.name().to_string(), sink);
        }
    }

    pub fn add_span_source(&mut self, uuid: Uuid, span_source: SpanSource) {
        self.source_map.insert(uuid, span_source);
    }

    crate fn has_sink(&self, name: &str) -> bool {
        self.sinks.contains_key(name)
    }

    crate fn get_sink(&self, name: &str) -> Arc<dyn Sink> {
        self.sinks.get(name).unwrap().clone()
    }

    crate fn run_sink(
        &mut self,
        command: Arc<dyn Sink>,
        name_span: Option<Span>,
        args: registry::EvaluatedArgs,
        input: Vec<Spanned<Value>>,
    ) -> Result<(), ShellError> {
        let command_args = SinkCommandArgs {
            ctx: self.clone(),
            call_info: CallInfo {
                name_span,
                source_map: self.source_map.clone(),
                args,
            },
            input,
        };

        command.run(command_args)
    }

    pub fn clone_commands(&self) -> CommandRegistry {
        self.registry.clone()
    }

    crate fn has_command(&self, name: &str) -> bool {
        self.registry.has(name)
    }

    crate fn get_command(&self, name: &str) -> Arc<dyn Command> {
        self.registry.get_command(name).unwrap()
    }

    crate fn run_command(
        &mut self,
        command: Arc<dyn Command>,
        name_span: Option<Span>,
        source_map: SourceMap,
        args: hir::Call,
        source: Text,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = self.command_args(args, input, source, source_map, name_span);

        command.run(command_args, self.registry())
    }

    fn call_info(
        &self,
        args: hir::Call,
        source: Text,
        source_map: SourceMap,
        name_span: Option<Span>,
    ) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo {
            args,
            source,
            source_map,
            name_span,
        }
    }

    fn command_args(
        &self,
        args: hir::Call,
        input: InputStream,
        source: Text,
        source_map: SourceMap,
        name_span: Option<Span>,
    ) -> CommandArgs {
        CommandArgs {
            host: self.host.clone(),
            env: self.env.clone(),
            call_info: self.call_info(args, source, source_map, name_span),
            input,
        }
    }
}
