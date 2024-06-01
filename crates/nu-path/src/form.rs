mod private {
    pub trait Sealed {}
}

pub trait PathForm: private::Sealed {}

pub trait MaybeRelative: PathForm {}

pub trait MaybeAbsolute: PathForm {}

pub trait IsAbsolute: PathForm {}

pub trait PathCast<Form: PathForm>: PathForm {}

impl<Form: PathForm> PathCast<Form> for Form {}

pub trait PathJoin: PathForm {
    type Output: PathForm;
}

pub trait PathPush: PathForm {}

pub trait PathSet: PathForm {}

/// A path whose form is unknown. It could be a relative, absolute, or canonical path.
///
///
pub struct Any;

impl private::Sealed for Any {}

impl PathForm for Any {}
impl MaybeRelative for Any {}
impl MaybeAbsolute for Any {}
impl PathJoin for Any {
    type Output = Self;
}
impl PathPush for Any {}
impl PathSet for Any {}

/// A strictly relative path.
///
/// The path is not guaranteed to be normalized. It may contain unresolved symlinks,
/// trailing slashes, dot components (`..` or `.`), and duplicate path separators.
pub struct Relative;

impl private::Sealed for Relative {}
impl PathForm for Relative {}
impl PathCast<Any> for Relative {}
impl MaybeRelative for Relative {}
impl PathJoin for Relative {
    type Output = Any;
}
impl PathSet for Relative {}

/// An absolute path.
///
/// The path is not guaranteed to be normalized. It may contain unresolved symlinks,
/// trailing slashes, dot components (`..` or `.`), and duplicate path separators.
pub struct Absolute;

impl private::Sealed for Absolute {}
impl PathForm for Absolute {}
impl PathCast<Any> for Absolute {}
impl MaybeAbsolute for Absolute {}
impl IsAbsolute for Absolute {}
impl PathJoin for Absolute {
    type Output = Self;
}
impl PathPush for Absolute {}
impl PathSet for Absolute {}

// A canonical path.
//
// An absolute path with all intermediate components normalized and symbolic links resolved.
pub struct Canonical;

impl private::Sealed for Canonical {}
impl PathForm for Canonical {}
impl PathCast<Any> for Canonical {}
impl PathCast<Absolute> for Canonical {}
impl MaybeAbsolute for Canonical {}
impl IsAbsolute for Canonical {}
impl PathJoin for Canonical {
    type Output = Absolute;
}
