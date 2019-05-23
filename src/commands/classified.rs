use crate::prelude::*;
use std::sync::Arc;
use subprocess::Exec;

crate enum ClassifiedCommand {
    Internal(InternalCommand),
    External(ExternalCommand),
}

impl ClassifiedCommand {
    crate fn run(
        self,
        input: VecDeque<Value>,
        context: &mut Context,
    ) -> Result<VecDeque<Value>, ShellError> {
        match self {
            ClassifiedCommand::Internal(internal) => {
                let result = context.run_command(internal.command, internal.args, input)?;

                let mut next = VecDeque::new();

                for v in result {
                    match v {
                        ReturnValue::Action(action) => match action {
                            CommandAction::ChangeCwd(cwd) => context.env.cwd = cwd,
                        },

                        ReturnValue::Value(v) => next.push_back(v),
                    }
                }

                Ok(next)
            }

            ClassifiedCommand::External(external) => {
                Exec::shell(&external.name)
                    .args(&external.args)
                    .cwd(context.env.cwd())
                    .join()
                    .unwrap();
                Ok(VecDeque::new())
            }
        }
    }
}

crate struct InternalCommand {
    crate command: Arc<dyn Command>,
    crate args: Vec<Value>,
}

crate struct ExternalCommand {
    crate name: String,
    crate args: Vec<String>,
}
