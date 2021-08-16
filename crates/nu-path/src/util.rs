use std::borrow::Cow;
use std::path::{Path, PathBuf};

// Utility for applying a function that can only be called on the borrowed type of the Cow
// and also returns a ref. If the Cow is a borrow, we can return the same borrow but an
// owned value needs extra handling because the returned valued has to be owned as well
pub fn cow_map_by_ref<B, O, F>(c: Cow<'_, B>, f: F) -> Cow<'_, B>
where
    B: ToOwned<Owned = O> + ?Sized,
    O: AsRef<B>,
    F: FnOnce(&B) -> &B,
{
    match c {
        Cow::Borrowed(b) => Cow::Borrowed(f(b)),
        Cow::Owned(o) => Cow::Owned(f(o.as_ref()).to_owned()),
    }
}

// Utility for applying a function over Cow<'a, Path> over a Cow<'a, str> while avoiding unnecessary conversions
pub fn cow_map_str_path<'a, F>(c: Cow<'a, str>, f: F) -> Cow<'a, str>
where
    F: FnOnce(Cow<'a, Path>) -> Cow<'a, Path>,
{
    let ret = match c {
        Cow::Borrowed(b) => f(Cow::Borrowed(Path::new(b))),
        Cow::Owned(o) => f(Cow::Owned(PathBuf::from(o))),
    };

    match ret {
        Cow::Borrowed(expanded) => expanded.to_string_lossy(),
        Cow::Owned(expanded) => Cow::Owned(expanded.to_string_lossy().to_string()),
    }
}

// Utility for applying a function over Cow<'a, str> over a Cow<'a, Path> while avoiding unnecessary conversions
pub fn cow_map_path_str<'a, F>(c: Cow<'a, Path>, f: F) -> Cow<'a, Path>
where
    F: FnOnce(Cow<'a, str>) -> Cow<'a, str>,
{
    let ret = match c {
        Cow::Borrowed(path) => f(path.to_string_lossy()),
        Cow::Owned(buf) => f(Cow::Owned(buf.to_string_lossy().to_string())),
    };

    match ret {
        Cow::Borrowed(expanded) => Cow::Borrowed(Path::new(expanded)),
        Cow::Owned(expanded) => Cow::Owned(PathBuf::from(expanded)),
    }
}

pub fn trim_trailing_slash(s: &str) -> &str {
    s.trim_end_matches(std::path::is_separator)
}
