use crate::commands::command::Sink;
use crate::commands::command::SinkCommandArgs;
use crate::parser::lexer::Span;
use crate::parser::Args;
use crate::prelude::*;

use indexmap::IndexMap;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    commands: IndexMap<String, Arc<dyn Command>>,
    sinks: IndexMap<String, Arc<dyn Sink>>,
    crate host: Arc<Mutex<dyn Host + Send>>,
    crate env: Arc<Mutex<VecDeque<Environment>>>,
}

impl Context {
    crate fn basic() -> Result<Context, Box<dyn Error>> {
        let mut env = VecDeque::new();
        env.push_back(Environment::basic()?);
        Ok(Context {
            commands: indexmap::IndexMap::new(),
            sinks: indexmap::IndexMap::new(),
            host: Arc::new(Mutex::new(crate::env::host::BasicHost)),
            env: Arc::new(Mutex::new(env)),
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
        args: Args,
        input: Vec<Value>,
    ) -> Result<(), ShellError> {
        let command_args = SinkCommandArgs {
            ctx: self.clone(),
            name_span,
            positional: args.positional,
            named: args.named,
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
        name_span: Option<Span>,
        args: Args,
        input: InputStream,
    ) -> Result<OutputStream, ShellError> {
        let command_args = CommandArgs {
            host: self.host.clone(),
            env: self.env.clone(),
            name_span,
            positional: args.positional,
            named: args.named,
            input,
        };

        command.run(command_args)
    }
}
