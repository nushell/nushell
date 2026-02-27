use crate::*;

/// Enable `clip copy` and `clip paste` commands that use native API.
///
/// These commands do not use the OSC52 code to tell the terminal to copy data but rather implement
/// them directly in Rust.
pub static NATIVE_CLIP: ExperimentalOption = ExperimentalOption::new(&NativeClip);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct NativeClip;

impl ExperimentalOptionMarker for NativeClip {
    const IDENTIFIER: &'static str = "native-clip";
    const DESCRIPTION: &'static str = "Adds clipboard commands that implement copy and pasting via native APIs instead of OSC52 codes.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 110, 1);
    const ISSUE: u32 = 17665;
}
