mod command;

use std::{borrow::Cow, collections::HashMap};

use crate::{
    commands::{SimpleCommand, ViewCommand},
    views::View,
};

pub use command::Command;

#[derive(Default, Clone)]
pub struct CommandRegistry {
    commands: HashMap<Cow<'static, str>, Command>,
    aliases: HashMap<Cow<'static, str>, Cow<'static, str>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, command: Command) {
        self.commands
            .insert(Cow::Owned(command.name().to_owned()), command);
    }

    pub fn register_command_view<C>(&mut self, command: C, is_light: bool)
    where
        C: ViewCommand + Clone + 'static,
        C::View: View,
    {
        self.commands.insert(
            Cow::Owned(command.name().to_owned()),
            Command::view(command, is_light),
        );
    }

    pub fn register_command_reactive<C>(&mut self, command: C)
    where
        C: SimpleCommand + Clone + 'static,
    {
        self.commands.insert(
            Cow::Owned(command.name().to_owned()),
            Command::reactive(command),
        );
    }

    pub fn create_aliase(&mut self, aliase: &str, command: &str) {
        self.aliases.insert(
            Cow::Owned(aliase.to_owned()),
            Cow::Owned(command.to_owned()),
        );
    }

    pub fn find(&self, args: &str) -> Option<std::io::Result<Command>> {
        let cmd = args.split_once(' ').map_or(args, |(cmd, _)| cmd);
        let args = &args[cmd.len()..];

        let mut command = self.find_command(cmd)?;
        if let Err(err) = command.parse(args) {
            return Some(Err(err));
        }

        Some(Ok(command))
    }

    pub fn get_commands(&self) -> impl Iterator<Item = &Command> {
        self.commands.values()
    }

    pub fn get_aliases(&self) -> impl Iterator<Item = (&str, &str)> {
        self.aliases
            .iter()
            .map(|(key, value)| (key.as_ref(), value.as_ref()))
    }

    fn find_command(&self, cmd: &str) -> Option<Command> {
        match self.commands.get(cmd).cloned() {
            None => self
                .aliases
                .get(cmd)
                .and_then(|cmd| self.commands.get(cmd).cloned()),
            cmd => cmd,
        }
    }
}
