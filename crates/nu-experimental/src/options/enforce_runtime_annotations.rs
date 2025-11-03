use crate::*;

/// Enable runtime type annotation feature to ensure that type annotations
/// are checked against the type that binds to them, returning conversion errors
/// if the types are incompatible.
pub static ENFORCE_RUNTIME_ANNOTATIONS: ExperimentalOption =
    ExperimentalOption::new(&EnforceRuntimeAnnotations);

struct EnforceRuntimeAnnotations;

impl ExperimentalOptionMarker for EnforceRuntimeAnnotations {
    const IDENTIFIER: &'static str = "enforce-runtime-annotations";
    const DESCRIPTION: &'static str = "\
        Enforce type checking of let assignments at runtime such that \
        invalid type conversion errors propagate the same way they would for predefined values.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 107, 1);
    const ISSUE: u32 = 16832;
}
