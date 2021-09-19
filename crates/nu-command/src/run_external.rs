use std::env;
use std::process::Command as CommandSys;

use nu_protocol::{
    ast::{Call, Expression},
    engine::{Command, EvaluationContext},
    ShellError, Signature, SyntaxShape, Value,
};

use nu_engine::eval_expression;

pub struct External;

impl Command for External {
    fn name(&self) -> &str {
        "run_external"
    }

    fn usage(&self) -> &str {
        "Runs external command"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("run_external").rest("rest", SyntaxShape::Any, "external command to run")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<Value, ShellError> {
        let command = ExternalCommand::try_new(call, context)?;
        command.run_with_input(input)
    }
}

pub struct ExternalCommand<'call, 'contex> {
    pub name: &'call Expression,
    pub args: &'call [Expression],
    pub context: &'contex EvaluationContext,
}

impl<'call, 'contex> ExternalCommand<'call, 'contex> {
    pub fn try_new(
        call: &'call Call,
        context: &'contex EvaluationContext,
    ) -> Result<Self, ShellError> {
        if call.positional.len() == 0 {
            return Err(ShellError::ExternalNotSupported(call.head));
        }

        Ok(Self {
            name: &call.positional[0],
            args: &call.positional[1..],
            context,
        })
    }

    pub fn get_name(&self) -> Result<String, ShellError> {
        let value = eval_expression(self.context, self.name)?;
        value.as_string()
    }

    pub fn get_args(&self) -> Vec<String> {
        self.args
            .iter()
            .filter_map(|expr| eval_expression(self.context, expr).ok())
            .filter_map(|value| value.as_string().ok())
            .collect()
    }

    pub fn run_with_input(&self, _input: Value) -> Result<Value, ShellError> {
        let mut process = self.create_command();

        // TODO. We don't have a way to know the current directory
        // This should be information from the EvaluationContex or EngineState
        let path = env::current_dir().unwrap();
        process.current_dir(path);

        let envs = self.context.stack.get_env_vars();
        process.envs(envs);

        match process.spawn() {
            Err(err) => Err(ShellError::ExternalCommand(
                format!("{}", err),
                self.name.span,
            )),
            Ok(mut child) => match child.wait() {
                Err(err) => Err(ShellError::ExternalCommand(
                    format!("{}", err),
                    self.name.span,
                )),
                Ok(_) => Ok(Value::nothing()),
            },
        }
    }

    fn create_command(&self) -> CommandSys {
        // in all the other cases shell out
        if cfg!(windows) {
            //TODO. This should be modifiable from the config file.
            // We could give the option to call from powershell
            // for minimal builds cwd is unused
            let mut process = CommandSys::new("cmd");
            process.arg("/c");
            process.arg(&self.get_name().unwrap());
            for arg in self.get_args() {
                // Clean the args before we use them:
                // https://stackoverflow.com/questions/1200235/how-to-pass-a-quoted-pipe-character-to-cmd-exe
                // cmd.exe needs to have a caret to escape a pipe
                let arg = arg.replace("|", "^|");
                process.arg(&arg);
            }
            process
        } else {
            let cmd_with_args = vec![self.get_name().unwrap(), self.get_args().join(" ")].join(" ");
            let mut process = CommandSys::new("sh");
            process.arg("-c").arg(cmd_with_args);
            process
        }
    }
}
