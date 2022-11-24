use std::collections::HashMap;

use crate::{
    commands::{HelpCmd, HelpManual, NuCmd, QuitCmd, SimpleCommand, TryCmd, ViewCommand},
    views::View,
    TableConfig,
};

#[derive(Clone)]
pub enum Command {
    Reactive(Box<dyn SCommand>),
    View(Box<dyn VCommand>),
}

pub struct CommandList {
    commands: HashMap<&'static str, Command>,
}

impl CommandList {
    pub fn new(table_cfg: &TableConfig) -> Self {
        macro_rules! cmd_view {
            ($object:expr) => {{
                let object = $object;

                let name = object.name();
                let cmd = Command::View(Box::new(ViewCmd(object)) as Box<dyn VCommand>);

                (name, cmd)
            }};
        }

        macro_rules! cmd_react {
            ($object:expr) => {{
                let object = $object;

                let name = object.name();
                let cmd = Command::Reactive(Box::new($object) as Box<dyn SCommand>);

                (name, cmd)
            }};
        }

        let mut cmd_list = vec![
            cmd_view!(NuCmd::new(table_cfg.clone())),
            cmd_view!(TryCmd::new(table_cfg.clone())),
            cmd_react!(QuitCmd::default()),
        ];

        let help_manuals = create_help_manuals(&cmd_list);
        let help_cmd = cmd_view!(HelpCmd::new(help_manuals, table_cfg.clone()));

        cmd_list.push(help_cmd);

        Self {
            commands: HashMap::from_iter(cmd_list),
        }
    }

    pub fn find(&self, args: &str) -> Option<Command> {
        let cmd = args.split_once(' ').map_or(args, |(cmd, _)| cmd);
        self.commands.get(cmd).cloned()
    }
}

fn create_help_manuals(cmd_list: &[(&str, Command)]) -> Vec<HelpManual> {
    let mut help_manuals: Vec<_> = cmd_list
        .iter()
        .map(|(_, cmd)| cmd)
        .map(create_help_manual)
        .collect();
    help_manuals.push(HelpCmd::default().help().unwrap());
    help_manuals
}

fn create_help_manual(cmd: &Command) -> HelpManual {
    let name = match cmd {
        Command::Reactive(c) => c.name(),
        Command::View(c) => c.name(),
    };

    let manual = match cmd {
        Command::Reactive(c) => c.help(),
        Command::View(c) => c.help(),
    };

    match manual {
        Some(manual) => manual,
        None => HelpManual {
            name,
            description: "",
            arguments: Vec::new(),
            examples: Vec::new(),
        },
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
