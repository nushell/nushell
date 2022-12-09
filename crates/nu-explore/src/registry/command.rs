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
