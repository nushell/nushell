use crate::prelude::*;
use std::sync::Arc;
use subprocess::Exec;

crate enum ClassifiedCommand {
    Internal(InternalCommand),
    External(ExternalCommand),
}

impl ClassifiedCommand {
    crate async fn run(
        self,
        input: InputStream,
        context: &mut Context,
    ) -> Result<InputStream, ShellError> {
        match self {
            ClassifiedCommand::Internal(internal) => {
                let result = context.run_command(internal.command, internal.args, input)?;
                let env = context.env.clone();

                let stream = result.filter_map(move |v| match v {
                    ReturnValue::Action(action) => match action {
                        CommandAction::ChangeCwd(cwd) => {
                            env.lock().unwrap().cwd = cwd;
                            futures::future::ready(None)
                        }
                    },

                    ReturnValue::Value(v) => futures::future::ready(Some(v)),
                });

                Ok(stream.boxed() as InputStream)
            }

            ClassifiedCommand::External(external) => {
                Exec::shell(&external.name)
                    .args(&external.args)
                    .cwd(context.env.lock().unwrap().cwd())
                    .join()
                    .unwrap();

                Ok(VecDeque::new().boxed() as InputStream)
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
