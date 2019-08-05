use crate::commands::command::{CallInfo, Sink, SinkCommandArgs};
use crate::parser::registry::{Args, CommandConfig, CommandRegistry};
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use indexmap::IndexMap;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpanSource {
    Url(String),
    File(String),
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

#[derive(Clone)]
pub struct Context {
    commands: IndexMap<String, Arc<dyn Command>>,
    sinks: IndexMap<String, Arc<dyn Sink>>,
    crate source_map: SourceMap,
    crate host: Arc<Mutex<dyn Host + Send>>,
    crate env: Arc<Mutex<Environment>>,
}

impl Context {
    crate fn basic() -> Result<Context, Box<dyn Error>> {
        Ok(Context {
            commands: indexmap::IndexMap::new(),
            sinks: indexmap::IndexMap::new(),
            source_map: SourceMap::new(),
            host: Arc::new(Mutex::new(crate::env::host::BasicHost)),
            env: Arc::new(Mutex::new(Environment::basic()?)),
        })
    }

    pub fn add_commands(&mut self, commands: Vec<Arc<dyn Command>>) {
        for command in commands {
            self.commands.insert(command.name().to_string(), command);
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
        name_span: Span,
        args: Args,
        input: Vec<Tagged<Value>>,
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

    pub fn clone_commands(&self) -> indexmap::IndexMap<String, Arc<dyn Command>> {
        self.commands.clone()
    }

    crate fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    crate fn get_command(&self, name: &str) -> Arc<dyn Command> {
        self.commands.get(name).unwrap().clone()
    }

    crate fn run_command(
        &mut self,
        command: Arc<dyn Command>,
        name_span: Span,
        source_map: SourceMap,
        args: Args,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = CommandArgs {
            host: self.host.clone(),
            env: self.env.clone(),
            call_info: CallInfo {
                name_span,
                source_map,
                args,
            },
            input,
        };

        command.run(command_args)
    }
}

impl CommandRegistry for Context {
    fn get(&self, name: &str) -> Option<CommandConfig> {
        self.commands.get(name).map(|c| c.config())
    }
}
