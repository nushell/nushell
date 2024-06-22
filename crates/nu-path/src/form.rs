use std::ffi::OsStr;

mod private {
    use std::ffi::OsStr;

    // This trait should not be extended by external crates in order to uphold safety guarantees.
    // As such, this trait is put inside a private module to prevent external impls.
    // This ensures that all possible [`PathForm`]s can only be defined here and will:
    // - be zero sized (enforced anyways by the `repr(transparent)` on `Path`)
    // - have a no-op [`Drop`] implementation
    pub trait Sealed: 'static {
        fn invariants_satisfied<P: AsRef<OsStr> + ?Sized>(path: &P) -> bool;
    }
}

/// A marker trait for the different kinds of path forms.
/// Each form has its own invariants that are guaranteed be upheld.
/// The list of path forms are:
/// - [`Any`]: a path with no invariants. It may be a relative or an absolute path.
/// - [`Relative`]: a strictly relative path.
/// - [`Absolute`]: a strictly absolute path.
/// - [`Canonical`]: a path that must be in canonicalized form.
pub trait PathForm: private::Sealed {}
impl PathForm for Any {}
impl PathForm for Relative {}
impl PathForm for Absolute {}
impl PathForm for Canonical {}

/// A path whose form is unknown. It could be a relative, absolute, or canonical path.
///
/// The path is not guaranteed to be normalized. It may contain unresolved symlinks,
/// trailing slashes, dot components (`..` or `.`), and repeated path separators.
pub struct Any;

impl private::Sealed for Any {
    fn invariants_satisfied<P: AsRef<OsStr> + ?Sized>(_: &P) -> bool {
        true
    }
}

/// A strictly relative path.
///
/// The path is not guaranteed to be normalized. It may contain unresolved symlinks,
/// trailing slashes, dot components (`..` or `.`), and repeated path separators.
pub struct Relative;

impl private::Sealed for Relative {
    fn invariants_satisfied<P: AsRef<OsStr> + ?Sized>(path: &P) -> bool {
        std::path::Path::new(path).is_relative()
    }
}

/// An absolute path.
///
/// The path is not guaranteed to be normalized. It may contain unresolved symlinks,
/// trailing slashes, dot components (`..` or `.`), and repeated path separators.
pub struct Absolute;

impl private::Sealed for Absolute {
    fn invariants_satisfied<P: AsRef<OsStr> + ?Sized>(path: &P) -> bool {
        std::path::Path::new(path).is_absolute()
    }
}

// A canonical path.
//
// An absolute path with all intermediate components normalized and symbolic links resolved.
pub struct Canonical;

impl private::Sealed for Canonical {
    fn invariants_satisfied<P: AsRef<OsStr> + ?Sized>(_: &P) -> bool {
        true
    }
}

/// A marker trait for [`PathForm`]s that may be relative paths.
/// This includes only the [`Any`] and [`Relative`] path forms.
///
/// [`push`](crate::PathBuf::push) and [`join`](crate::Path::join)
/// operations only support [`MaybeRelative`] path forms as input.
pub trait MaybeRelative: PathForm {}
impl MaybeRelative for Any {}
impl MaybeRelative for Relative {}

/// A marker trait for [`PathForm`]s that may be absolute paths.
/// This includes the [`Any`], [`Absolute`], and [`Canonical`] path forms.
pub trait MaybeAbsolute: PathForm {}
impl MaybeAbsolute for Any {}
impl MaybeAbsolute for Absolute {}
impl MaybeAbsolute for Canonical {}

/// A marker trait for [`PathForm`]s that are absolute paths.
/// This includes only the [`Absolute`] and [`Canonical`] path forms.
///
/// Only [`PathForm`]s that implement this trait can be easily converted to [`std::path::Path`]
/// or [`std::path::PathBuf`]. This is to encourage/force other Nushell crates to account for
/// the emulated current working directory, instead of using the [`std::env::current_dir`].
pub trait IsAbsolute: PathForm {}
impl IsAbsolute for Absolute {}
impl IsAbsolute for Canonical {}

/// A marker trait that signifies one [`PathForm`] can be used as or trivially converted to
/// another [`PathForm`].
///
/// The list of possible conversions are:
/// - [`Relative`], [`Absolute`], or [`Canonical`] into [`Any`].
/// - [`Canonical`] into [`Absolute`].
/// - Any form into itself.
pub trait PathCast<Form: PathForm>: PathForm {}
impl<Form: PathForm> PathCast<Form> for Form {}
impl PathCast<Any> for Relative {}
impl PathCast<Any> for Absolute {}
impl PathCast<Any> for Canonical {}
impl PathCast<Absolute> for Canonical {}

/// A trait used to specify the output [`PathForm`] of a path join operation.
///
/// The output path forms based on the left hand side path form are as follows:
///
/// | Left hand side | Output form  |
/// | --------------:|:------------ |
/// | [`Any`]        | [`Any`]      |
/// | [`Relative`]   | [`Any`]      |
/// | [`Absolute`]   | [`Absolute`] |
/// | [`Canonical`]  | [`Absolute`] |
pub trait PathJoin: PathForm {
    type Output: PathForm;
}
impl PathJoin for Any {
    type Output = Self;
}
impl PathJoin for Relative {
    type Output = Any;
}
impl PathJoin for Absolute {
    type Output = Self;
}
impl PathJoin for Canonical {
    type Output = Absolute;
}

/// A marker trait for [`PathForm`]s that support setting the file name or extension.
///
/// This includes the [`Any`], [`Relative`], and [`Absolute`] path forms.
/// [`Canonical`] paths do not support this, since appending file names and extensions that contain
/// path separators can cause the path to no longer be canonical.
pub trait PathSet: PathForm {}
impl PathSet for Any {}
impl PathSet for Relative {}
impl PathSet for Absolute {}

/// A marker trait for [`PathForm`]s that support pushing [`MaybeRelative`] paths.
///
/// This includes only [`Any`] and [`Absolute`] path forms.
/// Pushing onto a [`Relative`] path could cause it to become [`Absolute`],
/// which is why they do not support pushing.
/// In the future, a `push_rel` and/or a `try_push` method could be added as an alternative.
/// Similarly, [`Canonical`] paths may become uncanonical if a non-canonical path is pushed onto it.
pub trait PathPush: PathSet {}
impl PathPush for Any {}
impl PathPush for Absolute {}
