use crate::commands::{Command, UnevaluatedCallInfo};
use crate::parser::hir;
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
    registry: Arc<Mutex<IndexMap<String, Arc<Command>>>>,
}

impl CommandRegistry {
    pub(crate) fn empty() -> CommandRegistry {
        CommandRegistry {
            registry: Arc::new(Mutex::new(IndexMap::default())),
        }
    }

    pub(crate) fn get_command(&self, name: &str) -> Option<Arc<Command>> {
        let registry = self.registry.lock().unwrap();

        registry.get(name).map(|c| c.clone())
    }

    pub(crate) fn has(&self, name: &str) -> bool {
        let registry = self.registry.lock().unwrap();

        registry.contains_key(name)
    }

    fn insert(&mut self, name: impl Into<String>, command: Arc<Command>) {
        let mut registry = self.registry.lock().unwrap();
        registry.insert(name.into(), command);
    }

    pub(crate) fn names(&self) -> Vec<String> {
        let registry = self.registry.lock().unwrap();
        registry.keys().cloned().collect()
    }
}

#[derive(Clone)]
pub struct Context {
    registry: CommandRegistry,
    pub(crate) source_map: SourceMap,
    host: Arc<Mutex<dyn Host + Send>>,
    pub(crate) shell_manager: ShellManager,
}

impl Context {
    pub(crate) fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    pub(crate) fn basic() -> Result<Context, Box<dyn Error>> {
        let registry = CommandRegistry::new();
        Ok(Context {
            registry: registry.clone(),
            source_map: SourceMap::new(),
            host: Arc::new(Mutex::new(crate::env::host::BasicHost)),
            shell_manager: ShellManager::basic(registry)?,
        })
    }

    pub(crate) fn with_host(&mut self, block: impl FnOnce(&mut dyn Host)) {
        let mut host = self.host.lock().unwrap();

        block(&mut *host)
    }

    pub fn add_commands(&mut self, commands: Vec<Arc<Command>>) {
        for command in commands {
            self.registry.insert(command.name().to_string(), command);
        }
    }

    pub fn add_span_source(&mut self, uuid: Uuid, span_source: SpanSource) {
        self.source_map.insert(uuid, span_source);
    }

    pub(crate) fn has_command(&self, name: &str) -> bool {
        self.registry.has(name)
    }

    pub(crate) fn get_command(&self, name: &str) -> Arc<Command> {
        self.registry.get_command(name).unwrap()
    }

    pub(crate) fn run_command<'a>(
        &mut self,
        command: Arc<Command>,
        name_span: Span,
        source_map: SourceMap,
        args: hir::Call,
        source: &Text,
        input: InputStream,
    ) -> OutputStream {
        let command_args = self.command_args(args, input, source, source_map, name_span);
        command.run(command_args, self.registry())
    }

    fn call_info(
        &self,
        args: hir::Call,
        source: &Text,
        source_map: SourceMap,
        name_span: Span,
    ) -> UnevaluatedCallInfo {
        UnevaluatedCallInfo {
            args,
            source: source.clone(),
            source_map,
            name_span,
        }
    }

    fn command_args(
        &self,
        args: hir::Call,
        input: InputStream,
        source: &Text,
        source_map: SourceMap,
        name_span: Span,
    ) -> CommandArgs {
        CommandArgs {
            host: self.host.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info(args, source, source_map, name_span),
            input,
        }
    }
}
