use crate::form::{
    Absolute, Any, Canonical, IsAbsolute, MaybeAbsolute, MaybeRelative, PathCast, PathForm,
    PathJoin, PathMut, PathPush, Relative,
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

#[repr(transparent)]
pub struct Path<Form: PathForm = Any> {
    _form: PhantomData<Form>,
    inner: std::path::Path,
}

pub type RelativePath = Path<Relative>;

pub type AbsolutePath = Path<Absolute>;

pub type CanonicalPath = Path<Canonical>;

impl<Form: PathForm> Path<Form> {
    #[inline]
    fn new_unchecked<P: AsRef<OsStr> + ?Sized>(path: &P) -> &Self {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let path = std::path::Path::new(path.as_ref());
        let ptr = std::ptr::from_ref(path) as *const Self;
        unsafe { &*ptr }
    }

    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        self.inner.as_os_str()
    }

    #[inline]
    pub fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    #[inline]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.inner.to_string_lossy()
    }

    #[inline]
    pub fn to_path_buf(&self) -> PathBuf<Form> {
        PathBuf::new_unchecked(self.inner.to_path_buf())
    }

    #[inline]
    pub fn parent(&self) -> Option<&Self> {
        self.inner.parent().map(Self::new_unchecked)
    }

    #[inline]
    pub fn ancestors(&self) -> std::path::Ancestors<'_> {
        self.inner.ancestors()
    }

    #[inline]
    pub fn file_name(&self) -> Option<&OsStr> {
        self.inner.file_name()
    }

    #[inline]
    pub fn starts_with<F: PathForm>(&self, base: impl AsRef<Path<F>>) -> bool {
        self.inner.starts_with(&base.as_ref().inner)
    }

    #[inline]
    pub fn ends_with<F: PathForm>(&self, child: impl AsRef<Path<F>>) -> bool {
        self.inner.ends_with(&child.as_ref().inner)
    }

    #[inline]
    pub fn file_stem(&self) -> Option<&OsStr> {
        self.inner.file_stem()
    }

    #[inline]
    pub fn extension(&self) -> Option<&OsStr> {
        self.inner.extension()
    }

    #[inline]
    pub fn components(&self) -> std::path::Components<'_> {
        self.inner.components()
    }

    #[inline]
    pub fn iter(&self) -> std::path::Iter<'_> {
        self.inner.iter()
    }

    #[inline]
    pub fn display(&self) -> std::path::Display<'_> {
        self.inner.display()
    }

    #[inline]
    pub fn into_path_buf(self: Box<Self>) -> PathBuf<Form> {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let ptr = Box::into_raw(self) as *mut std::path::Path;
        let boxed = unsafe { Box::from_raw(ptr) };
        PathBuf::new_unchecked(boxed.into_path_buf())
    }

    #[inline]
    pub fn cast<To>(&self) -> &Path<To>
    where
        To: PathForm,
        Form: PathCast<To>,
    {
        Path::new_unchecked(self)
    }

    #[inline]
    pub fn as_any(&self) -> &Path {
        Path::new_unchecked(self)
    }
}

impl Path {
    #[inline]
    pub fn new<P: AsRef<OsStr> + ?Sized>(path: &P) -> &Self {
        Self::new_unchecked(path)
    }

    #[inline]
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    #[inline]
    pub fn is_relative(&self) -> bool {
        self.inner.is_relative()
    }

    #[inline]
    pub fn try_absolute(&self) -> Result<&AbsolutePath, &RelativePath> {
        self.is_absolute()
            .then_some(AbsolutePath::new_unchecked(&self.inner))
            .ok_or(RelativePath::new_unchecked(&self.inner))
    }

    #[inline]
    pub fn try_relative(&self) -> Result<&RelativePath, &AbsolutePath> {
        self.is_relative()
            .then_some(RelativePath::new_unchecked(&self.inner))
            .ok_or(AbsolutePath::new_unchecked(&self.inner))
    }

    #[inline]
    pub fn strip_prefix<Form: PathForm>(
        &self,
        base: impl AsRef<Path<Form>>,
    ) -> Result<&RelativePath, StripPrefixError> {
        self.inner
            .strip_prefix(&base.as_ref().inner)
            .map(RelativePath::new_unchecked)
    }
}

impl<Form: PathJoin> Path<Form> {
    #[inline]
    pub fn join<F: MaybeRelative>(&self, path: impl AsRef<Path<F>>) -> PathBuf<Form::Output> {
        PathBuf::new_unchecked(self.inner.join(&path.as_ref().inner))
    }
}

impl<Form: PathMut> Path<Form> {
    #[inline]
    pub fn as_mut_os_str(&mut self) -> &mut OsStr {
        self.inner.as_mut_os_str()
    }

    #[inline]
    pub fn with_file_name(&self, file_name: impl AsRef<OsStr>) -> PathBuf<Form> {
        PathBuf::new_unchecked(self.inner.with_file_name(file_name))
    }

    #[inline]
    pub fn with_extension(&self, extension: impl AsRef<OsStr>) -> PathBuf<Form> {
        PathBuf::new_unchecked(self.inner.with_extension(extension))
    }
}

impl<Form: MaybeRelative> Path<Form> {
    #[inline]
    pub fn as_relative_std_path(&self) -> &std::path::Path {
        &self.inner
    }
}

impl<Form: MaybeAbsolute> Path<Form> {
    #[inline]
    pub fn has_root(&self) -> bool {
        self.inner.has_root()
    }
}

impl<Form: IsAbsolute> Path<Form> {
    #[inline]
    pub fn as_std_path(&self) -> &std::path::Path {
        &self.inner
    }

    #[inline]
    pub fn to_std_pathbuf(&self) -> std::path::PathBuf {
        self.inner.to_path_buf()
    }

    #[inline]
    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        self.inner.metadata()
    }

    #[inline]
    pub fn read_dir(&self) -> io::Result<fs::ReadDir> {
        self.inner.read_dir()
    }

    #[inline]
    pub fn exists(&self) -> bool {
        self.inner.exists()
    }

    #[inline]
    pub fn is_file(&self) -> bool {
        self.inner.is_file()
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.inner.is_dir()
    }
}

impl AbsolutePath {
    #[cfg(not(windows))]
    #[inline]
    pub fn canonicalize(&self) -> io::Result<CanonicalPathBuf> {
        self.inner
            .canonicalize()
            .map(CanonicalPathBuf::new_unchecked)
    }

    #[cfg(windows)]
    pub fn canonicalize(&self) -> io::Result<CanonicalPathBuf> {
        use omnipath::WinPathExt;

        let path = self.inner.canonicalize()?.to_winuser_path()?;
        Ok(CanonicalPathBuf::new_unchecked(path))
    }

    #[inline]
    pub fn read_link(&self) -> io::Result<AbsolutePathBuf> {
        self.inner.read_link().map(PathBuf::new_unchecked)
    }

    #[inline]
    pub fn try_exists(&self) -> io::Result<bool> {
        self.inner.try_exists()
    }

    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.inner.is_symlink()
    }

    #[inline]
    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        self.inner.symlink_metadata()
    }
}

impl CanonicalPath {
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

impl<Form: PathMut> PathBuf<Form> {
    #[inline]
    pub fn as_mut_os_string(&mut self) -> &mut OsString {
        self.inner.as_mut_os_string()
    }

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

impl<Form: PathMut> DerefMut for PathBuf<Form> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: `Path<Form>` is a repr(transparent) wrapper around `std::path::Path`.
        let path: &mut std::path::Path = &mut self.inner;
        let ptr = std::ptr::from_mut(path) as *mut Path<Form>;
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
