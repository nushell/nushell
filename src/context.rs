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
    crate fn empty() -> CommandRegistry {
        CommandRegistry {
            registry: Arc::new(Mutex::new(IndexMap::default())),
        }
    }

    crate fn get_command(&self, name: &str) -> Option<Arc<Command>> {
        let registry = self.registry.lock().unwrap();

        registry.get(name).map(|c| c.clone())
    }

    fn has(&self, name: &str) -> bool {
        let registry = self.registry.lock().unwrap();

        registry.contains_key(name)
    }

    fn insert(&mut self, name: impl Into<String>, command: Arc<Command>) {
        let mut registry = self.registry.lock().unwrap();
        registry.insert(name.into(), command);
    }

    crate fn names(&self) -> Vec<String> {
        let registry = self.registry.lock().unwrap();
        registry.keys().cloned().collect()
    }
}

#[derive(Clone)]
pub struct Context {
    registry: CommandRegistry,
    crate source_map: SourceMap,
    crate host: Arc<Mutex<dyn Host + Send>>,
    crate shell_manager: ShellManager,
}

impl Context {
    crate fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    crate fn basic() -> Result<Context, Box<dyn Error>> {
        Ok(Context {
            registry: CommandRegistry::new(),
            source_map: SourceMap::new(),
            host: Arc::new(Mutex::new(crate::env::host::BasicHost)),
            shell_manager: ShellManager::basic()?,
        })
    }

    pub fn add_commands(&mut self, commands: Vec<Arc<Command>>) {
        for command in commands {
            self.registry.insert(command.name().to_string(), command);
        }
    }

    pub fn add_span_source(&mut self, uuid: Uuid, span_source: SpanSource) {
        self.source_map.insert(uuid, span_source);
    }

    pub fn clone_commands(&self) -> CommandRegistry {
        self.registry.clone()
    }

    crate fn has_command(&self, name: &str) -> bool {
        self.registry.has(name)
    }

    crate fn get_command(&self, name: &str) -> Arc<Command> {
        self.registry.get_command(name).unwrap()
    }

    crate async fn run_command(
        &mut self,
        command: Arc<Command>,
        name_span: Span,
        source_map: SourceMap,
        args: hir::Call,
        source: Text,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = self.command_args(args, input, source, source_map, name_span);
        /*
        let command_args = CommandArgs {
            host: self.host.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: CallInfo {
                name_span,
                source_map,
                args,
            },
            input,
        };
        */

        command.run(command_args, self.registry()).await
    }

    fn call_info(
        &self,
        args: hir::Call,
        source: Text,
        source_map: SourceMap,
        name_span: Span,
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
