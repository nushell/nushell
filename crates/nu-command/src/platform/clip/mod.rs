mod clipboard;
mod command;
mod copy;
mod get_config;
mod paste;

use nu_engine::command_prelude::{ShellError, Span};

pub use command::ClipCommand;
pub use copy::ClipCopy;
pub use paste::ClipPaste;

fn ensure_native_clip_enabled(span: Span) -> Result<(), ShellError> {
    if nu_experimental::NATIVE_CLIP.get() {
        return Ok(());
    }

    Err(ShellError::GenericError {
        error: "native-clip experimental option is disabled".into(),
        msg: format!(
            "Enable {} with $env.{} = [native-clip]",
            nu_experimental::NATIVE_CLIP.identifier(),
            nu_experimental::ENV
        ),
        span: Some(span),
        help: None,
        inner: vec![],
    })
}
