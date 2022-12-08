use std::{borrow::Cow, collections::HashMap};

use crate::{
    commands::{HelpManual, SimpleCommand, ViewCommand},
    views::View,
};

#[derive(Clone)]
pub enum Command {
    Reactive(Box<dyn SCommand>),
    View {
        cmd: Box<dyn VCommand>,
        is_light: bool,
    },
}

impl Command {
    pub fn view<C>(command: C, is_light: bool) -> Self
    where
        C: ViewCommand + Clone + 'static,
        C::View: View,
    {
        let cmd = Box::new(ViewCmd(command)) as Box<dyn VCommand>;

        Self::View { cmd, is_light }
    }

    pub fn reactive<C>(command: C) -> Self
    where
        C: SimpleCommand + Clone + 'static,
    {
        let cmd = Box::new(command) as Box<dyn SCommand>;

        Self::Reactive(cmd)
    }
}

impl Command {
    pub fn name(&self) -> &str {
        match self {
            Command::Reactive(cmd) => cmd.name(),
            Command::View { cmd, .. } => cmd.name(),
        }
    }

    pub fn parse(&mut self, args: &str) -> std::io::Result<()> {
        match self {
            Command::Reactive(cmd) => cmd.parse(args),
            Command::View { cmd, .. } => cmd.parse(args),
        }
    }
}

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

// type helper to deal with `Box`es
#[derive(Clone)]
struct ViewCmd<C>(C);

impl<C> ViewCommand for ViewCmd<C>
where
    C: ViewCommand,
    C::View: View + 'static,
{
    type View = Box<dyn View>;

    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn usage(&self) -> &'static str {
        self.0.usage()
    }

    fn help(&self) -> Option<HelpManual> {
        self.0.help()
    }

    fn display_config_option(&mut self, group: String, key: String, value: String) -> bool {
        self.0.display_config_option(group, key, value)
    }

    fn parse(&mut self, args: &str) -> std::io::Result<()> {
        self.0.parse(args)
    }

    fn spawn(
        &mut self,
        engine_state: &nu_protocol::engine::EngineState,
        stack: &mut nu_protocol::engine::Stack,
        value: Option<nu_protocol::Value>,
    ) -> std::io::Result<Self::View> {
        let view = self.0.spawn(engine_state, stack, value)?;
        Ok(Box::new(view) as Box<dyn View>)
    }
}

pub trait SCommand: SimpleCommand + SCommandClone {}

impl<T> SCommand for T where T: 'static + SimpleCommand + Clone {}

pub trait SCommandClone {
    fn clone_box(&self) -> Box<dyn SCommand>;
}

impl<T> SCommandClone for T
where
    T: 'static + SCommand + Clone,
{
    fn clone_box(&self) -> Box<dyn SCommand> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn SCommand> {
    fn clone(&self) -> Box<dyn SCommand> {
        self.clone_box()
    }
}

pub trait VCommand: ViewCommand<View = Box<dyn View>> + VCommandClone {}

impl<T> VCommand for T where T: 'static + ViewCommand<View = Box<dyn View>> + Clone {}

pub trait VCommandClone {
    fn clone_box(&self) -> Box<dyn VCommand>;
}

impl<T> VCommandClone for T
where
    T: 'static + VCommand + Clone,
{
    fn clone_box(&self) -> Box<dyn VCommand> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn VCommand> {
    fn clone(&self) -> Box<dyn VCommand> {
        self.clone_box()
    }
}
