use std::ops::Deref;

use nix::libc::mode_t;
use nu_engine::command_prelude::*;
use nu_system::get_umask;

/// Wraps umask::Mode, providing conversions to and from mode_t regardless of its
/// size.
//
// The nix::sys::stat::Mode struct (used for setting the umask) only provides
// conversions to and from nix::libc::mode_t, the size of which is
// platform-dependant. However, umask::Mode (used for parsing and formatting)
// only provides conversions for u32, which causes problems on platforms where
// mode_t is u16.
struct Mode(umask::Mode);

impl Deref for Mode {
    type Target = umask::Mode;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Mode> for mode_t {
    #[allow(clippy::unnecessary_cast)]
    fn from(mode: Mode) -> Self {
        // This is "u32 as u16" or "u32 as u32", depending on platform.
        u32::from(mode.0) as mode_t
    }
}

impl From<mode_t> for Mode {
    #[allow(clippy::unnecessary_cast)]
    fn from(value: mode_t) -> Self {
        // This is "u16 as u32" or "u32 as u32", depending on platform.
        Self((value as u32).into())
    }
}

#[derive(Clone)]
pub struct UMask;

impl Command for UMask {
    fn name(&self) -> &str {
        "umask"
    }

    fn description(&self) -> &str {
        "Get or set default file creation permissions."
    }

    fn extra_description(&self) -> &str {
        "When setting a new mask, the previous mask will be returned."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["permissions", "create", "file", "directory", "folder"]
    }

    fn signature(&self) -> Signature {
        Signature::build("umask")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .optional(
                "permissions",
                SyntaxShape::String,
                "The permissions to set on created files.",
            )
            .category(Category::Platform)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let maybe_perms_val = call.opt::<Spanned<String>>(engine_state, stack, 0)?;

        let prev_mask_bits = if let Some(perms_val) = maybe_perms_val {
            let perms = Mode(
                perms_val
                    .item
                    .parse()
                    .map_err(|err| ShellError::IncorrectValue {
                        msg: format!("Invalid mode: {0}.", err),
                        val_span: perms_val.span,
                        call_span: call.head,
                    })?,
            );

            // The `umask` syscall wants the bits to mask *out*, not *in*, so
            // the mask needs inverted before passing it in.
            let mask_bits = 0o777 ^ mode_t::from(perms);

            let mask =
                nix::sys::stat::Mode::from_bits(mask_bits).ok_or(ShellError::IncorrectValue {
                    // Can't happen? The umask crate shouldn't ever set bits
                    // which the nix crate doesn't recognize.
                    msg: "Invalid mask; unrecognized permission bits.".into(),
                    val_span: perms_val.span,
                    call_span: call.head,
                })?;

            nix::sys::stat::umask(mask).bits()
        } else {
            get_umask() as mode_t
        };

        // The `umask` syscall wants the bits to mask *out*, not *in*, so
        // the old mask needs uninverted before outputting it.
        let prev_perms = Mode::from(0o777 ^ prev_mask_bits);

        Ok(Value::string(prev_perms.to_string(), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Print current default file creation permissions.",
                example: "umask",
                result: None,
            },
            Example {
                description: "Make new files read-only to group and inaccessible to others.",
                example: "umask rwxr-x---",
                result: None,
            },
        ]
    }
}
