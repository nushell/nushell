pub mod add;
pub mod append;
pub mod command;
pub mod remove;
pub mod reset;
pub mod save;

pub use add::SubCommand as PathvarAdd;
pub use append::SubCommand as PathvarAppend;
pub use command::Command as Pathvar;
pub use remove::SubCommand as PathvarRemove;
pub use reset::SubCommand as PathvarReset;
pub use save::SubCommand as PathvarSave;

use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_source::{Tagged, TaggedItem};
use nu_test_support::NATIVE_PATH_ENV_VAR;

fn get_var(args: &CommandArgs) -> Result<Tagged<String>, ShellError> {
    Ok(args
        .get_flag("var")?
        .unwrap_or_else(|| String::from(NATIVE_PATH_ENV_VAR))
        .tagged_unknown())
}
