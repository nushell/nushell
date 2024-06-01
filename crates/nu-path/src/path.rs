use crate::form::{
    Absolute, Any, Canonical, IsAbsolute, MaybeRelative, PathCast, PathForm, PathJoin, PathPush,
    PathSet, Relative,
};
use std::{
    borrow::{Borrow, Cow},
    cmp::Ordering,
    collections::TryReserveError,
    convert::Infallible,
    ffi::{OsStr, OsString},
    fmt, fs,
    hash::{Hash, Hasher},
    io,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::StripPrefixError,
    rc::Rc,
    str::FromStr,
    sync::Arc,
};

/// A wrapper around [`std::path::Path`] with extra invariants determined by its `Form`.
///
/// The possible path forms are [`Any`], [`Relative`], [`Absolute`], or [`Canonical`].
/// To learn more, view the documentation on [`PathForm`] or any of the individual forms.
///
/// There are also several type aliases available, corresponding to each [`PathForm`]:
/// - [`RelativePath`] (same as [`Path<Relative>`])
/// - [`AbsolutePath`] (same as [`Path<Absolute>`])
/// - [`CanonicalPath`] (same as [`Path<Canonical>`])
///
/// If the `Form` is not specified, then it defaults to [`Any`], so [`Path`] and [`Path<Any>`]
/// are one in the same.
///
/// To create a [`Path`] with [`Any`] form, use the [`Path::new`] method or use `as_ref` on
/// a type from the [`std::path`] module like [`std::path::Path`] or [`std::path::PathBuf`].
#[repr(transparent)]
pub struct Path<Form: PathForm = Any> {
    _form: PhantomData<Form>,
    inner: std::path::Path,
}

/// A path that is strictly relative.
///
/// I.e., this path is guaranteed to never be absolute.
///
/// [`RelativePath`]s can be created by using [`try_relative`](Path::try_relative)
/// on a [`Path`] or using [`strip_prefix`](Path::strip_prefix) on a [`Path`] of any form.
/// You can also use `RelativePath::try_from` or `try_into`.
/// This supports attempted conversions from [`Path`] as well as types in [`std::path`].
///
/// [`RelativePath`]s cannot be easily converted into a [`std::path::Path`] by design.
/// Other Nushell crates need to account for the emulated current working directory
/// before passing a path to functions in [`std`] or other third party crates.
/// You can use a [`RelativePath`] as input to `join` on an [`AbsolutePath`] or [`CanonicalPath`].
/// This will return an [`AbsolutePath`] which can be referenced as and easily converted to
/// a [`std::path::Path`]. If you really mean it, you can use
/// [`as_relative_std_path`](RelativePath::as_relative_std_path) to get the underlying
/// [`std::path::Path`] from a [`RelativePath`].
pub type RelativePath = Path<Relative>;

/// A path that is strictly absolute.
///
/// I.e., this path is guaranteed to never be relative.
///
/// [`AbsolutePath`]s can be created by using [`try_absolute`](Path::try_absolute) on a [`Path`].
/// You can also use `AbsolutePath::try_from` or `try_into`.
/// This supports attempted conversions from [`Path`] as well as types in [`std::path`].
///
/// References to [`CanonicalPath`]s can be converted to [`AbsolutePath`] references using
/// `as_ref`, [`cast`](Path::cast), or [`as_absolute`](CanonicalPath::as_absolute).
pub type AbsolutePath = Path<Absolute>;

/// An absolute, canonical path.
///
/// [`CanonicalPath`]s can only be created by using [`canonicalize`](Path::canonicalize) on
/// an [`AbsolutePath`].
///
/// References to [`CanonicalPath`]s can be converted to [`AbsolutePath`] references using
/// `as_ref`, [`cast`](Path::cast), or [`as_absolute`](CanonicalPath::as_absolute).
pub type CanonicalPath = Path<Canonical>;

impl<Form: PathForm> Path<Form> {
    /// Create a new path without checking that invariants are satisfied.
    #[inline]
    fn new_unchecked<P: AsRef<OsStr> + ?Sized>(path: &P) -> &Self {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let path = std::path::Path::new(path.as_ref());
        let ptr = std::ptr::from_ref(path) as *const Self;
        unsafe { &*ptr }
    }

    /// Returns the underlying [`OsStr`] slice.
    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        self.inner.as_os_str()
    }

    /// Returns a [`str`] slice if the [`Path`] is valid unicode.
    #[inline]
    pub fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    /// Converts a [`Path`] to a `Cow<str>`.
    ///
    /// Any non-Unicode sequences are replaced with `U+FFFD REPLACEMENT CHARACTER`.
    #[inline]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.inner.to_string_lossy()
    }

    /// Converts a [`Path`] to an owned [`PathBuf`].
    #[inline]
    pub fn to_path_buf(&self) -> PathBuf<Form> {
        PathBuf::new_unchecked(self.inner.to_path_buf())
    }

    /// Returns the [`Path`] without its final component, if there is one.
    ///
    /// This means it returns `Some("")` for relative paths with one component.
    ///
    /// Returns [`None`] if the path terminates in a root or prefix, or if it's
    /// the empty string.
    #[inline]
    pub fn parent(&self) -> Option<&Self> {
        self.inner.parent().map(Self::new_unchecked)
    }

    /// Produces an iterator over a [`Path`] and its ancestors.
    ///
    /// The iterator will yield the [`Path`] that is returned if the [`parent`](Path::parent) method
    /// is used zero or more times. That means, the iterator will yield `&self`,
    /// `&self.parent().unwrap()`, `&self.parent().unwrap().parent().unwrap()` and so on.
    /// If the [`parent`](Path::parent) method returns [`None`], the iterator will do likewise.
    /// The iterator will always yield at least one value, namely `&self`.
    #[inline]
    pub fn ancestors(&self) -> std::path::Ancestors<'_> {
        self.inner.ancestors()
    }

    /// Returns the final component of a [`Path`], if there is one.
    ///
    /// If the path is a normal file, this is the file name. If it's the path of a directory, this
    /// is the directory name.
    ///
    /// Returns [`None`] if the path terminates in `..`.
    #[inline]
    pub fn file_name(&self) -> Option<&OsStr> {
        self.inner.file_name()
    }

    /// Returns a relative path that, when joined onto `base`, yields `self`.
    #[inline]
    pub fn strip_prefix<F: PathForm>(
        &self,
        base: impl AsRef<Path<F>>,
    ) -> Result<&RelativePath, StripPrefixError> {
        self.inner
            .strip_prefix(&base.as_ref().inner)
            .map(RelativePath::new_unchecked)
    }

    /// Determines whether `base` is a prefix of `self`.
    ///
    /// Only considers whole path components to match.
    #[inline]
    pub fn starts_with<F: PathForm>(&self, base: impl AsRef<Path<F>>) -> bool {
        self.inner.starts_with(&base.as_ref().inner)
    }

    /// Determines whether `child` is a suffix of `self`.
    ///
    /// Only considers whole path components to match.
    #[inline]
    pub fn ends_with<F: PathForm>(&self, child: impl AsRef<Path<F>>) -> bool {
        self.inner.ends_with(&child.as_ref().inner)
    }

    /// Extracts the stem (non-extension) portion of [`self.file_name`](Path::file_name).
    ///
    /// The stem is:
    ///
    /// * [`None`], if there is no file name;
    /// * The entire file name if there is no embedded `.`;
    /// * The entire file name if the file name begins with `.` and has no other `.`s within;
    /// * Otherwise, the portion of the file name before the final `.`
    #[inline]
    pub fn file_stem(&self) -> Option<&OsStr> {
        self.inner.file_stem()
    }

    /// Extracts the extension (without the leading dot) of [`self.file_name`](Path::file_name),
    /// if possible.
    ///
    /// The extension is:
    ///
    /// * [`None`], if there is no file name;
    /// * [`None`], if there is no embedded `.`;
    /// * [`None`], if the file name begins with `.` and has no other `.`s within;
    /// * Otherwise, the portion of the file name after the final `.`
    #[inline]
    pub fn extension(&self) -> Option<&OsStr> {
        self.inner.extension()
    }

    /// Produces an iterator over the [`Component`](std::path::Component)s of the path.
    ///
    /// When parsing the path, there is a small amount of normalization:
    ///
    /// * Repeated separators are ignored, so `a/b` and `a//b` both have
    ///   `a` and `b` as components.
    ///
    /// * Occurrences of `.` are normalized away, except if they are at the
    ///   beginning of the path. For example, `a/./b`, `a/b/`, `a/b/.` and
    ///   `a/b` all have `a` and `b` as components, but `./a/b` starts with
    ///   an additional [`CurDir`](std::path::Component) component.
    ///
    /// * A trailing slash is normalized away, `/a/b` and `/a/b/` are equivalent.
    ///
    /// Note that no other normalization takes place; in particular, `a/c`
    /// and `a/b/../c` are distinct, to account for the possibility that `b`
    /// is a symbolic link (so its parent isn't `a`).
    #[inline]
    pub fn components(&self) -> std::path::Components<'_> {
        self.inner.components()
    }

    /// Produces an iterator over the path's components viewed as [`OsStr`] slices.
    ///
    /// For more information about the particulars of how the path is separated into components,
    /// see [`components`](Path::components).
    #[inline]
    pub fn iter(&self) -> std::path::Iter<'_> {
        self.inner.iter()
    }

    /// Returns an object that implements [`Display`](fmt::Display) for safely printing paths
    /// that may contain non-Unicode data. This may perform lossy conversion,
    /// depending on the platform. If you would like an implementation which escapes the path
    /// please use [`Debug`](fmt::Debug) instead.
    #[inline]
    pub fn display(&self) -> std::path::Display<'_> {
        self.inner.display()
    }

    /// Converts a [`Box<Path>`](Box) into a [`PathBuf`] without copying or allocating.
    #[inline]
    pub fn into_path_buf(self: Box<Self>) -> PathBuf<Form> {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let ptr = Box::into_raw(self) as *mut std::path::Path;
        let boxed = unsafe { Box::from_raw(ptr) };
        PathBuf::new_unchecked(boxed.into_path_buf())
    }

    /// Returns a reference to the same [`Path`] in a different form.
    ///
    /// [`PathForm`]s can be converted to one another based on [`PathCast`] implementations.
    /// Namely, the following form conversions are possible:
    /// - [`Relative`], [`Absolute`], or [`Canonical`] into [`Any`].
    /// - [`Canonical`] into [`Absolute`].
    /// - Any form into itself.
    #[inline]
    pub fn cast<To>(&self) -> &Path<To>
    where
        To: PathForm,
        Form: PathCast<To>,
    {
        Path::new_unchecked(self)
    }

    /// Returns a reference to a path with its form as [`Any`].
    #[inline]
    pub fn as_any(&self) -> &Path {
        Path::new_unchecked(self)
    }
}

impl Path {
    /// Create a new [`Path`] by wrapping a string slice.
    ///
    /// This is a cost-free conversion.
    #[inline]
    pub fn new<P: AsRef<OsStr> + ?Sized>(path: &P) -> &Self {
        Self::new_unchecked(path)
    }

    /// Returns a mutable reference to the underlying [`OsStr`] slice.
    #[inline]
    pub fn as_mut_os_str(&mut self) -> &mut OsStr {
        self.inner.as_mut_os_str()
    }

    /// Returns `true` if the [`Path`] is absolute, i.e., if it is independent of
    /// the current directory.
    ///
    /// * On Unix, a path is absolute if it starts with the root,
    /// so [`is_absolute`](Path::is_absolute) and [`has_root`](Path::has_root) are equivalent.
    ///
    /// * On Windows, a path is absolute if it has a prefix and starts with the root:
    /// `c:\windows` is absolute, while `c:temp` and `\temp` are not.
    #[inline]
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    // Returns `true` if the [`Path`] is relative, i.e., not absolute.
    ///
    /// See [`is_absolute`](Path::is_absolute)'s documentation for more details.
    #[inline]
    pub fn is_relative(&self) -> bool {
        self.inner.is_relative()
    }

    /// Returns an `Ok` [`AbsolutePath`] if the [`Path`] is absolute.
    /// Otherwise, returns an `Err` [`RelativePath`].
    #[inline]
    pub fn try_absolute(&self) -> Result<&AbsolutePath, &RelativePath> {
        self.is_absolute()
            .then_some(AbsolutePath::new_unchecked(&self.inner))
            .ok_or(RelativePath::new_unchecked(&self.inner))
    }

    /// Returns an `Ok` [`RelativePath`] if the [`Path`] is relative.
    /// Otherwise, returns an `Err` [`AbsolutePath`].
    #[inline]
    pub fn try_relative(&self) -> Result<&RelativePath, &AbsolutePath> {
        self.is_relative()
            .then_some(RelativePath::new_unchecked(&self.inner))
            .ok_or(AbsolutePath::new_unchecked(&self.inner))
    }
}

impl<Form: PathJoin> Path<Form> {
    /// Creates an owned [`PathBuf`] with `path` adjoined to `self`.
    ///
    /// If `path` is absolute, it replaces the current path.
    ///
    /// See [`PathBuf::push`] for more details on what it means to adjoin a path.
    #[inline]
    pub fn join<F: MaybeRelative>(&self, path: impl AsRef<Path<F>>) -> PathBuf<Form::Output> {
        PathBuf::new_unchecked(self.inner.join(&path.as_ref().inner))
    }
}

impl<Form: PathSet> Path<Form> {
    /// Creates an owned [`PathBuf`] like `self` but with the given file name.
    ///
    /// See [`PathBuf::set_file_name`] for more details.
    #[inline]
    pub fn with_file_name(&self, file_name: impl AsRef<OsStr>) -> PathBuf<Form> {
        PathBuf::new_unchecked(self.inner.with_file_name(file_name))
    }

    /// Creates an owned [`PathBuf`] like `self` but with the given extension.
    ///
    /// See [`PathBuf::set_extension`] for more details.
    #[inline]
    pub fn with_extension(&self, extension: impl AsRef<OsStr>) -> PathBuf<Form> {
        PathBuf::new_unchecked(self.inner.with_extension(extension))
    }
}

impl<Form: MaybeRelative> Path<Form> {
    /// Returns the, potentially relative, underlying [`std::path::Path`].
    ///
    /// # Note
    ///
    /// Caution should be taken when using this function. Nushell keeps track of an emulated current
    /// working directory, and using the [`std::path::Path`] returned from this method will likely
    /// use [`std::env::current_dir`] to resolve the path instead of using the emulated current
    /// working directory.
    ///
    /// Instead, you should probably join this path onto the emulated current working directory.
    /// Any [`AbsolutePath`] or [`CanonicalPath`] will also suffice.
    #[inline]
    pub fn as_relative_std_path(&self) -> &std::path::Path {
        &self.inner
    }

    // Returns `true` if the [`Path`] has a root.
    ///
    /// * On Unix, a path has a root if it begins with `/`.
    ///
    /// * On Windows, a path has a root if it:
    ///     * has no prefix and begins with a separator, e.g., `\windows`
    ///     * has a prefix followed by a separator, e.g., `c:\windows` but not `c:windows`
    ///     * has any non-disk prefix, e.g., `\\server\share`
    #[inline]
    pub fn has_root(&self) -> bool {
        self.inner.has_root()
    }
}

impl<Form: IsAbsolute> Path<Form> {
    /// Returns the underlying [`std::path::Path`].
    #[inline]
    pub fn as_std_path(&self) -> &std::path::Path {
        &self.inner
    }

    /// Converts a [`Path`] to an owned [`std::path::PathBuf`].
    #[inline]
    pub fn to_std_pathbuf(&self) -> std::path::PathBuf {
        self.inner.to_path_buf()
    }

    /// Queries the file system to get information about a file, directory, etc.
    ///
    /// This function will traverse symbolic links to query information about the destination file.
    ///
    /// This is an alias to [`std::fs::metadata`].
    #[inline]
    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        self.inner.metadata()
    }

    /// Returns an iterator over the entries within a directory.
    ///
    /// The iterator will yield instances of <code>[io::Result]<[fs::DirEntry]></code>.
    /// New errors may be encountered after an iterator is initially constructed.
    ///
    /// This is an alias to [`std::fs::read_dir`].
    #[inline]
    pub fn read_dir(&self) -> io::Result<fs::ReadDir> {
        self.inner.read_dir()
    }

    /// Returns `true` if the path points at an existing entity.
    ///
    /// Warning: this method may be error-prone, consider using [`try_exists`](Path::try_exists)
    /// instead! It also has a risk of introducing time-of-check to time-of-use (TOCTOU) bugs.
    ///
    /// This function will traverse symbolic links to query information about the destination file.
    ///
    /// If you cannot access the metadata of the file, e.g. because of a permission error
    /// or broken symbolic links, this will return `false`.
    #[inline]
    pub fn exists(&self) -> bool {
        self.inner.exists()
    }

    /// Returns `true` if the path exists on disk and is pointing at a regular file.
    ///
    /// This function will traverse symbolic links to query information about the destination file.
    ///
    /// If you cannot access the metadata of the file, e.g. because of a permission error
    /// or broken symbolic links, this will return `false`.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.inner.is_file()
    }

    /// Returns `true` if the path exists on disk and is pointing at a directory.
    ///
    /// This function will traverse symbolic links to query information about the destination file.
    ///
    /// If you cannot access the metadata of the file, e.g. because of a permission error
    /// or broken symbolic links, this will return `false`.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.inner.is_dir()
    }
}

impl AbsolutePath {
    /// Returns the canonical, absolute form of the path with all intermediate components
    /// normalized and symbolic links resolved.
    ///
    /// On Windows, this will also simplify to a winuser path.
    ///
    /// This is an alias to [`std::fs::canonicalize`].
    #[cfg(not(windows))]
    #[inline]
    pub fn canonicalize(&self) -> io::Result<CanonicalPathBuf> {
        self.inner
            .canonicalize()
            .map(CanonicalPathBuf::new_unchecked)
    }

    /// Returns the canonical, absolute form of the path with all intermediate components
    /// normalized and symbolic links resolved.
    ///
    /// On Windows, this will also simplify to a winuser path.
    ///
    /// This is an alias to [`std::fs::canonicalize`].
    #[cfg(windows)]
    pub fn canonicalize(&self) -> io::Result<CanonicalPathBuf> {
        use omnipath::WinPathExt;

        let path = self.inner.canonicalize()?.to_winuser_path()?;
        Ok(CanonicalPathBuf::new_unchecked(path))
    }

    /// Reads a symbolic link, returning the file that the link points to.
    ///
    /// This is an alias to [`std::fs::read_link`].
    #[inline]
    pub fn read_link(&self) -> io::Result<AbsolutePathBuf> {
        self.inner.read_link().map(PathBuf::new_unchecked)
    }

    /// Returns `Ok(true)` if the path points at an existing entity.
    ///
    /// This function will traverse symbolic links to query information about the destination file.
    /// In case of broken symbolic links this will return `Ok(false)`.
    ///
    /// [`Path::exists`] only checks whether or not a path was both found and readable.
    /// By contrast, [`try_exists`](Path::try_exists) will return `Ok(true)` or `Ok(false)`,
    /// respectively, if the path was _verified_ to exist or not exist.
    /// If its existence can neither be confirmed nor denied, it will propagate an `Err` instead.
    /// This can be the case if e.g. listing permission is denied on one of the parent directories.
    ///
    /// Note that while this avoids some pitfalls of the [`exist`](Path::exists) method,
    /// it still can not prevent time-of-check to time-of-use (TOCTOU) bugs.
    /// You should only use it in scenarios where those bugs are not an issue.
    #[inline]
    pub fn try_exists(&self) -> io::Result<bool> {
        self.inner.try_exists()
    }

    /// Returns `true` if the path exists on disk and is pointing at a symbolic link.
    ///
    /// This function will not traverse symbolic links.
    /// In case of a broken symbolic link this will also return true.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a permission error,
    /// this will return false.
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.inner.is_symlink()
    }

    /// Queries the metadata about a file without following symlinks.
    ///
    /// This is an alias to [`std::fs::symlink_metadata`].
    #[inline]
    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        self.inner.symlink_metadata()
    }
}

impl CanonicalPath {
    /// Returns a [`CanonicalPath`] as a [`AbsolutePath`].
    #[inline]
    pub fn as_absolute(&self) -> &AbsolutePath {
        self.cast()
    }
}

impl<Form: PathForm> fmt::Debug for Path<Form> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, fmt)
    }
}

impl<Form: PathForm> Clone for Box<Path<Form>> {
    #[inline]
    fn clone(&self) -> Self {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let path: Box<std::path::Path> = self.inner.into();
        let ptr = Box::into_raw(path) as *mut Path<Form>;
        unsafe { Box::from_raw(ptr) }
    }
}

impl<Form: PathForm> ToOwned for Path<Form> {
    type Owned = PathBuf<Form>;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        self.to_path_buf()
    }

    #[inline]
    fn clone_into(&self, target: &mut PathBuf<Form>) {
        self.inner.clone_into(&mut target.inner);
    }
}

impl<'a, Form: PathForm> IntoIterator for &'a Path<Form> {
    type Item = &'a OsStr;

    type IntoIter = std::path::Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[repr(transparent)]
pub struct PathBuf<Form: PathForm = Any> {
    _form: PhantomData<Form>,
    inner: std::path::PathBuf,
}

pub type RelativePathBuf = PathBuf<Relative>;

pub type AbsolutePathBuf = PathBuf<Absolute>;

pub type CanonicalPathBuf = PathBuf<Canonical>;

impl<Form: PathForm> PathBuf<Form> {
    #[inline]
    pub(crate) fn new_unchecked(buf: std::path::PathBuf) -> Self {
        Self {
            _form: PhantomData,
            inner: buf,
        }
    }

    #[inline]
    pub fn as_path(&self) -> &Path<Form> {
        Path::new_unchecked(&self.inner)
    }

    #[inline]
    pub fn pop(&mut self) -> bool {
        self.inner.pop()
    }

    #[inline]
    pub fn into_os_string(self) -> OsString {
        self.inner.into_os_string()
    }

    #[inline]
    pub fn into_boxed_path(self) -> Box<Path<Form>> {
        std_box_to_box(self.inner.into_boxed_path())
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional)
    }

    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.inner.try_reserve(additional)
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.inner.reserve_exact(additional)
    }

    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.inner.try_reserve_exact(additional)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit()
    }

    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.inner.shrink_to(min_capacity)
    }

    #[inline]
    pub fn cast_into<To>(self) -> PathBuf<To>
    where
        To: PathForm,
        Form: PathCast<To>,
    {
        PathBuf::new_unchecked(self.inner)
    }

    #[inline]
    pub fn into_any(self) -> PathBuf {
        PathBuf::new_unchecked(self.inner)
    }
}

impl PathBuf {
    #[inline]
    pub fn new() -> Self {
        Self::new_unchecked(std::path::PathBuf::new())
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::new_unchecked(std::path::PathBuf::with_capacity(capacity))
    }

    #[inline]
    pub fn as_mut_os_string(&mut self) -> &mut OsString {
        self.inner.as_mut_os_string()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    #[inline]
    pub fn try_into_relative(self) -> Result<RelativePathBuf, Self> {
        if self.inner.is_relative() {
            Ok(PathBuf::new_unchecked(self.inner))
        } else {
            Err(self)
        }
    }

    #[inline]
    pub fn try_into_absolute(self) -> Result<AbsolutePathBuf, Self> {
        if self.inner.is_absolute() {
            Ok(PathBuf::new_unchecked(self.inner))
        } else {
            Err(self)
        }
    }
}

impl<Form: PathPush> PathBuf<Form> {
    #[inline]
    pub fn push<R: MaybeRelative>(&mut self, path: impl AsRef<Path<R>>) {
        self.inner.push(&path.as_ref().inner)
    }
}

impl<Form: PathSet> PathBuf<Form> {
    #[inline]
    pub fn set_file_name(&mut self, file_name: impl AsRef<OsStr>) {
        self.inner.set_file_name(file_name)
    }

    #[inline]
    pub fn set_extension(&mut self, extension: impl AsRef<OsStr>) -> bool {
        self.inner.set_extension(extension)
    }
}

impl<Form: MaybeRelative> PathBuf<Form> {
    #[inline]
    pub fn into_relative_std_pathbuf(self) -> std::path::PathBuf {
        self.inner
    }
}

impl<Form: IsAbsolute> PathBuf<Form> {
    #[inline]
    pub fn into_std_pathbuf(self) -> std::path::PathBuf {
        self.inner
    }
}

impl CanonicalPathBuf {
    #[inline]
    pub fn into_absolute(self) -> AbsolutePathBuf {
        self.cast_into()
    }
}

impl Default for PathBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<Form: PathForm> Clone for PathBuf<Form> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            _form: PhantomData,
            inner: self.inner.clone(),
        }
    }
}

impl<Form: PathForm> fmt::Debug for PathBuf<Form> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<Form: PathForm> Deref for PathBuf<Form> {
    type Target = Path<Form>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for PathBuf {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let path: &mut std::path::Path = &mut self.inner;
        let ptr = std::ptr::from_mut(path) as *mut Path;
        unsafe { &mut *ptr }
    }
}

impl<From: PathCast<To>, To: PathForm> Borrow<Path<To>> for PathBuf<From> {
    #[inline]
    fn borrow(&self) -> &Path<To> {
        self.cast()
    }
}

impl<Form: IsAbsolute> Borrow<std::path::Path> for PathBuf<Form> {
    #[inline]
    fn borrow(&self) -> &std::path::Path {
        self.as_ref()
    }
}

impl Borrow<Path> for std::path::PathBuf {
    #[inline]
    fn borrow(&self) -> &Path {
        self.as_ref()
    }
}

impl FromStr for PathBuf {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl FromStr for RelativePathBuf {
    type Err = TryRelativeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl FromStr for AbsolutePathBuf {
    type Err = TryAbsoluteError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl<P: AsRef<Path>> Extend<P> for PathBuf {
    fn extend<T: IntoIterator<Item = P>>(&mut self, iter: T) {
        for path in iter {
            self.push(path);
        }
    }
}

impl<P: AsRef<Path>> FromIterator<P> for PathBuf {
    fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
        let mut buf = Self::new_unchecked(std::path::PathBuf::new());
        buf.extend(iter);
        buf
    }
}

impl<'a, Form: PathForm> IntoIterator for &'a PathBuf<Form> {
    type Item = &'a OsStr;

    type IntoIter = std::path::Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[inline]
fn std_box_to_box<Form: PathForm>(path: Box<std::path::Path>) -> Box<Path<Form>> {
    // Safety: `Path<From>` is a repr(transparent) wrapper around `std::path::Path`.
    let ptr = Box::into_raw(path) as *mut Path<Form>;
    unsafe { Box::from_raw(ptr) }
}

/*
================================================================================
  AsRef
================================================================================
*/

// Here we match all `AsRef` implementations on `std::path::Path` and `std::path::PathBuf`,
// adding casting variations where possible.

macro_rules! impl_as_ref {
    ([$($from:ty),* $(,)?] => $to:ty |$self:ident| $cast:block) => {
        $(
            impl AsRef<$to> for $from {
                #[inline]
                fn as_ref(&$self) -> &$to $cast
            }
        )*
    };
}

// === To and from crate types ===

impl<From: PathCast<To>, To: PathForm> AsRef<Path<To>> for Path<From> {
    #[inline]
    fn as_ref(&self) -> &Path<To> {
        self.cast()
    }
}

impl<From: PathCast<To>, To: PathForm> AsRef<Path<To>> for PathBuf<From> {
    #[inline]
    fn as_ref(&self) -> &Path<To> {
        self.cast()
    }
}

impl_as_ref!(
    [
        Box<RelativePath>, Box<AbsolutePath>, Box<CanonicalPath>,
        Cow<'_, RelativePath>, Cow<'_, AbsolutePath>, Cow<'_, CanonicalPath>,
        Rc<RelativePath>, Rc<AbsolutePath>, Rc<CanonicalPath>,
        Arc<RelativePath>, Arc<AbsolutePath>, Arc<CanonicalPath>,
    ]
    => Path |self| { self.cast() }
);

impl_as_ref!(
    [Box<CanonicalPath>, Cow<'_, CanonicalPath>, Rc<CanonicalPath>, Arc<CanonicalPath>]
    => AbsolutePath |self| { self.cast() }
);

// === To and from std::path types ===

impl<Form: IsAbsolute> AsRef<std::path::Path> for Path<Form> {
    #[inline]
    fn as_ref(&self) -> &std::path::Path {
        self.as_std_path()
    }
}

impl<Form: IsAbsolute> AsRef<std::path::Path> for PathBuf<Form> {
    #[inline]
    fn as_ref(&self) -> &std::path::Path {
        self.as_std_path()
    }
}

impl_as_ref!(
    [std::path::Path, std::path::PathBuf, std::path::Component<'_>]
    => Path |self| { Path::new(self) }
);

impl_as_ref!(
    [Box<std::path::Path>, Cow<'_, std::path::Path>, Rc<std::path::Path>, Arc<std::path::Path>]
    => Path |self| { Path::new(self.as_os_str()) }
);

// === To and from string types ===

impl<Form: PathForm> AsRef<OsStr> for Path<Form> {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl<Form: PathForm> AsRef<OsStr> for PathBuf<Form> {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl_as_ref!([OsStr, OsString, Cow<'_, OsStr>, str, String] => Path |self| { Path::new(self) });

/*
================================================================================
  From
================================================================================
*/

// Here we match all `From` implementations on `std::path::Path` and `std::path::PathBuf`,
// adding casting variations where possible.

macro_rules! impl_from {
    ([$($from:ty),* $(,)?] => $to:ty |$value:ident| $convert:block) => {
        $(
            impl From<$from> for $to {
                #[inline]
                fn from($value: $from) -> Self $convert
            }
        )*
    };
    (<$form:ident> $from:ty => $to:ty |$value:ident| $convert:block) => {
        impl<$form: PathForm> From<$from> for $to {
            #[inline]
            fn from($value: $from) -> Self $convert
        }
    };
}

macro_rules! impl_into_std {
    (<$form:ident> $from:ty => [$($to:ty),* $(,)?] |$value:ident| $convert:block) => {
        $(
            impl<$form: IsAbsolute> From<$from> for $to {
                #[inline]
                fn from($value: $from) -> Self $convert
            }
        )*
    };
}

// ===== Owned to Owned =====

// === To and from crate types ===

impl_from!([RelativePathBuf, AbsolutePathBuf, CanonicalPathBuf] => PathBuf
    |buf| { buf.cast_into() }
);
impl_from!([CanonicalPathBuf] => AbsolutePathBuf |buf| { buf.cast_into() });

#[inline]
fn box_to_box<From: PathCast<To>, To: PathForm>(path: Box<Path<From>>) -> Box<Path<To>> {
    // Safety: `Path<From>` and `Path<To>` differ only by PhantomData tag.
    let ptr = Box::into_raw(path) as *mut Path<To>;
    unsafe { Box::from_raw(ptr) }
}
impl_from!([Box<RelativePath>, Box<AbsolutePath>, Box<CanonicalPath>] => Box<Path>
    |path| { box_to_box(path) }
);
impl_from!([Box<CanonicalPath>] => Box<AbsolutePath> |path| { box_to_box(path) });

impl_from!(<Form> PathBuf<Form> => Box<Path<Form>> |buf| { buf.into_boxed_path() });
impl_from!([RelativePathBuf, AbsolutePathBuf, CanonicalPathBuf] => Box<Path>
    |buf| { buf.into_boxed_path().into() }
);
impl_from!([CanonicalPathBuf] => Box<AbsolutePath> |buf| { buf.into_boxed_path().into() });

impl_from!(<Form> Box<Path<Form>> => PathBuf<Form> |path| { path.into_path_buf() });
impl_from!([Box<RelativePath>, Box<AbsolutePath>, Box<CanonicalPath>] => PathBuf
    |path| { path.into_path_buf().into() }
);
impl_from!([Box<CanonicalPath>] => AbsolutePathBuf |path| { path.into_path_buf().into() });

impl_from!(<Form> PathBuf<Form> => Cow<'_, Path<Form>> |buf| { Self::Owned(buf) });
impl_from!([RelativePathBuf, AbsolutePathBuf, CanonicalPathBuf] => Cow<'_, Path>
    |buf| { Self::Owned(buf.into()) }
);
impl_from!([CanonicalPathBuf] => Cow<'_, AbsolutePath> |buf| { Self::Owned(buf.into()) });

impl_from!(<Form> Cow<'_, Path<Form>> => PathBuf<Form> |cow| { cow.into_owned() });
impl_from!([Cow<'_, RelativePath>, Cow<'_, AbsolutePath>, Cow<'_, CanonicalPath>] => PathBuf
    |cow| { cow.into_owned().into() }
);
impl_from!([Cow<'_, CanonicalPath>] => AbsolutePathBuf |cow| { cow.into_owned().into() });

#[inline]
fn cow_to_box<From, To>(cow: Cow<'_, From>) -> Box<To>
where
    From: ?Sized + ToOwned,
    for<'a> &'a From: Into<Box<To>>,
    From::Owned: Into<Box<To>>,
    To: ?Sized,
{
    match cow {
        Cow::Borrowed(path) => path.into(),
        Cow::Owned(path) => path.into(),
    }
}
impl_from!(<Form> Cow<'_, Path<Form>> => Box<Path<Form>> |cow| { cow_to_box(cow) });
impl_from!([Cow<'_, RelativePath>, Cow<'_, AbsolutePath>, Cow<'_, CanonicalPath>] => Box<Path>
    |cow| { cow_to_box(cow) }
);
impl_from!([Cow<'_, CanonicalPath>] => Box<AbsolutePath> |cow| { cow_to_box(cow) });

#[inline]
fn buf_to_arc<From: PathCast<To>, To: PathForm>(buf: PathBuf<From>) -> Arc<Path<To>> {
    // Safety: `Path<To>` is a repr(transparent) wrapper around `std::path::Path`.
    let arc: Arc<std::path::Path> = buf.inner.into();
    let ptr = Arc::into_raw(arc) as *const Path<To>;
    unsafe { Arc::from_raw(ptr) }
}
impl_from!(<Form> PathBuf<Form> => Arc<Path<Form>> |buf| { buf_to_arc(buf) });
impl_from!([RelativePathBuf, AbsolutePathBuf, CanonicalPathBuf] => Arc<Path>
    |buf| { buf_to_arc(buf) }
);
impl_from!([CanonicalPathBuf] => Arc<AbsolutePath> |buf| { buf_to_arc(buf) });

#[inline]
fn buf_to_rc<From: PathCast<To>, To: PathForm>(buf: PathBuf<From>) -> Rc<Path<To>> {
    // Safety: `Path<To>` is a repr(transparent) wrapper around `std::path::Path`.
    let rc: Rc<std::path::Path> = buf.inner.into();
    let ptr = Rc::into_raw(rc) as *const Path<To>;
    unsafe { Rc::from_raw(ptr) }
}
impl_from!(<Form> PathBuf<Form> => Rc<Path<Form>> |buf| { buf_to_rc(buf) });
impl_from!([RelativePathBuf, AbsolutePathBuf, CanonicalPathBuf] => Rc<Path>
    |buf| { buf_to_rc(buf) }
);
impl_from!([CanonicalPathBuf] => Rc<AbsolutePath> |buf| { buf_to_rc(buf) });

// === To and from std::path types ===

impl_into_std!(<Form> PathBuf<Form> => [std::path::PathBuf] |buf| { buf.inner });
impl_into_std!(
    <Form> PathBuf<Form> => [
        Box<std::path::Path>, Cow<'_, std::path::Path>, Arc<std::path::Path>, Rc<std::path::Path>
    ]
    |buf| { buf.inner.into() }
);
impl_into_std!(<Form> Box<Path<Form>> => [std::path::PathBuf, Box<std::path::Path>]
    |path| { path.inner.into() }
);

impl_from!([std::path::PathBuf] => PathBuf |buf| { Self::new_unchecked(buf) });
impl_from!([Box<std::path::Path>] => PathBuf |path| { Self::new_unchecked(path.into()) });
impl_from!([Cow<'_, std::path::Path>] => PathBuf |cow| { Self::new_unchecked(cow.into()) });

impl From<Box<std::path::Path>> for Box<Path> {
    #[inline]
    fn from(path: Box<std::path::Path>) -> Self {
        std_box_to_box(path)
    }
}
impl_from!([std::path::PathBuf] => Box<Path> |buf| { buf.into_boxed_path().into() });
impl_from!([Cow<'_, std::path::Path>] => Box<Path> |cow| { cow_to_box(cow) });

// === To and from string types ===

impl_from!(<Form> PathBuf<Form> => OsString |buf| { buf.inner.into() });
impl_from!([OsString, String] => PathBuf |s| { Self::new_unchecked(s.into()) });

// ===== Borrowed to Owned =====

// === To and from crate types ===
// Here we also add casting conversions from `T: impl AsRef<Path<Form>>` to `PathBuf<Form>`.

impl<Source: PathCast<To>, To: PathForm> From<&Path<Source>> for Box<Path<To>> {
    #[inline]
    fn from(path: &Path<Source>) -> Self {
        std_box_to_box(path.inner.into())
    }
}

impl<'a, Source: PathCast<To>, To: PathForm> From<&'a Path<Source>> for Cow<'a, Path<To>> {
    #[inline]
    fn from(path: &'a Path<Source>) -> Self {
        path.cast().into()
    }
}

impl<'a, Source: PathCast<To>, To: PathForm> From<&'a PathBuf<Source>> for Cow<'a, Path<To>> {
    #[inline]
    fn from(buf: &'a PathBuf<Source>) -> Self {
        buf.cast().into()
    }
}

impl<Source: PathCast<To>, To: PathForm> From<&Path<Source>> for Arc<Path<To>> {
    #[inline]
    fn from(path: &Path<Source>) -> Self {
        // Safety: `Path<Source>` is a repr(transparent) wrapper around `std::path::Path`.
        let arc: Arc<std::path::Path> = path.inner.into();
        let ptr = Arc::into_raw(arc) as *const Path<To>;
        unsafe { Arc::from_raw(ptr) }
    }
}

impl<Source: PathCast<To>, To: PathForm> From<&Path<Source>> for Rc<Path<To>> {
    #[inline]
    fn from(path: &Path<Source>) -> Self {
        // Safety: `Path<Source>` is a repr(transparent) wrapper around `std::path::Path`.
        let rc: Rc<std::path::Path> = path.inner.into();
        let ptr = Rc::into_raw(rc) as *const Path<To>;
        unsafe { Rc::from_raw(ptr) }
    }
}

impl<T: ?Sized + AsRef<RelativePath>> From<&T> for RelativePathBuf {
    #[inline]
    fn from(s: &T) -> Self {
        s.as_ref().into()
    }
}

impl<T: ?Sized + AsRef<AbsolutePath>> From<&T> for AbsolutePathBuf {
    #[inline]
    fn from(s: &T) -> Self {
        s.as_ref().into()
    }
}

impl<T: ?Sized + AsRef<CanonicalPath>> From<&T> for CanonicalPathBuf {
    #[inline]
    fn from(s: &T) -> Self {
        s.as_ref().into()
    }
}

// === To and from std::path types ===

impl_into_std!(
    <Form> &Path<Form> => [Box<std::path::Path>, Arc<std::path::Path>, Rc<std::path::Path>]
    |path| { path.inner.into() }
);

impl<'a, Form: IsAbsolute> From<&'a Path<Form>> for Cow<'a, std::path::Path> {
    #[inline]
    fn from(path: &'a Path<Form>) -> Self {
        path.inner.into()
    }
}

impl<'a, Form: IsAbsolute> From<&'a PathBuf<Form>> for Cow<'a, std::path::Path> {
    #[inline]
    fn from(buf: &'a PathBuf<Form>) -> Self {
        Self::Borrowed(buf.as_ref())
    }
}

impl_from!([&std::path::Path] => Box<Path> |path| { Path::new(path).into() });

// === To and from string types ===

impl<T: ?Sized + AsRef<OsStr>> From<&T> for PathBuf {
    #[inline]
    fn from(s: &T) -> Self {
        Self::new_unchecked(s.as_ref().into())
    }
}

/*
================================================================================
  TryFrom
================================================================================
*/

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryRelativeError;

impl fmt::Display for TryRelativeError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "path was not a relative path")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryAbsoluteError;

impl fmt::Display for TryAbsoluteError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "path was not an absolute path")
    }
}

// ===== Borrowed to borrowed =====
// Here we match all `AsRef` implementations on `std::path::Path`.

macro_rules! impl_try_from_borrowed_to_borrowed {
    ([$($from:ty),* $(,)?], |$value:ident| $convert:block $(,)?) => {
        $(
            impl<'a> TryFrom<&'a $from> for &'a RelativePath {
                type Error = TryRelativeError;

                #[inline]
                fn try_from($value: &'a $from) -> Result<Self, Self::Error> $convert
            }

            impl<'a> TryFrom<&'a $from> for &'a AbsolutePath {
                type Error = TryAbsoluteError;

                #[inline]
                fn try_from($value: &'a $from) -> Result<Self, Self::Error> $convert
            }
        )*
    };
}

// === From crate types ===

impl<'a> TryFrom<&'a Path> for &'a RelativePath {
    type Error = TryRelativeError;

    #[inline]
    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        path.try_relative().map_err(|_| TryRelativeError)
    }
}

impl<'a> TryFrom<&'a Path> for &'a AbsolutePath {
    type Error = TryAbsoluteError;

    #[inline]
    fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
        path.try_absolute().map_err(|_| TryAbsoluteError)
    }
}

impl_try_from_borrowed_to_borrowed!([PathBuf], |buf| { Path::new(buf).try_into() });

// === From std::path types ===

impl_try_from_borrowed_to_borrowed!([std::path::Path], |path| { Path::new(path).try_into() });
impl_try_from_borrowed_to_borrowed!([std::path::PathBuf], |buf| { Path::new(buf).try_into() });
impl_try_from_borrowed_to_borrowed!([std::path::Component<'_>], |component| {
    Path::new(component).try_into()
});
impl_try_from_borrowed_to_borrowed!([std::path::Components<'_>], |components| {
    Path::new(components).try_into()
});
impl_try_from_borrowed_to_borrowed!([std::path::Iter<'_>], |iter| { Path::new(iter).try_into() });

// === From string types ===

impl_try_from_borrowed_to_borrowed!(
    [OsStr, OsString, Cow<'_, OsStr>, str, String],
    |s| { Path::new(s).try_into() },
);

// ===== Borrowed to Owned =====
// Here we match all `From<&T>` implementations on `std::path::Path` and `std::path::PathBuf`.
// Note that to match `From<&T: AsRef<OsStr>>` on `std::path::PathBuf`,
// we add string conversions and a few others.

macro_rules! impl_try_from_borrowed_to_owned {
    ([$($from:ty),* $(,)?] => $rel:ty, $abs:ty $(,)?) => {
        $(
            impl TryFrom<&$from> for $rel {
                type Error = TryRelativeError;

                #[inline]
                fn try_from(path: &$from) -> Result<Self, Self::Error> {
                    let path: &RelativePath = path.try_into()?;
                    Ok(path.into())
                }
            }

            impl TryFrom<&$from> for $abs {
                type Error = TryAbsoluteError;

                #[inline]
                fn try_from(path: &$from) -> Result<Self, Self::Error> {
                    let path: &AbsolutePath = path.try_into()?;
                    Ok(path.into())
                }
            }
        )*
    };
    (<$life:lifetime> $from:ty => $rel:ty, $abs:ty $(,)?) => {
        impl<$life> TryFrom<&$life $from> for $rel {
            type Error = TryRelativeError;

            #[inline]
            fn try_from(path: &$life $from) -> Result<Self, Self::Error> {
                let path: &RelativePath = path.try_into()?;
                Ok(path.into())
            }
        }

        impl<$life> TryFrom<&$life $from> for $abs {
            type Error = TryAbsoluteError;

            #[inline]
            fn try_from(path: &$life $from) -> Result<Self, Self::Error> {
                let path: &AbsolutePath = path.try_into()?;
                Ok(path.into())
            }
        }
    };
}

// === From crate types ===

impl_try_from_borrowed_to_owned!([Path] => Box<RelativePath>, Box<AbsolutePath>);
impl_try_from_borrowed_to_owned!(<'a> Path => Cow<'a, RelativePath>, Cow<'a, AbsolutePath>);
impl_try_from_borrowed_to_owned!([Path] => Arc<RelativePath>, Arc<AbsolutePath>);
impl_try_from_borrowed_to_owned!([Path] => Rc<RelativePath>, Rc<AbsolutePath>);

impl_try_from_borrowed_to_owned!([Path, PathBuf] => RelativePathBuf, AbsolutePathBuf);
impl_try_from_borrowed_to_owned!(<'a> PathBuf => Cow<'a, RelativePath>, Cow<'a, AbsolutePath>);

// === From std::path types ===

impl_try_from_borrowed_to_owned!([std::path::Path] => Box<RelativePath>, Box<AbsolutePath>);

impl_try_from_borrowed_to_owned!(
    [std::path::Path, std::path::PathBuf, std::path::Component<'_>]
    => RelativePathBuf, AbsolutePathBuf
);

// === From string types ===

impl_try_from_borrowed_to_owned!(
    [OsStr, OsString, Cow<'_, OsStr>, str, String] => RelativePathBuf, AbsolutePathBuf
);

// ===== Owned to Owned =====
// Here we match all `From<T>` implementations on `std::path::Path` and `std::path::PathBuf`
// where `T` is an owned type.

// === From crate types ===

impl TryFrom<PathBuf> for RelativePathBuf {
    type Error = PathBuf;

    #[inline]
    fn try_from(buf: PathBuf) -> Result<Self, Self::Error> {
        buf.try_into_relative()
    }
}

impl TryFrom<PathBuf> for AbsolutePathBuf {
    type Error = PathBuf;

    #[inline]
    fn try_from(buf: PathBuf) -> Result<Self, Self::Error> {
        buf.try_into_absolute()
    }
}

impl TryFrom<Box<Path>> for RelativePathBuf {
    type Error = Box<Path>;

    #[inline]
    fn try_from(path: Box<Path>) -> Result<Self, Self::Error> {
        if path.is_relative() {
            Ok(Self::new_unchecked(path.inner.into()))
        } else {
            Err(path)
        }
    }
}

impl TryFrom<Box<Path>> for AbsolutePathBuf {
    type Error = Box<Path>;

    #[inline]
    fn try_from(path: Box<Path>) -> Result<Self, Self::Error> {
        if path.is_absolute() {
            Ok(Self::new_unchecked(path.inner.into()))
        } else {
            Err(path)
        }
    }
}

impl TryFrom<Box<Path>> for Box<RelativePath> {
    type Error = Box<Path>;

    #[inline]
    fn try_from(path: Box<Path>) -> Result<Self, Self::Error> {
        if path.is_relative() {
            // Safety: `Path` and `RelativePath` differ only by PhantomData tag.
            let ptr = Box::into_raw(path) as *mut RelativePath;
            Ok(unsafe { Box::from_raw(ptr) })
        } else {
            Err(path)
        }
    }
}

impl TryFrom<Box<Path>> for Box<AbsolutePath> {
    type Error = Box<Path>;

    #[inline]
    fn try_from(path: Box<Path>) -> Result<Self, Self::Error> {
        if path.is_absolute() {
            // Safety: `Path` and `AbsolutePath` differ only by PhantomData tag.
            let ptr = Box::into_raw(path) as *mut AbsolutePath;
            Ok(unsafe { Box::from_raw(ptr) })
        } else {
            Err(path)
        }
    }
}

impl TryFrom<PathBuf> for Box<RelativePath> {
    type Error = PathBuf;

    #[inline]
    fn try_from(buf: PathBuf) -> Result<Self, Self::Error> {
        RelativePathBuf::try_from(buf).map(Into::into)
    }
}

impl TryFrom<PathBuf> for Box<AbsolutePath> {
    type Error = PathBuf;

    #[inline]
    fn try_from(buf: PathBuf) -> Result<Self, Self::Error> {
        AbsolutePathBuf::try_from(buf).map(Into::into)
    }
}

impl<'a> TryFrom<Cow<'a, Path>> for RelativePathBuf {
    type Error = Cow<'a, Path>;

    #[inline]
    fn try_from(path: Cow<'a, Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Self::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Self::try_from(path).map_err(Cow::Owned),
        }
    }
}

impl<'a> TryFrom<Cow<'a, Path>> for AbsolutePathBuf {
    type Error = Cow<'a, Path>;

    #[inline]
    fn try_from(path: Cow<'a, Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Self::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Self::try_from(path).map_err(Cow::Owned),
        }
    }
}

impl<'a> TryFrom<Cow<'a, Path>> for Box<RelativePath> {
    type Error = Cow<'a, Path>;

    #[inline]
    fn try_from(path: Cow<'a, Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Box::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Box::try_from(path).map_err(Cow::Owned),
        }
    }
}

impl<'a> TryFrom<Cow<'a, Path>> for Box<AbsolutePath> {
    type Error = Cow<'a, Path>;

    #[inline]
    fn try_from(path: Cow<'a, Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Box::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Box::try_from(path).map_err(Cow::Owned),
        }
    }
}

// === From std::path types ===

impl TryFrom<std::path::PathBuf> for RelativePathBuf {
    type Error = std::path::PathBuf;

    #[inline]
    fn try_from(buf: std::path::PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(buf)).map_err(|buf| buf.inner)
    }
}

impl TryFrom<std::path::PathBuf> for AbsolutePathBuf {
    type Error = std::path::PathBuf;

    #[inline]
    fn try_from(buf: std::path::PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(buf)).map_err(|buf| buf.inner)
    }
}

impl TryFrom<Box<std::path::Path>> for RelativePathBuf {
    type Error = Box<std::path::Path>;

    #[inline]
    fn try_from(path: Box<std::path::Path>) -> Result<Self, Self::Error> {
        if path.is_relative() {
            Ok(Self::new_unchecked(path.into()))
        } else {
            Err(path)
        }
    }
}

impl TryFrom<Box<std::path::Path>> for AbsolutePathBuf {
    type Error = Box<std::path::Path>;

    #[inline]
    fn try_from(path: Box<std::path::Path>) -> Result<Self, Self::Error> {
        if path.is_absolute() {
            Ok(Self::new_unchecked(path.into()))
        } else {
            Err(path)
        }
    }
}

impl TryFrom<Box<std::path::Path>> for Box<RelativePath> {
    type Error = Box<std::path::Path>;

    #[inline]
    fn try_from(path: Box<std::path::Path>) -> Result<Self, Self::Error> {
        if path.is_relative() {
            Ok(std_box_to_box(path))
        } else {
            Err(path)
        }
    }
}

impl TryFrom<Box<std::path::Path>> for Box<AbsolutePath> {
    type Error = Box<std::path::Path>;

    #[inline]
    fn try_from(path: Box<std::path::Path>) -> Result<Self, Self::Error> {
        if path.is_absolute() {
            Ok(std_box_to_box(path))
        } else {
            Err(path)
        }
    }
}

impl TryFrom<std::path::PathBuf> for Box<RelativePath> {
    type Error = std::path::PathBuf;

    #[inline]
    fn try_from(buf: std::path::PathBuf) -> Result<Self, Self::Error> {
        RelativePathBuf::try_from(buf).map(Into::into)
    }
}

impl TryFrom<std::path::PathBuf> for Box<AbsolutePath> {
    type Error = std::path::PathBuf;

    #[inline]
    fn try_from(buf: std::path::PathBuf) -> Result<Self, Self::Error> {
        AbsolutePathBuf::try_from(buf).map(Into::into)
    }
}

impl<'a> TryFrom<Cow<'a, std::path::Path>> for RelativePathBuf {
    type Error = Cow<'a, std::path::Path>;

    #[inline]
    fn try_from(path: Cow<'a, std::path::Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Self::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Self::try_from(path).map_err(Cow::Owned),
        }
    }
}

impl<'a> TryFrom<Cow<'a, std::path::Path>> for AbsolutePathBuf {
    type Error = Cow<'a, std::path::Path>;

    #[inline]
    fn try_from(path: Cow<'a, std::path::Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Self::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Self::try_from(path).map_err(Cow::Owned),
        }
    }
}

impl<'a> TryFrom<Cow<'a, std::path::Path>> for Box<RelativePath> {
    type Error = Cow<'a, std::path::Path>;

    #[inline]
    fn try_from(path: Cow<'a, std::path::Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Box::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Box::try_from(path).map_err(Cow::Owned),
        }
    }
}

impl<'a> TryFrom<Cow<'a, std::path::Path>> for Box<AbsolutePath> {
    type Error = Cow<'a, std::path::Path>;

    #[inline]
    fn try_from(path: Cow<'a, std::path::Path>) -> Result<Self, Self::Error> {
        match path {
            Cow::Borrowed(path) => Box::try_from(path).map_err(|_| Cow::Borrowed(path)),
            Cow::Owned(path) => Box::try_from(path).map_err(Cow::Owned),
        }
    }
}

// === From string types ===

impl TryFrom<OsString> for RelativePathBuf {
    type Error = OsString;

    #[inline]
    fn try_from(s: OsString) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(s)).map_err(|buf| buf.into_os_string())
    }
}

impl TryFrom<OsString> for AbsolutePathBuf {
    type Error = OsString;

    #[inline]
    fn try_from(s: OsString) -> Result<Self, Self::Error> {
        Self::try_from(PathBuf::from(s)).map_err(|buf| buf.into_os_string())
    }
}

impl TryFrom<String> for RelativePathBuf {
    type Error = String;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if Path::new(&s).is_relative() {
            Ok(Self::new_unchecked(s.into()))
        } else {
            Err(s)
        }
    }
}

impl TryFrom<String> for AbsolutePathBuf {
    type Error = String;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if Path::new(&s).is_absolute() {
            Ok(Self::new_unchecked(s.into()))
        } else {
            Err(s)
        }
    }
}

/*
================================================================================
  PartialEq, Eq, PartialOrd, and Ord
================================================================================
*/

// Here we match all `PartialEq` and `PartialOrd` implementations on `std::path::Path`
// and `std::path::PathBuf`, adding casting variations where possible.

// === Between crate types ===

impl<From: PathCast<To>, To: PathForm> PartialEq<Path<To>> for Path<From> {
    fn eq(&self, other: &Path<To>) -> bool {
        self.inner == other.inner
    }
}

impl<Form: PathForm> Eq for Path<Form> {}

impl<From: PathCast<To>, To: PathForm> PartialOrd<Path<To>> for Path<From> {
    fn partial_cmp(&self, other: &Path<To>) -> Option<Ordering> {
        Some(self.inner.cmp(&other.inner))
    }
}

impl<Form: PathForm> Ord for Path<Form> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<Form: PathForm> Hash for Path<Form> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<From: PathCast<To>, To: PathForm> PartialEq<PathBuf<To>> for PathBuf<From> {
    fn eq(&self, other: &PathBuf<To>) -> bool {
        self.inner == other.inner
    }
}

impl<Form: PathForm> Eq for PathBuf<Form> {}

impl<From: PathCast<To>, To: PathForm> PartialOrd<PathBuf<To>> for PathBuf<From> {
    fn partial_cmp(&self, other: &PathBuf<To>) -> Option<Ordering> {
        Some(self.inner.cmp(&other.inner))
    }
}

impl<Form: PathForm> Ord for PathBuf<Form> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<Form: PathForm> Hash for PathBuf<Form> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

macro_rules! impl_cmp {
    (<$($life:lifetime),*> $lhs:ty, $rhs:ty) => {
        impl<$($life,)* Form: PathForm> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path<Form> as PartialEq>::eq(self, other)
            }
        }

        impl<$($life,)* Form: PathForm> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path<Form> as PartialEq>::eq(self, other)
            }
        }

        impl<$($life,)* Form: PathForm> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path<Form> as PartialOrd>::partial_cmp(self, other)
            }
        }

        impl<$($life,)* Form: PathForm> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path<Form> as PartialOrd>::partial_cmp(self, other)
            }
        }
    };
}

impl_cmp!(<> PathBuf<Form>, Path<Form>);
impl_cmp!(<'a> PathBuf<Form>, &'a Path<Form>);
impl_cmp!(<'a> Cow<'a, Path<Form>>, Path<Form>);
impl_cmp!(<'a, 'b> Cow<'a, Path<Form>>, &'b Path<Form>);
impl_cmp!(<'a> Cow<'a, Path<Form>>, PathBuf<Form>);

macro_rules! impl_cmp_cast {
    (<$($life:lifetime),*> $lhs:ty, $rhs:ty) => {
        impl<$($life),*> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.cast(), other.cast())
            }
        }

        impl<$($life),*> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(self.cast(), other.cast())
            }
        }

        impl<$($life),*> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.cast(), other.cast())
            }
        }

        impl<$($life),*> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.cast(), other.cast())
            }
        }
    };
}

impl_cmp_cast!(<> PathBuf, RelativePath);
impl_cmp_cast!(<> PathBuf, AbsolutePath);
impl_cmp_cast!(<> PathBuf, CanonicalPath);
impl_cmp_cast!(<> AbsolutePathBuf, CanonicalPath);
impl_cmp_cast!(<> RelativePathBuf, Path);
impl_cmp_cast!(<> AbsolutePathBuf, Path);
impl_cmp_cast!(<> CanonicalPathBuf, Path);
impl_cmp_cast!(<> CanonicalPathBuf, AbsolutePath);

impl_cmp_cast!(<'a> PathBuf, &'a RelativePath);
impl_cmp_cast!(<'a> PathBuf, &'a AbsolutePath);
impl_cmp_cast!(<'a> PathBuf, &'a CanonicalPath);
impl_cmp_cast!(<'a> AbsolutePathBuf, &'a CanonicalPath);
impl_cmp_cast!(<'a> RelativePathBuf, &'a Path);
impl_cmp_cast!(<'a> AbsolutePathBuf, &'a Path);
impl_cmp_cast!(<'a> CanonicalPathBuf, &'a Path);
impl_cmp_cast!(<'a> CanonicalPathBuf, &'a AbsolutePath);

impl_cmp_cast!(<'a> Cow<'a, Path>, RelativePath);
impl_cmp_cast!(<'a> Cow<'a, Path>, AbsolutePath);
impl_cmp_cast!(<'a> Cow<'a, Path>, CanonicalPath);
impl_cmp_cast!(<'a> Cow<'a, AbsolutePath>, CanonicalPath);
impl_cmp_cast!(<'a> Cow<'a, RelativePath>, Path);
impl_cmp_cast!(<'a> Cow<'a, AbsolutePath>, Path);
impl_cmp_cast!(<'a> Cow<'a, CanonicalPath>, Path);
impl_cmp_cast!(<'a> Cow<'a, CanonicalPath>, AbsolutePath);

impl_cmp_cast!(<'a, 'b> Cow<'a, Path>, &'b RelativePath);
impl_cmp_cast!(<'a, 'b> Cow<'a, Path>, &'b AbsolutePath);
impl_cmp_cast!(<'a, 'b> Cow<'a, Path>, &'b CanonicalPath);
impl_cmp_cast!(<'a, 'b> Cow<'a, AbsolutePath>, &'b CanonicalPath);
impl_cmp_cast!(<'a, 'b> Cow<'a, RelativePath>, &'b Path);
impl_cmp_cast!(<'a, 'b> Cow<'a, AbsolutePath>, &'b Path);
impl_cmp_cast!(<'a, 'b> Cow<'a, CanonicalPath>, &'b Path);
impl_cmp_cast!(<'a, 'b> Cow<'a, CanonicalPath>, &'b AbsolutePath);

impl_cmp_cast!(<'a> Cow<'a, Path>, RelativePathBuf);
impl_cmp_cast!(<'a> Cow<'a, Path>, AbsolutePathBuf);
impl_cmp_cast!(<'a> Cow<'a, Path>, CanonicalPathBuf);
impl_cmp_cast!(<'a> Cow<'a, AbsolutePath>, CanonicalPathBuf);
impl_cmp_cast!(<'a> Cow<'a, RelativePath>, PathBuf);
impl_cmp_cast!(<'a> Cow<'a, AbsolutePath>, PathBuf);
impl_cmp_cast!(<'a> Cow<'a, CanonicalPath>, PathBuf);
impl_cmp_cast!(<'a> Cow<'a, CanonicalPath>, AbsolutePathBuf);

// === Between std::path types ===

macro_rules! impl_cmp_std {
    (<$($life:lifetime),*> $lhs:ty, $rhs:ty) => {
        impl<$($life,)* Form: PathForm> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_any())
            }
        }

        impl<$($life,)* Form: PathForm> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(self.as_any(), other.as_ref())
            }
        }

        impl<$($life,)* Form: PathForm> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_ref(), other.as_any())
            }
        }

        impl<$($life,)* Form: PathForm> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<Ordering> {
                <Path as PartialOrd>::partial_cmp(self.as_any(), other.as_ref())
            }
        }
    };
}

impl_cmp_std!(<> std::path::Path, Path<Form>);
impl_cmp_std!(<> std::path::PathBuf, Path<Form>);
impl_cmp_std!(<'a> std::path::PathBuf, &'a Path<Form>);
impl_cmp_std!(<'a> Cow<'a, std::path::Path>, Path<Form>);
impl_cmp_std!(<'a, 'b> Cow<'a, std::path::Path>, &'b Path<Form>);

impl_cmp_std!(<> std::path::Path, PathBuf<Form>);
impl_cmp_std!(<'a> &'a std::path::Path, PathBuf<Form>);
impl_cmp_std!(<> std::path::PathBuf, PathBuf<Form>);
impl_cmp_std!(<'a> Cow<'a, std::path::Path>, PathBuf<Form>);

// === Between string types ===

impl_cmp_std!(<> OsStr, Path<Form>);
impl_cmp_std!(<'a> OsStr, &'a Path<Form>);
impl_cmp_std!(<'a> &'a OsStr, Path<Form>);
impl_cmp_std!(<'a> Cow<'a, OsStr>, Path<Form>);
impl_cmp_std!(<'a, 'b> Cow<'b, OsStr>, &'a Path<Form>);
impl_cmp_std!(<> OsString, Path<Form>);
impl_cmp_std!(<'a> OsString, &'a Path<Form>);

impl_cmp_std!(<> OsStr, PathBuf<Form>);
impl_cmp_std!(<'a> &'a OsStr, PathBuf<Form>);
impl_cmp_std!(<'a> Cow<'a, OsStr>, PathBuf<Form>);
impl_cmp_std!(<> OsString, PathBuf<Form>);
