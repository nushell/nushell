mod ansi;
mod base;
mod char_;
mod detect;
mod detect_columns;
mod detect_type;
mod encode_decode;
mod format;
mod guess_width;
mod parse;
mod split;
mod str_;

pub use ansi::{Ansi, AnsiLink, AnsiStrip};
pub use base::{
    DecodeBase32, DecodeBase32Hex, DecodeBase64, DecodeHex, EncodeBase32, EncodeBase32Hex,
    EncodeBase64, EncodeHex,
};
pub use char_::Char;
pub use detect::Detect;
pub use detect_columns::*;
pub use detect_type::*;
pub use encode_decode::*;
pub use format::*;
pub use parse::*;
pub use split::*;
pub use str_::*;

use nu_engine::CallExt;
use nu_protocol::{
    ShellError,
    engine::{Call, EngineState, Stack, StateWorkingSet},
};

// For handling the grapheme_cluster related flags on some commands.
// This ensures the error messages are consistent.
pub fn grapheme_flags(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<bool, ShellError> {
    let g_flag = call.has_flag(engine_state, stack, "grapheme-clusters")?;
    // Check for the other flags and produce errors if they exist.
    // Note that Nushell already prevents nonexistent flags from being used with commands,
    // so this function can be reused for both the --utf-8-bytes commands and the --code-points commands.
    if g_flag && call.has_flag(engine_state, stack, "utf-8-bytes")? {
        Err(ShellError::IncompatibleParametersSingle {
            msg: "Incompatible flags: --grapheme-clusters (-g) and --utf-8-bytes (-b)".to_string(),
            span: call.head,
        })?
    }
    if g_flag && call.has_flag(engine_state, stack, "code-points")? {
        Err(ShellError::IncompatibleParametersSingle {
            msg: "Incompatible flags: --grapheme-clusters (-g) and --code-points (-c)".to_string(),
            span: call.head,
        })?
    }
    if g_flag && call.has_flag(engine_state, stack, "chars")? {
        Err(ShellError::IncompatibleParametersSingle {
            msg: "Incompatible flags: --grapheme-clusters (-g) and --chars (-c)".to_string(),
            span: call.head,
        })?
    }
    // Grapheme cluster usage is decided by the non-default -g flag
    Ok(g_flag)
}

// Const version of grapheme_flags
pub fn grapheme_flags_const(
    working_set: &StateWorkingSet,
    call: &Call,
) -> Result<bool, ShellError> {
    let g_flag = call.has_flag_const(working_set, "grapheme-clusters")?;
    if g_flag && call.has_flag_const(working_set, "utf-8-bytes")? {
        Err(ShellError::IncompatibleParametersSingle {
            msg: "Incompatible flags: --grapheme-clusters (-g) and --utf-8-bytes (-b)".to_string(),
            span: call.head,
        })?
    }
    if g_flag && call.has_flag_const(working_set, "code-points")? {
        Err(ShellError::IncompatibleParametersSingle {
            msg: "Incompatible flags: --grapheme-clusters (-g) and --utf-8-bytes (-b)".to_string(),
            span: call.head,
        })?
    }
    Ok(g_flag)
}
