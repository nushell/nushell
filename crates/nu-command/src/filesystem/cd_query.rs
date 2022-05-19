// Attribution:
// Thanks kn team https://github.com/micouy/kn

use alphanumeric_sort::compare_os_str;
use nu_protocol::ShellError;
use nu_protocol::Span;
use powierza_coefficient::powierża_coefficient;
use std::cmp::{Ord, Ordering};
use std::{
    convert::AsRef,
    ffi::{OsStr, OsString},
    fs::DirEntry,
    mem,
    path::{Component, Path, PathBuf},
};

/// A path matching an abbreviation.
///
/// Stores [`Congruence`](Congruence)'s of its ancestors, with that of the
/// closest ancestors first (so that it can be compared
/// [lexicographically](std::cmp::Ord#lexicographical-comparison).
struct Finding {
    file_name: OsString,
    path: PathBuf,
    congruence: Vec<Congruence>,
}

/// Returns an interator over directory's children matching the abbreviation.
fn get_matching_children<'a, P>(
    path: &'a P,
    abbr: &'a Abbr,
    parent_congruence: &'a [Congruence],
) -> impl Iterator<Item = Finding> + 'a
where
    P: AsRef<Path>,
{
    let filter_map_entry = move |entry: DirEntry| {
        let file_type = entry.file_type().ok()?;

        if file_type.is_dir() || file_type.is_symlink() {
            let file_name: String = entry.file_name().into_string().ok()?;

            if let Some(congruence) = abbr.compare(&file_name) {
                let mut entry_congruence = parent_congruence.to_vec();
                entry_congruence.insert(0, congruence);

                return Some(Finding {
                    file_name: entry.file_name(),
                    congruence: entry_congruence,
                    path: entry.path(),
                });
            }
        }

        None
    };

    path.as_ref()
        .read_dir()
        .ok()
        .map(|reader| {
            reader
                .filter_map(|entry| entry.ok())
                .filter_map(filter_map_entry)
        })
        .into_iter()
        .flatten()
}

/// The `query` subcommand.
///
/// It takes two args — `--abbr` and `--exclude` (optionally). The value of
/// `--abbr` gets split into a prefix containing components like `c:/`, `/`,
/// `~/`, and dots, and [`Abbr`](Abbr)'s. If there is more than one dir matching
/// the query, the value of `--exclude` is excluded from the search.
pub fn query<P>(arg: &P, excluded: Option<PathBuf>, span: Span) -> Result<PathBuf, ShellError>
where
    P: AsRef<Path>,
{
    // If the arg is a real path and not an abbreviation, return it. It
    // prevents potential unexpected behavior due to abbreviation expansion.
    // For example, `kn` doesn't allow for any component other than `Normal` in
    // the abbreviation but the arg itself may be a valid path. `kn` should only
    // behave differently from `cd` in situations where `cd` would fail.
    if arg.as_ref().is_dir() {
        return Ok(arg.as_ref().into());
    }

    let (prefix, abbrs) = parse_arg(&arg)?;
    let start_dir = match prefix {
        Some(start_dir) => start_dir,
        None => std::env::current_dir()?,
    };

    match abbrs.as_slice() {
        [] => Ok(start_dir),
        [first_abbr, abbrs @ ..] => {
            let mut current_level =
                get_matching_children(&start_dir, first_abbr, &[]).collect::<Vec<_>>();
            let mut next_level = vec![];

            for abbr in abbrs {
                let children = current_level.iter().flat_map(|parent| {
                    get_matching_children(&parent.path, abbr, &parent.congruence)
                });

                next_level.clear();
                next_level.extend(children);

                mem::swap(&mut next_level, &mut current_level);
            }

            let cmp_findings = |finding_a: &Finding, finding_b: &Finding| {
                finding_a
                    .congruence
                    .cmp(&finding_b.congruence)
                    .then(compare_os_str(&finding_a.file_name, &finding_b.file_name))
            };

            let found_path = match excluded {
                Some(excluded) if current_level.len() > 1 => current_level
                    .into_iter()
                    .filter(|finding| finding.path != excluded)
                    .min_by(cmp_findings)
                    .map(|Finding { path, .. }| path),
                _ => current_level
                    .into_iter()
                    .min_by(cmp_findings)
                    .map(|Finding { path, .. }| path),
            };

            found_path.ok_or(ShellError::NotADirectory(span))
        }
    }
}

/// Checks if the component contains only dots and returns the equivalent number
/// of [`ParentDir`](Component::ParentDir) components if it does.
///
/// It is the number of dots, less one. For example, `...` is converted to
/// `../..`, `....` to `../../..` etc.
fn parse_dots(component: &str) -> Option<usize> {
    component
        .chars()
        .try_fold(
            0,
            |n_dots, c| if c == '.' { Some(n_dots + 1) } else { None },
        )
        .and_then(|n_dots| if n_dots > 1 { Some(n_dots - 1) } else { None })
}

/// Extracts leading components of the path that are not parts of the
/// abbreviation.
///
/// The prefix is the path where the search starts. If there is no prefix (when
/// the path consists only of normal components), the search starts in the
/// current directory, just as you'd expect. The function collects each
/// [`Prefix`](Component::Prefix), [`RootDir`](Component::RootDir),
/// [`CurDir`](Component::CurDir), and [`ParentDir`](Component::ParentDir)
/// components and stops at the first [`Normal`](Component::Normal) component
/// **unless** it only contains dots. In this case, it converts it to as many
/// [`ParentDir`](Component::ParentDir)'s as there are dots in this component,
/// less one. For example, `...` is converted to `../..`, `....` to `../../..`
/// etc.
fn extract_prefix<'a, P>(
    arg: &'a P,
) -> Result<(Option<PathBuf>, impl Iterator<Item = Component<'a>> + 'a), ShellError>
where
    P: AsRef<Path> + ?Sized + 'a,
{
    use Component::*;

    let mut components = arg.as_ref().components().peekable();
    let mut prefix: Option<PathBuf> = None;
    let mut push_to_prefix = |component: Component| match &mut prefix {
        None => prefix = Some(PathBuf::from(&component)),
        Some(prefix) => prefix.push(component),
    };
    let parse_dots_os = |component_os: &OsStr| {
        component_os
            .to_os_string()
            .into_string()
            .map_err(|_| ShellError::NonUnicodeInput)
            .map(|component| parse_dots(&component))
    };

    while let Some(component) = components.peek() {
        match component {
            Prefix(_) | RootDir | CurDir | ParentDir => push_to_prefix(*component),
            Normal(component_os) => {
                if let Some(n_dots) = parse_dots_os(component_os)? {
                    (0..n_dots).for_each(|_| push_to_prefix(ParentDir));
                } else {
                    break;
                }
            }
        }

        let _consumed = components.next();
    }

    Ok((prefix, components))
}

/// Converts each component into [`Abbr`](Abbr) without checking
/// the component's type.
///
/// This may change in the future.
fn parse_abbrs<'a, I>(components: I) -> Result<Vec<Abbr>, ShellError>
where
    I: Iterator<Item = Component<'a>> + 'a,
{
    use Component::*;

    let abbrs = components
        .into_iter()
        .map(|component| match component {
            Prefix(_) | RootDir | CurDir | ParentDir => {
                let component_string = component
                    .as_os_str()
                    .to_os_string()
                    .to_string_lossy()
                    .to_string();

                Err(ShellError::UnexpectedAbbrComponent(component_string))
            }
            Normal(component_os) => component_os
                .to_os_string()
                .into_string()
                .map_err(|_| ShellError::NonUnicodeInput)
                .map(|string| Abbr::new_sanitized(&string)),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(abbrs)
}

/// Parses the provided argument into a prefix and [`Abbr`](Abbr)'s.
fn parse_arg<P>(arg: &P) -> Result<(Option<PathBuf>, Vec<Abbr>), ShellError>
where
    P: AsRef<Path>,
{
    let (prefix, suffix) = extract_prefix(arg)?;
    let abbrs = parse_abbrs(suffix)?;

    Ok((prefix, abbrs))
}

#[cfg(test)]
mod test {
    use super::*;

    //     // #[cfg(any(test, doc))]
    //     // #[macro_export]
    //     // macro_rules! assert_variant {
    //     //     ($expression_in:expr , $( pat )|+ $( if $guard: expr )? $( => $expression_out:expr )? ) => {
    //     //         match $expression_in {
    //     //             $( $pattern )|+ $( if $guard )? => { $( $expression_out )? },
    //     //             variant => panic!("{:?}", variant),
    //     //         }
    //     //     };

    //     //     ($expression_in:expr , $( pat )|+ $( if $guard: expr )? $( => $expression_out:expr)? , $panic:expr) => {
    //     //         match $expression_in {
    //     //             $( $pattern )|+ $( if $guard )? => { $( $expression_out )? },
    //     //             _ => panic!($panic),
    //     //         }
    //     //     };
    //     // }

    //     /// Asserts that the expression matches the variant. Optionally returns a value.
    //     ///
    //     /// Inspired by [`std::matches`](https://doc.rust-lang.org/stable/std/macro.matches.html).
    //     ///
    //     /// # Examples
    //     ///
    //     /// ```
    //     /// # fn main() -> Option<()> {
    //     /// use kn::Congruence::*;
    //     ///
    //     /// let abbr = Abbr::new_sanitized("abcjkl");
    //     /// let coeff_1 = assert_variant!(abbr.compare("abc_jkl"), Some(Subsequence(coeff)) => coeff);
    //     /// let coeff_2 = assert_variant!(abbr.compare("ab_cj_kl"), Some(Subsequence(coeff)) => coeff);
    //     /// assert!(coeff_1 < coeff_2);
    //     /// # Ok(())
    //     /// # }
    //     /// ```
    //     #[cfg(any(test, doc))]
    //     #[macro_export]
    //     macro_rules! assert_variant {
    //     ($expression_in:expr , $( $pattern:pat )+ $( if $guard: expr )? $( => $expression_out:expr )? ) => {
    //         match $expression_in {
    //             $( $pattern )|+ $( if $guard )? => { $( $expression_out )? },
    //             variant => panic!("{:?}", variant),
    //         }
    //     };

    //     ($expression_in:expr , $( $pattern:pat )+ $( if $guard: expr )? $( => $expression_out:expr)? , $panic:expr) => {
    //         match $expression_in {
    //             $( $pattern )|+ $( if $guard )? => { $( $expression_out )? },
    //             _ => panic!($panic),
    //         }
    //     };
    // }

    //     #[test]
    //     fn test_parse_dots() {
    //         assert_variant!(parse_dots(""), None);
    //         assert_variant!(parse_dots("."), None);
    //         assert_variant!(parse_dots(".."), Some(1));
    //         assert_variant!(parse_dots("..."), Some(2));
    //         assert_variant!(parse_dots("...."), Some(3));
    //         assert_variant!(parse_dots("xyz"), None);
    //         assert_variant!(parse_dots("...dot"), None);
    //     }

    #[test]
    fn test_extract_prefix() {
        {
            let (prefix, suffix) = extract_prefix("suf/fix").unwrap();
            let suffix = suffix.collect::<PathBuf>();

            assert_eq!(prefix, None);
            assert_eq!(as_path(&suffix), as_path("suf/fix"));
        }

        {
            let (prefix, suffix) = extract_prefix("./.././suf/fix").unwrap();
            let suffix = suffix.collect::<PathBuf>();

            assert_eq!(prefix.unwrap(), as_path("./.."));
            assert_eq!(as_path(&suffix), as_path("suf/fix"));
        }

        {
            let (prefix, suffix) = extract_prefix(".../.../suf/fix").unwrap();
            let suffix = suffix.collect::<PathBuf>();

            assert_eq!(prefix.unwrap(), as_path("../../../.."));
            assert_eq!(as_path(&suffix), as_path("suf/fix"));
        }
    }

    #[test]
    fn test_parse_arg_invalid_unicode() {
        #[cfg(unix)]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;

            let source = [0x66, 0x6f, 0x80, 0x6f];
            let non_unicode_input = OsStr::from_bytes(&source[..]).to_os_string();
            let result = parse_arg(&non_unicode_input);

            assert!(result.is_err());
        }

        #[cfg(windows)]
        {
            use std::os::windows::prelude::*;

            let source = [0x0066, 0x006f, 0xd800, 0x006f];
            let os_string = OsString::from_wide(&source[..]);
            let result = parse_arg(&os_string);

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_congruence_ordering() {
        assert!(Complete < Prefix);
        assert!(Complete < Subsequence(1));
        assert!(Prefix < Subsequence(1));
        assert!(Subsequence(1) < Subsequence(1000));
    }

    //     #[test]
    //     fn test_compare_abbr() {
    //         let abbr = Abbr::new_sanitized("abcjkl");

    //         assert_variant!(abbr.compare("abcjkl"), Some(Complete));
    //         assert_variant!(abbr.compare("abcjkl_"), Some(Prefix));
    //         assert_variant!(abbr.compare("_abcjkl"), Some(Subsequence(0)));
    //         assert_variant!(abbr.compare("abc_jkl"), Some(Subsequence(1)));

    //         assert_variant!(abbr.compare("xyz"), None);
    //         assert_variant!(abbr.compare(""), None);
    //     }

    //     #[test]
    //     fn test_compare_abbr_different_cases() {
    //         let abbr = Abbr::new_sanitized("AbCjKl");

    //         assert_variant!(abbr.compare("aBcJkL"), Some(Complete));
    //         assert_variant!(abbr.compare("AbcJkl_"), Some(Prefix));
    //         assert_variant!(abbr.compare("_aBcjKl"), Some(Subsequence(0)));
    //         assert_variant!(abbr.compare("abC_jkL"), Some(Subsequence(1)));
    //     }

    //     #[test]
    //     fn test_empty_abbr_empty_component() {
    //         let empty = "";

    //         let abbr = Abbr::new_sanitized(empty);
    //         assert_variant!(abbr.compare("non empty component"), None);

    //         let abbr = Abbr::new_sanitized("non empty abbr");
    //         assert_variant!(abbr.compare(empty), None);
    //     }

    #[test]
    fn test_order_paths() {
        fn sort<'a>(paths: &'a [&'a str], abbr: &str) -> Vec<&'a str> {
            let abbr = Abbr::new_sanitized(abbr);
            let mut paths = paths.to_owned();
            paths.sort_by_key(|path| abbr.compare(path).unwrap());

            paths
        }

        let paths = vec!["playground", "plotka"];
        assert_eq!(paths, sort(&paths, "pla"));

        let paths = vec!["veccentric", "vehiccles"];
        assert_eq!(paths, sort(&paths, "vecc"));
    }
}

/// Shorthand for `AsRef<Path>::as_ref(&x)`.
#[cfg(any(test, doc))]
pub fn as_path<P>(path: &P) -> &Path
where
    P: AsRef<Path> + ?Sized,
{
    path.as_ref()
}

/// A component of the user's query.
///
/// It is used in comparing and ordering of found paths. Read more in
/// [`Congruence`'s docs](Congruence).
#[derive(Debug, Clone)]
pub enum Abbr {
    /// Wildcard matches every component with congruence
    /// [`Complete`](Congruence::Complete).
    Wildcard,

    /// Literal abbreviation.
    Literal(String),
}

impl Abbr {
    /// Constructs [`Abbr::Wildcard`](Abbr::Wildcard) if the
    /// string slice is '-', otherwise constructs
    /// wrapped [`Abbr::Literal`](Abbr::Literal) with the abbreviation
    /// mapped to its ASCII lowercase equivalent.
    pub fn new_sanitized(abbr: &str) -> Self {
        if abbr == "-" {
            Self::Wildcard
        } else {
            Self::Literal(abbr.to_ascii_lowercase())
        }
    }

    /// Compares a component against the abbreviation.
    pub fn compare(&self, component: &str) -> Option<Congruence> {
        // What about characters with accents? [https://eev.ee/blog/2015/09/12/dark-corners-of-unicode/]
        let component = component.to_ascii_lowercase();

        match self {
            Self::Wildcard => Some(Congruence::Complete),
            Self::Literal(literal) => {
                if literal.is_empty() || component.is_empty() {
                    None
                } else if *literal == component {
                    Some(Congruence::Complete)
                } else if component.starts_with(literal) {
                    Some(Congruence::Prefix)
                } else {
                    powierża_coefficient(literal, &component).map(Congruence::Subsequence)
                }
            }
        }
    }
}

/// The strength of the match between an abbreviation and a component.
///
/// [`Congruence`](Congruence) is used to order path components in the following
/// way:
///
/// 1. Components are first ordered based on how well they match the
/// abbreviation — first [`Complete`](Congruence::Complete), then
/// [`Prefix`](Congruence::Prefix), then
/// [`Subsequence`](Congruence::Subsequence).
/// 2. Components with congruence [`Subsequence`](Congruence::Subsequence) are
/// ordered by their [Powierża coefficient](https://github.com/micouy/powierza-coefficient).
/// 3. If the order of two components cannot be determined based on the above, [`alphanumeric_sort`](https://docs.rs/alphanumeric-sort) is used.
///
/// Below are the results of matching components against abbreviation `abc`:
///
/// | Component   | Match strength                           |
/// |-------------|------------------------------------------|
/// | `abc`       | [`Complete`](Congruence::Complete)       |
/// | `abc___`    | [`Prefix`](Congruence::Prefix)           |
/// | `_a_b_c_`   | [`Subsequence`](Congruence::Subsequence) |
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Congruence {
    /// Either the abbreviation and the component are the same or the
    /// abbreviation is a wildcard.
    Complete,

    /// The abbreviation is a prefix of the component.
    Prefix,

    /// The abbreviation's characters form a subsequence of the component's
    /// characters. The field contains the Powierża coefficient of the pair of
    /// strings.
    Subsequence(u32),
}

use Congruence::*;

impl PartialOrd for Congruence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Congruence {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;

        match (self, other) {
            (Complete, Complete) => Equal,
            (Complete, Prefix) => Less,
            (Complete, Subsequence(_)) => Less,

            (Prefix, Complete) => Greater,
            (Prefix, Prefix) => Equal,
            (Prefix, Subsequence(_)) => Less,

            (Subsequence(_), Complete) => Greater,
            (Subsequence(_), Prefix) => Greater,
            (Subsequence(dist_a), Subsequence(dist_b)) => dist_a.cmp(dist_b),
        }
    }
}
