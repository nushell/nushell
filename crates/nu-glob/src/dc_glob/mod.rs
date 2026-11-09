//! Experimental dc-glob backend imported from glob_experiment by Devyn Cairns for evaluation in Nushell.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::Pattern;

mod compiler;
mod globber;
mod matcher;
mod parser;

const GLOB_CHARS: &[char] = &['*', '?', '['];

/// Options for dc-glob filesystem traversal.
///
/// Patterns follow nu-glob's rules, with `{a,b}` alternatives added: a trailing `**` matches the
/// directories below its prefix but not the prefix itself, and `\` is a literal character.
#[derive(Debug, Clone, Default)]
pub struct GlobWalkOptions {
    /// Maximum recursion depth. `None` means unbounded.
    pub max_depth: Option<usize>,
    /// Follow directory symlinks while traversing.
    pub follow_symlinks: bool,
    /// Exclusion glob patterns used to filter and prune traversal. They match the way
    /// [`DcPattern`] does: nu-glob's rules, `{a,b}` included.
    pub excludes: Vec<String>,
    /// Optional interrupt flag. When set to `true` the traversal stops as soon
    /// as possible and the iterator ends.
    pub interrupt: Option<Arc<AtomicBool>>,
    /// When `true`, match path components case-insensitively (ASCII only).
    pub ignore_case: bool,
    /// When `true`, `*`, `?` and `[...]` don't match a `.` at the start of a name, so a hidden
    /// name matches only where the pattern spells its dot out. Mirrors
    /// [`MatchOptions::require_literal_leading_dot`](crate::MatchOptions).
    pub require_literal_leading_dot: bool,
    /// When `true`, `**` drops a directory whose name starts with `.`: it neither descends into
    /// it nor lets the rest of the pattern match it. Mirrors
    /// [`MatchOptions::recursive_match_hidden_dir`](crate::MatchOptions) set to `false`.
    pub skip_hidden_in_recursion: bool,
}

impl GlobWalkOptions {
    /// Options that give the results nu-glob gives for `options`: directory symlinks are
    /// followed, and case and hidden names are handled the way `options` asks.
    pub fn nu_glob(options: crate::MatchOptions) -> Self {
        GlobWalkOptions {
            follow_symlinks: true,
            ignore_case: !options.case_sensitive,
            require_literal_leading_dot: options.require_literal_leading_dot,
            skip_hidden_in_recursion: !options.recursive_match_hidden_dir,
            ..GlobWalkOptions::default()
        }
    }
}

/// Return true if the given pattern contains glob metacharacters.
///
/// Braces don't count: a word like `HEAD@{1}` or `{a,b}` without `*`, `?` or `[` is not
/// expanded, so it reaches commands and external programs unchanged.
pub fn is_glob(pattern: &str) -> bool {
    pattern.contains(GLOB_CHARS)
}

/// Return true if a path component of a glob has pattern syntax, so the walk has to match it
/// rather than start below it.
///
/// Unlike [`is_glob`], `{` counts, so `{src,tests}` in `{src,tests}/**/*.rs` is matched as
/// alternatives rather than taken as a directory name.
pub fn is_glob_component(component: &str) -> bool {
    is_glob(component) || component.contains('{')
}

/// Escape glob metacharacters, braces included, so a pattern is matched literally.
pub fn escape(pattern: &str) -> String {
    let mut escaped = String::with_capacity(pattern.len());
    for ch in pattern.chars() {
        if matches!(ch, '?' | '*' | '[' | ']' | '{' | '}') {
            escaped.push('[');
            escaped.push(ch);
            escaped.push(']');
        } else {
            escaped.push(ch);
        }
    }
    escaped
}

/// Reject the patterns nu-glob rejects, such as `***`, `a**` or an unclosed `[`.
///
/// nu-glob parses the pattern both as a whole and one path component at a time, so `a/[b/c]` is
/// rejected too. Braces are literal to nu-glob, so `**` can't be part of `{...}`.
fn check_nu_glob_syntax(pattern: &str) -> anyhow::Result<()> {
    crate::Pattern::new(pattern)?;
    for component in pattern.split(std::path::is_separator) {
        crate::Pattern::new(component)?;
    }
    Ok(())
}

/// Expand `pattern` relative to `relative_to` using the experimental dc-glob backend, with the
/// results nu-glob's `glob` gives: directory symlinks are followed.
pub fn glob_from(
    relative_to: impl AsRef<Path>,
    pattern: impl AsRef<str>,
) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<PathBuf>> + Send>> {
    glob_with(
        relative_to,
        pattern,
        &GlobWalkOptions::nu_glob(crate::MatchOptions::default()),
    )
}

/// Expand `pattern` relative to `relative_to` using dc-glob with traversal options.
pub fn glob_with(
    relative_to: impl AsRef<Path>,
    pattern: impl AsRef<str>,
    options: &GlobWalkOptions,
) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<PathBuf>> + Send>> {
    let pattern = pattern.as_ref().to_owned();
    // A mistyped `rm ***` fails rather than expanding.
    check_nu_glob_syntax(&pattern)?;
    let parsed = parser::parse(&pattern);
    // The fast path compares only the file name, which can't tell whether a leading `.` came
    // from a literal in the pattern, so the matcher handles those patterns.
    let fast_path = if options.require_literal_leading_dot {
        None
    } else {
        detect_recursive_suffix_fast_path(&parsed)
    };
    let mut include_program = compiler::compile_with_options(&parsed, options.ignore_case)?;
    // As with nu-glob, a pattern that ends with a separator, like `*/`, matches only
    // directories. Parsing drops that separator, so it is checked here.
    include_program.dirs_only |= pattern.ends_with(std::path::is_separator);
    include_program.literal_leading_dot = options.require_literal_leading_dot;
    include_program.skip_hidden_in_recursion = options.skip_hidden_in_recursion;

    let exclude_programs = options
        .excludes
        .iter()
        .map(|exclude| DcPattern::new(exclude))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(Box::new(globber::glob_with_options(
        relative_to.as_ref().to_path_buf(),
        Arc::new(include_program),
        globber::TraversalOptions {
            max_depth: options.max_depth,
            follow_symlinks: options.follow_symlinks,
        },
        exclude_programs,
        globber::InterruptFlag(options.interrupt.clone()),
        fast_path,
    )))
}

fn detect_recursive_suffix_fast_path(
    pattern: &parser::Pattern,
) -> Option<globber::RecursiveFastPath> {
    use parser::AstNode;

    let nodes = pattern.nodes.as_slice();
    if nodes.len() < 3 {
        return None;
    }

    let recurse_index = nodes
        .windows(2)
        .position(|window| matches!(window, [AstNode::Recurse, AstNode::Separator]))?;

    let tail = &nodes[(recurse_index + 2)..];
    if tail.is_empty() {
        return None;
    }

    let static_prefix_only = nodes[..recurse_index].iter().all(|node| {
        matches!(
            node,
            AstNode::Prefix(_)
                | AstNode::RootDir
                | AstNode::CurDir
                | AstNode::ParentDir
                | AstNode::LiteralString(_)
                | AstNode::Separator
        )
    });

    if !static_prefix_only || tail.iter().any(|node| matches!(node, AstNode::Separator)) {
        return None;
    }

    if let [AstNode::Wildcard] = tail {
        return Some(globber::RecursiveFastPath::Suffix(Box::<[u8]>::default()));
    }

    if let [AstNode::Wildcard, AstNode::LiteralString(bytes)] = tail {
        return Some(globber::RecursiveFastPath::Suffix(
            bytes.clone().into_boxed_slice(),
        ));
    }

    let tokens = tail
        .iter()
        .map(|node| match node {
            AstNode::LiteralString(bytes) => Some(globber::BasenameToken::Literal(
                bytes.clone().into_boxed_slice(),
            )),
            AstNode::Wildcard => Some(globber::BasenameToken::Wildcard),
            AstNode::AnyCharacter => Some(globber::BasenameToken::AnyCharacter),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;

    Some(globber::RecursiveFastPath::BasenamePattern(
        tokens.into_boxed_slice(),
    ))
}

/// Return a formatted AST dump for a glob pattern.
pub fn debug_parse(pattern: impl AsRef<str>) -> String {
    let parsed = parser::parse(pattern.as_ref());
    format!("{parsed:#?}")
}

/// Return a formatted compiled-program dump for a glob pattern.
pub fn debug_compile(pattern: impl AsRef<str>) -> anyhow::Result<String> {
    let parsed = parser::parse(pattern.as_ref());
    let compiled = compiler::compile(&parsed)?;
    Ok(format!("{compiled:#?}"))
}

/// Return whether `path` matches `pattern` as a complete dc-glob match.
pub fn debug_matches(pattern: impl AsRef<str>, path: impl AsRef<Path>) -> anyhow::Result<bool> {
    let parsed = parser::parse(pattern.as_ref());
    let compiled = compiler::compile(&parsed)?;
    Ok(matcher::path_matches(path.as_ref(), false, &compiled).valid_as_complete_match)
}

/// A glob pattern for matching paths without filesystem traversal, as `du --exclude`,
/// `watch --glob` and dc-glob's excludes do.
///
/// It matches the way nu-glob's [`Pattern`] does, with `{a,b}` alternatives added: each
/// combination of choices is a nu-glob pattern, and a path matches when one of them does. So as
/// with nu-glob, `*` also matches `/` here, and `target/` doesn't match `target`.
#[derive(Clone)]
pub struct DcPattern {
    pattern: String,
    choices: Vec<Pattern>,
    options: crate::MatchOptions,
}

impl std::fmt::Debug for DcPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DcPattern")
            .field("pattern", &self.pattern)
            .finish()
    }
}

impl DcPattern {
    /// Compile a glob pattern. Returns an error if the pattern is invalid, as nu-glob's
    /// [`Pattern::new`] does.
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
        Self::compile(pattern, false)
    }

    /// Compile a glob pattern with case-insensitive matching (ASCII only).
    pub fn with_ignore_case(pattern: &str) -> anyhow::Result<Self> {
        Self::compile(pattern, true)
    }

    fn compile(pattern: &str, case_insensitive: bool) -> anyhow::Result<Self> {
        // Braces are literal to nu-glob, so this also rejects `**` inside `{...}`.
        Pattern::new(pattern)?;
        let choices = parser::expand_alternatives(pattern)?
            .iter()
            .map(|choice| Pattern::new(choice))
            .collect::<Result<_, _>>()?;
        Ok(DcPattern {
            pattern: pattern.to_owned(),
            choices,
            options: crate::MatchOptions {
                case_sensitive: !case_insensitive,
                ..crate::MatchOptions::default()
            },
        })
    }

    /// Return `true` if the given path matches this pattern.
    pub fn matches_path(&self, path: &Path) -> bool {
        self.matches_path_with(path, self.options)
    }

    /// Return `true` if the given path matches this pattern with `options`, as
    /// [`Pattern::matches_path_with`] does.
    pub fn matches_path_with(&self, path: &Path, options: crate::MatchOptions) -> bool {
        self.choices
            .iter()
            .any(|choice| choice.matches_path_with(path, options))
    }
}

#[cfg(test)]
mod tests {
    use super::compiler::{Instruction, Program};
    use super::matcher::path_matches;
    use super::parser::{AstNode, CharacterClass};
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn expected_path(parts: &[&str]) -> String {
        parts.join(std::path::MAIN_SEPARATOR_STR)
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "nu_dc_glob_{prefix}_{}_{}",
            std::process::id(),
            ts + u128::from(NEXT_ID.fetch_add(1, Ordering::Relaxed))
        ))
    }

    fn write_file(path: &Path) {
        fs::create_dir_all(path.parent().expect("file path must have parent"))
            .expect("failed to create parent directory");
        fs::write(path, b"x").expect("failed to write test file");
    }

    fn collect_ok_paths(
        iter: impl Iterator<Item = anyhow::Result<PathBuf>>,
    ) -> anyhow::Result<Vec<String>> {
        let mut out = Vec::new();
        for item in iter {
            let path_str = item?.to_string_lossy().into_owned();
            // Normalize path separators to native platform style
            #[cfg(windows)]
            let normalized = path_str.replace('/', "\\");
            #[cfg(not(windows))]
            let normalized = path_str.replace('\\', "/");
            out.push(normalized);
        }
        out.sort();
        Ok(out)
    }

    #[test]
    fn negated_character_classes_exclude_members() {
        for (pattern, path, expected) in [
            ("[!f]*", "foo", false),
            ("[!f]*", "bar", true),
            ("[!a-c]", "a", false),
            ("[!a-c]", "b", false),
            ("[!a-c]", "c", false),
            ("[!a-c]", "d", true),
            ("[!é]", "é", false),
            ("[!é]", "日", true),
            ("[!α-γ]", "β", false),
            ("[!α-γ]", "δ", true),
            ("[!!]", "!", false),
            ("[!!]", "f", true),
            ("[!f]", "/", false),
            ("[!f]*", "dir/file", false),
            ("[^f]", "^", true),
            ("[^f]", "f", true),
            ("[!f]", "", false),
            ("[!f]", "ab", false),
            ("[f!]", "!", true),
            ("[f!]", "f", true),
            ("[f!]", "b", false),
        ] {
            assert_eq!(
                debug_matches(pattern, path).unwrap(),
                expected,
                "{pattern}: {path}"
            );
        }
        let pattern = DcPattern::with_ignore_case("[!a-c]").unwrap();
        assert!(!pattern.matches_path(Path::new("B")));
        assert!(pattern.matches_path(Path::new("D")));
    }

    #[cfg(unix)]
    #[test]
    fn negated_character_classes_match_undecodable_bytes() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        for (bytes, pattern, expected) in [
            (&b"\xffoo"[..], "?oo", true),
            (&b"\xffoo"[..], "[!f]oo", true),
            (&b"\xffoo"[..], "[!f]*", true),
            (&b"\xffoo"[..], "[f]*", false),
            (&b"a\xffoo"[..], "a[!f]oo", true),
            (&b"\xff\xfeoo"[..], "[!f][!f]oo", true),
            (&b"\xff\xfeoo"[..], "[!f]oo", false),
            (&b"foo"[..], "[!f]oo", false),
            (&b"oo"[..], "[!f]oo", false),
        ] {
            assert_eq!(
                debug_matches(pattern, Path::new(OsStr::from_bytes(bytes))).unwrap(),
                expected,
                "pattern {pattern:?}, path {bytes:?}"
            );
        }
    }

    #[test]
    fn glob_with_ignore_case_matches_literal_file_with_different_case() {
        let root = unique_test_dir("ignore_case_literal");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("README.md"));

        let options = GlobWalkOptions {
            ignore_case: true,
            ..GlobWalkOptions::default()
        };
        let paths = collect_ok_paths(
            glob_with(&root, "readme.md", &options).expect("ignore-case glob should compile"),
        )
        .expect("ignore-case glob should traverse");
        assert_eq!(paths, vec!["README.md"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_ignore_case_resolves_literal_directory_prefix() {
        let root = unique_test_dir("ignore_case_prefix");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("docs/note.md"));

        let options = GlobWalkOptions {
            ignore_case: true,
            ..GlobWalkOptions::default()
        };
        let paths = collect_ok_paths(
            glob_with(&root, "Docs/*.md", &options).expect("ignore-case glob should compile"),
        )
        .expect("ignore-case glob should traverse");
        assert_eq!(paths, vec![expected_path(&["docs", "note.md"])]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_ignore_case_keeps_parent_after_literal() {
        let root = unique_test_dir("ignore_case_parent");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("docs/note.md"));
        write_file(&root.join("docs/deep/x.md"));

        // A `..` after a literal component has to be resolved by the
        // filesystem. With `-i` the literal used to be left to the matcher, so
        // the traversal started above the `..` and never reached the parent:
        // the pattern matched nothing.
        for ignore_case in [false, true] {
            let options = GlobWalkOptions {
                ignore_case,
                ..GlobWalkOptions::default()
            };
            for (pattern, expected) in [
                (
                    "docs/../docs/*.md",
                    expected_path(&["docs", "..", "docs", "note.md"]),
                ),
                (
                    "docs/deep/../*.md",
                    expected_path(&["docs", "deep", "..", "note.md"]),
                ),
            ] {
                let paths = collect_ok_paths(
                    glob_with(&root, pattern, &options).expect("parent glob should compile"),
                )
                .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
                assert_eq!(
                    paths,
                    vec![expected],
                    "ignore_case={ignore_case}, pattern {pattern}"
                );
            }
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_missing_root_returns_no_results() {
        let root = unique_test_dir("missing_root");
        fs::create_dir_all(&root).expect("failed to create root test directory");

        for pattern in [
            "missing/x.txt",
            "missing/*.txt",
            "../nope/x.txt",
            "../nope/*.txt",
        ] {
            let paths = collect_ok_paths(
                glob_with(&root, pattern, &GlobWalkOptions::default())
                    .expect("missing-path glob should compile"),
            )
            .unwrap_or_else(|err| panic!("{pattern}: {err:?}"));
            assert!(paths.is_empty(), "{pattern} should not match anything");
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_negated_character_class_excludes_files() {
        let root = unique_test_dir("negated_class");
        fs::create_dir_all(&root).unwrap();
        write_file(&root.join("foo"));
        write_file(&root.join("bar"));
        let result = glob_with(&root, "[!f]*", &GlobWalkOptions::default()).unwrap();
        let paths = collect_ok_paths(result).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(paths, vec!["bar"]);
    }

    #[test]
    fn glob_with_streams_and_matches_simple_pattern() {
        let root = unique_test_dir("basic");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("five.txt"));
        write_file(&root.join("six.md"));

        let result = glob_with(root.as_path(), "*.txt", &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect streamed paths");

        assert_eq!(paths, vec!["five.txt"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_explicit_current_directory_prefix() {
        let root = unique_test_dir("current_directory");
        write_file(&root.join("one.txt"));
        write_file(&root.join("nested/two.txt"));

        let paths = collect_ok_paths(
            glob_with(&root, "./*.txt", &GlobWalkOptions::default())
                .expect("current-directory glob should compile"),
        )
        .expect("current-directory glob should traverse");
        assert_eq!(paths, vec!["one.txt"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_explicit_parent_directory_prefix() {
        let root = unique_test_dir("parent_directory");
        write_file(&root.join("one.txt"));
        write_file(&root.join("child/grandchild/local.txt"));

        for (base, pattern, expected) in [
            (
                root.join("child"),
                "../*.txt",
                expected_path(&["..", "one.txt"]),
            ),
            (
                root.join("child/grandchild"),
                "../../*.txt",
                expected_path(&["..", "..", "one.txt"]),
            ),
        ] {
            let paths = collect_ok_paths(
                glob_with(base, pattern, &GlobWalkOptions::default())
                    .expect("parent-directory glob should compile"),
            )
            .expect("parent-directory glob should traverse");
            assert_eq!(paths, vec![expected], "pattern: {pattern}");
        }

        for pattern in ["../missing", "../one.txt/missing"] {
            let paths = collect_ok_paths(
                glob_with(root.join("child"), pattern, &GlobWalkOptions::default())
                    .expect("missing literal glob should compile"),
            )
            .expect("missing literal glob should be an ordinary non-match");
            assert!(paths.is_empty(), "pattern: {pattern}");
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_parent_directory_recursion() {
        let root = unique_test_dir("parent_recursion");
        write_file(&root.join("one.txt"));
        write_file(&root.join("child/two.txt"));
        write_file(&root.join("sibling/deep/three.txt"));

        let paths = collect_ok_paths(
            glob_with(
                root.join("child"),
                "../**/*.txt",
                &GlobWalkOptions::default(),
            )
            .expect("recursive parent glob should compile"),
        )
        .expect("recursive parent glob should traverse");
        let mut expected = vec![
            expected_path(&["..", "one.txt"]),
            expected_path(&["..", "child", "two.txt"]),
            expected_path(&["..", "sibling", "deep", "three.txt"]),
        ];
        expected.sort();
        assert_eq!(paths, expected);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_explicit_directory_and_literal_paths() {
        let root = unique_test_dir("literal_relative_paths");
        write_file(&root.join("one.txt"));
        fs::create_dir_all(root.join("child")).expect("failed to create child directory");

        for (pattern, expected) in [
            (".", String::new()),
            ("..", "..".to_string()),
            ("../child", expected_path(&["..", "child"])),
            ("../one.txt", expected_path(&["..", "one.txt"])),
        ] {
            let paths = collect_ok_paths(
                glob_with(root.join("child"), pattern, &GlobWalkOptions::default())
                    .expect("literal relative glob should compile"),
            )
            .expect("literal relative glob should traverse");
            assert_eq!(paths, vec![expected], "pattern: {pattern}");
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_parent_directory_respects_depth_and_excludes() {
        let root = unique_test_dir("parent_options");
        write_file(&root.join("one.txt"));
        write_file(&root.join("child/two.txt"));
        write_file(&root.join("skip/three.txt"));
        let options = GlobWalkOptions {
            max_depth: Some(2),
            excludes: vec!["../skip/**".to_string()],
            ..Default::default()
        };

        let paths = collect_ok_paths(
            glob_with(root.join("child"), "../**/*.txt", &options)
                .expect("parent glob with options should compile"),
        )
        .expect("parent glob with options should traverse");
        let mut expected = vec![
            expected_path(&["..", "one.txt"]),
            expected_path(&["..", "child", "two.txt"]),
        ];
        expected.sort();
        assert_eq!(paths, expected);

        let options = GlobWalkOptions {
            max_depth: Some(1),
            ..options
        };
        let paths = collect_ok_paths(
            glob_with(root.join("child"), "../**/*.txt", &options)
                .expect("shallow parent glob should compile"),
        )
        .expect("shallow parent glob should traverse");
        assert_eq!(paths, vec![expected_path(&["..", "one.txt"])]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_literal_relative_path_respects_excludes_and_interrupt() {
        let root = unique_test_dir("literal_options");
        fs::create_dir_all(root.join("child")).expect("failed to create child directory");
        let options = GlobWalkOptions {
            excludes: vec!["..".to_string()],
            ..Default::default()
        };
        let paths = collect_ok_paths(
            glob_with(root.join("child"), "..", &options)
                .expect("excluded literal glob should compile"),
        )
        .expect("excluded literal glob should traverse");
        assert!(paths.is_empty());

        let options = GlobWalkOptions {
            interrupt: Some(Arc::new(AtomicBool::new(true))),
            ..Default::default()
        };
        let paths = collect_ok_paths(
            glob_with(&root, ".", &options).expect("interrupted literal glob should compile"),
        )
        .expect("interrupted literal glob should finish");
        assert!(paths.is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn glob_with_parent_after_symlink_uses_filesystem_parent() {
        let root = unique_test_dir("symlink_parent");
        write_file(&root.join("wrong.txt"));
        write_file(&root.join("target/correct.txt"));
        fs::create_dir_all(root.join("target/child")).expect("failed to create symlink target");
        std::os::unix::fs::symlink(root.join("target/child"), root.join("link"))
            .expect("failed to create directory symlink");

        let paths = collect_ok_paths(
            glob_with(&root, "link/../*.txt", &GlobWalkOptions::default())
                .expect("symlink-parent glob should compile"),
        )
        .expect("symlink-parent glob should traverse");
        assert_eq!(paths, vec![expected_path(&["link", "..", "correct.txt"])]);

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn glob_with_literal_symlink_loop_reports_error() {
        let root = unique_test_dir("literal_symlink_loop");
        fs::create_dir_all(&root).expect("failed to create root directory");
        std::os::unix::fs::symlink("loop", root.join("loop"))
            .expect("failed to create symlink loop");

        let mut paths = glob_with(&root, "loop/file", &GlobWalkOptions::default())
            .expect("literal glob should compile");
        assert!(matches!(paths.next(), Some(Err(_))));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_respects_depth_limit() {
        let root = unique_test_dir("depth");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("a.txt"));
        write_file(&root.join("nested/inner.txt"));

        let options = GlobWalkOptions {
            max_depth: Some(1),
            ..Default::default()
        };
        let result =
            glob_with(root.as_path(), "**/*.txt", &options).expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect streamed paths");

        assert_eq!(paths, vec!["a.txt"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_respects_excludes_and_prunes_nested_dirs() {
        let root = unique_test_dir("exclude_prune");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("src/keep/main.rs"));
        write_file(&root.join("src/target/skip.rs"));
        write_file(&root.join("src/.git/config"));
        write_file(&root.join("src/.git/hooks/pre-commit"));
        write_file(&root.join("src/node_modules/pkg/index.js"));

        let options = GlobWalkOptions {
            excludes: vec![
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
            ],
            ..Default::default()
        };

        let result = glob_with(root.as_path(), "**/*", &options).expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect streamed paths");

        assert!(paths.contains(&expected_path(&["src", "keep", "main.rs"])));
        assert!(
            !paths
                .iter()
                .any(|p| p.contains(&format!("target{}skip.rs", std::path::MAIN_SEPARATOR)))
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.contains(&format!(".git{}config", std::path::MAIN_SEPARATOR)))
        );
        assert!(!paths.iter().any(|p| p.contains(&format!(
            ".git{}hooks{}pre-commit",
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        ))));
        assert!(!paths.iter().any(|p| p.contains(&format!(
            "node_modules{}pkg{}index.js",
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        ))));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_absolute_recursive_pattern_finds_nested_files() {
        let root = unique_test_dir("absolute_recursive");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("src/lib.rs"));
        write_file(&root.join("README.md"));

        let pattern = format!("{}/**/*", root.to_string_lossy());
        let result = glob_with(root.as_path(), &pattern, &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect streamed paths");

        assert!(
            paths.iter().any(|p| {
                Path::new(p).is_absolute() && p.ends_with(&expected_path(&["src", "lib.rs"]))
            }),
            "absolute recursive pattern should include nested absolute files"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn debug_helpers_behave_as_expected() {
        let parse = debug_parse("**/*.txt");
        assert!(parse.contains("Recurse"));

        let compile = debug_compile("**/*.txt").expect("debug_compile should succeed");
        assert!(compile.contains("Complete"));

        assert!(debug_matches("*.txt", "file.txt").expect("debug_matches should succeed"));
        assert!(!debug_matches("*.txt", "file.md").expect("debug_matches should succeed"));
    }

    #[test]
    fn detect_fast_path_for_recursive_richer_basename_pattern() {
        let parsed = parser::parse("crates/**/mod*.rs");
        let fast_path = detect_recursive_suffix_fast_path(&parsed);

        assert!(matches!(
            fast_path,
            Some(globber::RecursiveFastPath::BasenamePattern(_))
        ));
    }

    #[test]
    fn glob_with_matches_recursive_richer_basename_pattern() {
        let root = unique_test_dir("richer_tail");
        fs::create_dir_all(&root).expect("failed to create root test directory");
        write_file(&root.join("crates/nu-glob/src/mod.rs"));
        write_file(&root.join("crates/nu-glob/src/index.rs"));
        write_file(&root.join("crates/nu-protocol/src/mod_helpers.rs"));

        let result = glob_with(
            root.as_path(),
            "crates/**/mod*.rs",
            &GlobWalkOptions::default(),
        )
        .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect streamed paths");

        assert!(paths.contains(&expected_path(&["crates", "nu-glob", "src", "mod.rs"])));
        assert!(paths.contains(&expected_path(&[
            "crates",
            "nu-protocol",
            "src",
            "mod_helpers.rs"
        ])));
        assert!(!paths.contains(&expected_path(&["crates", "nu-glob", "src", "index.rs"])));

        let _ = fs::remove_dir_all(&root);
    }

    // ── parser tests ──────────────────────────────────────────────────────────

    fn node_kinds(pattern: &str) -> Vec<std::mem::Discriminant<AstNode>> {
        parser::parse(pattern)
            .nodes
            .iter()
            .map(std::mem::discriminant)
            .collect()
    }

    fn discriminant<T>(val: &T) -> std::mem::Discriminant<T> {
        std::mem::discriminant(val)
    }

    #[test]
    fn parser_literal_string() {
        let p = parser::parse("hello");
        assert_eq!(p.nodes.len(), 1);
        match &p.nodes[0] {
            AstNode::LiteralString(bytes) => assert_eq!(bytes, b"hello"),
            other => panic!("expected LiteralString, got {other:?}"),
        }
    }

    #[test]
    fn parser_wildcard() {
        let p = parser::parse("*");
        assert!(p.nodes.iter().any(|n| matches!(n, AstNode::Wildcard)));
    }

    #[test]
    fn parser_recurse() {
        let p = parser::parse("**");
        assert!(p.nodes.iter().any(|n| matches!(n, AstNode::Recurse)));
        // ** must not produce a plain Wildcard
        assert!(!p.nodes.iter().any(|n| matches!(n, AstNode::Wildcard)));
    }

    #[test]
    fn parser_any_character() {
        let p = parser::parse("?");
        assert_eq!(p.nodes.len(), 1);
        assert!(matches!(p.nodes[0], AstNode::AnyCharacter));
    }

    #[test]
    fn parser_separator_between_components() {
        let p = parser::parse("a/b");
        assert!(p.nodes.iter().any(|n| matches!(n, AstNode::Separator)));
        assert!(
            p.nodes
                .iter()
                .any(|n| matches!(n, AstNode::LiteralString(_)))
        );
    }

    #[test]
    fn parser_cur_dir() {
        let p = parser::parse("./foo");
        assert!(p.nodes.iter().any(|n| matches!(n, AstNode::CurDir)));
    }

    #[test]
    fn parser_parent_dir() {
        let p = parser::parse("../foo");
        assert!(p.nodes.iter().any(|n| matches!(n, AstNode::ParentDir)));
    }

    #[test]
    fn parser_alternatives() {
        let p = parser::parse("{a,b,c}");
        assert_eq!(p.nodes.len(), 1);
        match &p.nodes[0] {
            AstNode::Alternatives { choices } => assert_eq!(choices.len(), 3),
            other => panic!("expected Alternatives, got {other:?}"),
        }
    }

    #[test]
    fn parser_character_class_range() {
        let p = parser::parse("[a-z]");
        assert_eq!(p.nodes.len(), 1);
        match &p.nodes[0] {
            AstNode::Characters {
                classes,
                negated: false,
            } => {
                assert_eq!(classes.len(), 1);
                assert_eq!(classes[0], CharacterClass::Range('a', 'z'));
            }
            other => panic!("expected Characters, got {other:?}"),
        }
    }

    #[test]
    fn parser_character_class_single() {
        let p = parser::parse("[abc]");
        assert_eq!(p.nodes.len(), 1);
        match &p.nodes[0] {
            AstNode::Characters {
                classes,
                negated: false,
            } => {
                assert_eq!(classes.len(), 3);
                assert!(classes.contains(&CharacterClass::Single('a')));
                assert!(classes.contains(&CharacterClass::Single('b')));
                assert!(classes.contains(&CharacterClass::Single('c')));
            }
            other => panic!("expected Characters, got {other:?}"),
        }
    }

    /// As in bash, zsh and fish, braces without a comma between them are literal, and they stay
    /// one literal with the text around them, so the globber can treat it as a path component.
    #[test]
    fn parser_brace_group_without_comma_is_literal() {
        for pattern in ["a{b}", "{}", "HEAD@{1}", "{a", "a}"] {
            let p = parser::parse(pattern);
            match p.nodes.as_slice() {
                [AstNode::LiteralString(bytes)] => assert_eq!(bytes, pattern.as_bytes()),
                other => panic!("`{pattern}` should be one literal, got {other:?}"),
            }
        }
    }

    #[test]
    fn parser_literal_group_inside_alternatives_keeps_the_outer_group_open() {
        let p = parser::parse("{a,{b}}");
        match p.nodes.as_slice() {
            [AstNode::Alternatives { choices }] => {
                let choices: Vec<_> = choices
                    .iter()
                    .map(|choice| match choice.nodes.as_slice() {
                        [AstNode::LiteralString(bytes)] => {
                            String::from_utf8_lossy(bytes).into_owned()
                        }
                        other => panic!("expected one literal, got {other:?}"),
                    })
                    .collect();
                assert_eq!(choices, ["a", "{b}"]);
            }
            other => panic!("expected Alternatives, got {other:?}"),
        }
    }

    /// wax's `<x:n,m>` repeats aren't pattern syntax, and `\` doesn't escape, as with nu-glob.
    #[test]
    fn parser_repeats_and_backslashes_are_literal() {
        let p = parser::parse("<a:3>");
        match p.nodes.as_slice() {
            [AstNode::LiteralString(bytes)] => assert_eq!(bytes, b"<a:3>"),
            other => panic!("expected one literal, got {other:?}"),
        }
        #[cfg(not(windows))]
        {
            let p = parser::parse(r"a\*");
            match p.nodes.as_slice() {
                [AstNode::LiteralString(bytes), AstNode::Wildcard] => assert_eq!(bytes, br"a\"),
                other => panic!("expected a literal and a wildcard, got {other:?}"),
            }
        }
    }

    #[test]
    fn parser_glob_pattern_recurse_then_literal() {
        let p = parser::parse("**/*.rs");
        // Should contain Recurse, Separator, Wildcard, LiteralString(".rs")
        let kinds = node_kinds("**/*.rs");
        assert!(
            kinds.contains(&discriminant(&AstNode::Recurse)),
            "must have Recurse"
        );
        assert!(
            kinds.contains(&discriminant(&AstNode::Separator)),
            "must have Separator"
        );
        assert!(
            kinds.contains(&discriminant(&AstNode::Wildcard)),
            "must have Wildcard"
        );
        let _ = p; // used above
    }

    // ── compiler tests ────────────────────────────────────────────────────────

    fn compile_pattern(pattern: &str) -> Program {
        let parsed = parser::parse(pattern);
        compiler::compile(&parsed).expect("compile should not fail for valid pattern")
    }

    fn last_instruction(prog: &Program) -> &Instruction {
        prog.instructions
            .last()
            .expect("program must have at least one instruction")
    }

    #[test]
    fn compiler_program_ends_with_complete() {
        let prog = compile_pattern("*.txt");
        assert_eq!(last_instruction(&prog), &Instruction::Complete);
    }

    #[test]
    fn compiler_literal_produces_literal_string_instruction() {
        let prog = compile_pattern("hello");
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::LiteralString(b) if &**b == b"hello")),
            "expected LiteralString(hello) in {:?}",
            prog.instructions
        );
    }

    #[test]
    fn compiler_wildcard_produces_alternative_jump_gadget() {
        let prog = compile_pattern("*");
        // Wildcard gadget must contain Alternative and AnyCharacter instructions
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::Alternative(_))),
            "wildcard must produce Alternative instruction"
        );
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::AnyCharacter)),
            "wildcard must produce AnyCharacter instruction"
        );
    }

    #[test]
    fn compiler_recurse_produces_anystring_separator_gadget() {
        let prog = compile_pattern("**");
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::AnyString)),
            "recurse must produce AnyString instruction"
        );
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::Separator)),
            "recurse must produce Separator instruction"
        );
        assert!(prog.dirs_only, "bare ** must set dirs_only");
    }

    #[test]
    fn compiler_star_star_slash_star_is_not_dirs_only() {
        let prog = compile_pattern("**/*");
        assert!(
            !prog.dirs_only,
            "**/* must not be treated as directory-only trailing **"
        );
    }

    #[test]
    fn compiler_prefixed_dirs_only_sets_flag() {
        let prog = compile_pattern("foo/**");
        assert!(prog.dirs_only, "foo/** must set dirs_only");
    }

    #[test]
    fn compiler_multi_component_dirs_only_keeps_intermediate_separators() {
        // Regression: separator elision must only drop the `/` immediately before
        // pure terminal `**`, not every `/` in `a/b/**` (broke `ls **` via absolute
        // `{cwd}/**` rewriting in glob_from).
        let prog = compile_pattern("a/b/**");
        assert!(prog.dirs_only, "a/b/** must set dirs_only");

        let mut saw_a = false;
        let mut saw_sep_after_a = false;
        let mut saw_b = false;
        let mut saw_boundary = false;
        for inst in &prog.instructions {
            match inst {
                Instruction::LiteralString(bytes) if &**bytes == b"a" => {
                    saw_a = true;
                }
                Instruction::Separator if saw_a && !saw_b => {
                    saw_sep_after_a = true;
                }
                Instruction::LiteralString(bytes) if &**bytes == b"b" => {
                    assert!(
                        saw_sep_after_a,
                        "a/b/** must keep Separator between a and b, got {prog}"
                    );
                    saw_b = true;
                }
                Instruction::ComponentBoundary if saw_b => {
                    saw_boundary = true;
                }
                _ => {}
            }
        }
        assert!(saw_a && saw_b && saw_boundary, "unexpected program: {prog}");
    }

    #[test]
    fn compiler_any_character_produces_any_character_instruction() {
        let prog = compile_pattern("?");
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::AnyCharacter)),
            "? must produce AnyCharacter instruction"
        );
    }

    #[test]
    fn compiler_alternatives_produce_alternative_instruction() {
        let prog = compile_pattern("{foo,bar}");
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::Alternative(_))),
            "{{foo,bar}} must produce Alternative instruction"
        );
    }

    #[test]
    fn compiler_absolute_path_sets_absolute_prefix_on_unix() {
        // On Unix, /foo/bar has RootDir + literal components
        #[cfg(unix)]
        {
            let prog = compile_pattern("/foo/bar");
            assert!(
                prog.absolute_prefix.is_some(),
                "absolute path should set absolute_prefix"
            );
        }
    }

    #[test]
    fn compiler_relative_path_leaves_absolute_prefix_empty() {
        let prog = compile_pattern("foo/bar");
        assert!(
            prog.absolute_prefix.is_none(),
            "relative path must not set absolute_prefix"
        );
    }

    // ── matcher tests ─────────────────────────────────────────────────────────

    fn compile_for_match(pattern: &str) -> Program {
        let parsed = parser::parse(pattern);
        compiler::compile(&parsed).expect("compile should succeed")
    }

    fn matches_complete(pattern: &str, path: &str) -> bool {
        let prog = compile_for_match(pattern);
        path_matches(Path::new(path), false, &prog).valid_as_complete_match
    }

    fn matches_prefix(pattern: &str, path: &str) -> bool {
        let prog = compile_for_match(pattern);
        path_matches(Path::new(path), false, &prog).valid_as_prefix
    }

    #[test]
    fn matcher_literal_exact_match() {
        assert!(matches_complete("hello", "hello"));
        assert!(!matches_complete("hello", "world"));
    }

    #[test]
    fn matcher_wildcard_matches_any_string_without_separator() {
        assert!(matches_complete("*.txt", "file.txt"));
        assert!(matches_complete("*.txt", "my_file.txt"));
        assert!(!matches_complete("*.txt", "file.md"));
        // wildcard does not cross separator boundaries
        assert!(!matches_complete("*.txt", "dir/file.txt"));
    }

    #[test]
    fn matcher_any_character_matches_single_char() {
        assert!(matches_complete("f?o", "foo"));
        assert!(matches_complete("f?o", "fXo"));
        assert!(!matches_complete("o?f", "of"));
        assert!(!matches_complete("f?o", "fXXo"));
    }

    #[test]
    fn matcher_recurse_matches_across_separators() {
        assert!(matches_complete("**/*.txt", "a/b/c/file.txt"));
        assert!(matches_complete("**/*.txt", "file.txt"));
        assert!(!matches_complete("**/*.txt", "a/b/c/file.md"));
    }

    #[test]
    fn matcher_character_class_range() {
        assert!(matches_complete("[a-z]", "a"));
        assert!(matches_complete("[a-z]", "m"));
        assert!(matches_complete("[a-z]", "z"));
        assert!(!matches_complete("[a-z]", "A"));
        assert!(!matches_complete("[a-z]", "1"));
    }

    #[test]
    fn matcher_character_class_single_chars() {
        assert!(matches_complete("[abc]", "a"));
        assert!(matches_complete("[abc]", "b"));
        assert!(matches_complete("[abc]", "c"));
        assert!(!matches_complete("[abc]", "d"));
    }

    #[test]
    fn matcher_alternatives() {
        assert!(matches_complete("{foo,bar}", "foo"));
        assert!(matches_complete("{foo,bar}", "bar"));
        assert!(!matches_complete("{foo,bar}", "baz"));
    }

    #[test]
    fn matcher_alternatives_in_path() {
        assert!(matches_complete("src/{lib,main}.rs", "src/lib.rs"));
        assert!(matches_complete("src/{lib,main}.rs", "src/main.rs"));
        assert!(!matches_complete("src/{lib,main}.rs", "src/other.rs"));
    }

    #[test]
    fn matcher_valid_as_prefix_with_short_path() {
        // "a/b/c.txt" pattern – the path "a/b" is a valid prefix (could match with more input)
        assert!(matches_prefix("a/b/c.txt", "a/b"));
        // but a completely unrelated path is not a valid prefix
        assert!(!matches_prefix("a/b/c.txt", "x/y"));
    }

    #[test]
    fn matcher_recurse_double_star_matches_deep_paths() {
        // **/*.rs matches files at any depth
        assert!(matches_complete("**/*.rs", "src/lib.rs"));
        assert!(matches_complete("**/*.rs", "a/b/c/deep.rs"));
        assert!(!matches_complete("**/*.rs", "src/lib.txt"));
        // ** as a prefix matches with a separator suffix
        assert!(matches_prefix("**/foo", "a/b"));
    }

    #[test]
    fn matcher_bare_double_star_matches_any_depth() {
        // As with nu-glob, bare ** needs a component, so the empty path (start dir) doesn't match.
        assert!(!matches_complete("**", ""));
        assert!(matches_complete("**", "1"));
        assert!(matches_complete("**", "1/2"));
        assert!(matches_complete("**", "1/2/3"));
    }

    #[test]
    fn matcher_star_slash_star_requires_two_components() {
        // Regression for #18600: empty `*` must not skip a path component.
        assert!(!matches_complete("*/*", "1"));
        assert!(matches_complete("*/*", "1/2"));
        assert!(!matches_complete("*/*", "1/2/3"));
    }

    #[test]
    fn matcher_prefixed_trailing_double_star_needs_a_component() {
        // As with nu-glob, `foo/**` matches what is below `foo`, not `foo` itself.
        assert!(!matches_complete("foo/**", "foo"));
        assert!(matches_complete("foo/**", "foo/bar"));
        assert!(matches_complete("foo/**", "foo/bar/baz"));
        assert!(!matches_complete("foo/**", "bar"));
        assert!(!matches_complete("foo/**", "foobar"));
        // #18600 guard still holds after Separator-at-EOF advance.
        assert!(!matches_complete("*/*", "1"));
    }

    #[test]
    fn matcher_nested_recursive_stars_enforce_min_depth() {
        // **/* matches paths at any depth (rust-lang/glob also matches the empty
        // string; filesystem expansion still omits the start directory).
        assert!(matches_complete("**/*", "1"));
        assert!(matches_complete("**/*", "1/2"));
        assert!(matches_complete("**/*", "1/2/3"));

        // **/*/* requires at least two components
        assert!(!matches_complete("**/*/*", "1"));
        assert!(matches_complete("**/*/*", "1/2"));
        assert!(matches_complete("**/*/*", "1/2/3"));

        // **/*/*/* requires at least three components
        assert!(!matches_complete("**/*/*/*", "1"));
        assert!(!matches_complete("**/*/*/*", "1/2"));
        assert!(matches_complete("**/*/*/*", "1/2/3"));
        assert!(matches_complete("**/*/*/*", "1/2/3/4"));
    }

    #[test]
    fn matcher_double_star_slash_literal_matches_at_any_depth() {
        assert!(matches_complete("**/foo", "foo"));
        assert!(matches_complete("**/foo", "a/foo"));
        assert!(matches_complete("**/foo", "a/b/foo"));
        assert!(!matches_complete("**/foo", "foo/bar"));
        assert!(!matches_complete("**/foo", "bar"));
    }

    #[test]
    fn matcher_literal_multi_component_path() {
        assert!(matches_complete("foo/bar/baz", "foo/bar/baz"));
        assert!(!matches_complete("foo/bar/baz", "foo/bar"));
        assert!(!matches_complete("foo/bar/baz", "foo/bar/baz/extra"));
    }

    #[test]
    fn matcher_complete_match_requires_full_consumption() {
        // Pattern "*.txt" does NOT match "file.txt.bak" as a complete match
        assert!(!matches_complete("*.txt", "file.txt.bak"));
    }

    #[test]
    fn matcher_braces_alternate_only_with_a_comma() {
        assert!(matches_complete("*.{rs,toml}", "a.rs"));
        assert!(matches_complete("*.{rs,toml}", "a.toml"));
        assert!(!matches_complete("*.{rs,toml}", "a.txt"));
        assert!(matches_complete("{a,{b}}", "{b}"));
        assert!(matches_complete("a{b}*", "a{b}.txt"));
        assert!(!matches_complete("a{b}*", "ab.txt"));
        // wax's repeats are gone: `<a:3>` names itself.
        assert!(matches_complete("<a:3>", "<a:3>"));
        assert!(!matches_complete("<a:3>", "aaa"));
    }

    // ── traversal start dir tests ──────────────────────────────────────────────
    //
    // These test that traversal_start_dir correctly excludes filename-pattern
    // literals from the starting directory (Bug #1 in the dc-glob regression report).

    #[test]
    fn glob_with_issue_18600_nested_depth_and_bare_double_star() {
        // Fixture from https://github.com/nushell/nushell/issues/18600
        // mkdir 0/1/2/3 && cd 0
        let root = unique_test_dir("issue_18600");
        fs::create_dir_all(root.join("1/2/3")).expect("failed to create nested dirs");
        write_file(&root.join("1/2/3/file.txt"));

        let options = GlobWalkOptions::default();

        // bare ** → nested directories only, as with nu-glob: not the start, not files
        let paths = collect_ok_paths(
            glob_with(root.as_path(), "**", &options).expect("glob ** should succeed"),
        )
        .expect("collect **");
        assert!(
            !paths.iter().any(|p| p.is_empty()),
            "bare ** must not include the start directory, got {paths:?}"
        );
        assert!(paths.contains(&expected_path(&["1"])));
        assert!(paths.contains(&expected_path(&["1", "2"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3"])));
        assert!(
            !paths.iter().any(|p| p.ends_with("file.txt")),
            "bare ** must not list files, got {paths:?}"
        );

        // **/* → all entries under start, not including start itself
        let paths = collect_ok_paths(
            glob_with(root.as_path(), "**/*", &options).expect("glob **/* should succeed"),
        )
        .expect("collect **/*");
        assert!(
            !paths.iter().any(|p| p.is_empty()),
            "**/* must not include the start directory, got {paths:?}"
        );
        assert!(paths.contains(&expected_path(&["1"])));
        assert!(paths.contains(&expected_path(&["1", "2"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3", "file.txt"])));

        // **/*/* → min depth 2
        let paths = collect_ok_paths(
            glob_with(root.as_path(), "**/*/*", &options).expect("glob **/*/* should succeed"),
        )
        .expect("collect **/*/*");
        assert!(!paths.contains(&expected_path(&["1"])));
        assert!(paths.contains(&expected_path(&["1", "2"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3", "file.txt"])));

        // **/*/*/* → min depth 3
        let paths = collect_ok_paths(
            glob_with(root.as_path(), "**/*/*/*", &options).expect("glob **/*/*/* should succeed"),
        )
        .expect("collect **/*/*/*");
        assert!(!paths.contains(&expected_path(&["1"])));
        assert!(!paths.contains(&expected_path(&["1", "2"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3"])));
        assert!(paths.contains(&expected_path(&["1", "2", "3", "file.txt"])));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_prefixed_trailing_double_star_lists_dirs_below_prefix() {
        // As with nu-glob, `foo/**` lists the directories below `foo`, not `foo` itself, and no
        // files.
        let root = unique_test_dir("prefixed_trailing");
        fs::create_dir_all(root.join("foo/bar")).expect("failed to create nested dirs");
        write_file(&root.join("foo/bar/file.txt"));
        write_file(&root.join("foo/sibling.txt"));

        let paths = collect_ok_paths(
            glob_with(root.as_path(), "foo/**", &GlobWalkOptions::default())
                .expect("glob foo/** should succeed"),
        )
        .expect("collect foo/**");

        assert_eq!(paths, vec![expected_path(&["foo", "bar"])]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_multi_component_trailing_double_star() {
        // `a/b/**` must keep intermediate separators and emit the dirs below a/b only.
        let root = unique_test_dir("multi_component_trailing");
        fs::create_dir_all(root.join("a/b/c")).expect("failed to create nested dirs");
        write_file(&root.join("a/b/c/file.txt"));
        write_file(&root.join("a/sibling.txt"));

        let paths = collect_ok_paths(
            glob_with(root.as_path(), "a/b/**", &GlobWalkOptions::default())
                .expect("glob a/b/** should succeed"),
        )
        .expect("collect a/b/**");

        assert!(
            !paths.contains(&expected_path(&["a", "b"])),
            "a/b/** must not include the prefix directory itself, got {paths:?}"
        );
        assert!(paths.contains(&expected_path(&["a", "b", "c"])));
        assert!(
            !paths
                .iter()
                .any(|p| p.ends_with("file.txt") || p.ends_with("sibling.txt")),
            "a/b/** must not list files, got {paths:?}"
        );
        assert!(
            !paths.contains(&expected_path(&["a"])),
            "a/b/** must not list the parent of the prefix, got {paths:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_absolute_trailing_double_star() {
        // Absolute `{root}/**` is the shape `glob_from` builds for `ls **`.
        let root = unique_test_dir("absolute_trailing");
        fs::create_dir_all(root.join("1/2")).expect("failed to create nested dirs");
        write_file(&root.join("1/2/file.txt"));

        let pattern = format!("{}/**", root.to_string_lossy());
        let paths = collect_ok_paths(
            glob_with(root.as_path(), &pattern, &GlobWalkOptions::default())
                .expect("glob absolute /** should succeed"),
        )
        .expect("collect absolute /**");

        let root_str = root.to_string_lossy().to_string();
        assert!(
            !paths.iter().any(|p| {
                let p = p.trim_end_matches(['/', '\\']);
                p == root_str.trim_end_matches(['/', '\\'])
            }),
            "absolute /** must not include the start directory, got {paths:?}"
        );
        assert!(
            paths
                .iter()
                .any(|p| Path::new(p).ends_with(expected_path(&["1"]))),
            "absolute /** should include nested dir 1, got {paths:?}"
        );
        assert!(
            paths
                .iter()
                .any(|p| Path::new(p).ends_with(expected_path(&["1", "2"]))),
            "absolute /** should include nested dir 1/2, got {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p.ends_with("file.txt")),
            "absolute /** must not list files, got {paths:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_trailing_double_star_respects_excludes_on_start() {
        // Start-dir emit for trailing `**` must consult exclude_patterns.
        let root = unique_test_dir("trailing_exclude");
        fs::create_dir_all(root.join("foo/bar")).expect("create foo/bar");
        fs::create_dir_all(root.join("other")).expect("create other");
        fs::create_dir_all(root.join("1")).expect("create 1");

        // Prefixed `foo/**`: excluding `foo` must suppress the start-dir emit and descendants.
        let options = GlobWalkOptions {
            excludes: vec!["foo".to_string(), "foo/**".to_string()],
            ..Default::default()
        };
        let paths = collect_ok_paths(
            glob_with(root.as_path(), "foo/**", &options).expect("glob foo/** should succeed"),
        )
        .expect("collect foo/** with excludes");

        assert!(
            !paths.iter().any(|p| {
                p == &expected_path(&["foo"])
                    || p.starts_with(&format!("foo{}", std::path::MAIN_SEPARATOR))
            }),
            "excluded foo/** must not emit foo or descendants, got {paths:?}"
        );

        // Bare `**` with nested excludes lists the other directories, and never the start.
        let options = GlobWalkOptions {
            excludes: vec!["1".to_string(), "1/**".to_string()],
            ..Default::default()
        };
        let paths = collect_ok_paths(
            glob_with(root.as_path(), "**", &options).expect("glob ** should succeed"),
        )
        .expect("collect ** with excludes");
        assert!(
            !paths.iter().any(|p| p.is_empty()),
            "bare ** should not emit the start dir, got {paths:?}"
        );
        assert!(!paths.contains(&expected_path(&["1"])));
        assert!(paths.contains(&expected_path(&["other"])));
        assert!(paths.contains(&expected_path(&["foo"])));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_dir_prefix_literal_then_wildcard() {
        // Pattern `dir/nu*` — "dir" is a directory literal, "nu*" is a filename
        // pattern. The traversal must start from `root/dir`, NOT `root/dir/nu`.
        let root = unique_test_dir("literal_wildcard");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("dir/nu_test1"));
        write_file(&root.join("dir/nu_test2"));
        write_file(&root.join("dir/other"));

        let result = glob_with(root.as_path(), "dir/nu*", &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect paths");

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&expected_path(&["dir", "nu_test1"])));
        assert!(paths.contains(&expected_path(&["dir", "nu_test2"])));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_dir_prefix_wildcard_then_literal() {
        // Pattern `dir/*nu*` — wildcard comes first in the filename pattern.
        // This must continue to work correctly (regression guard).
        let root = unique_test_dir("wildcard_literal");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("dir/nu_test1"));
        write_file(&root.join("dir/nu_test2"));
        write_file(&root.join("dir/other"));

        let result = glob_with(root.as_path(), "dir/*nu*", &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect paths");

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&expected_path(&["dir", "nu_test1"])));
        assert!(paths.contains(&expected_path(&["dir", "nu_test2"])));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_nested_dir_prefix_literal_then_wildcard() {
        // Pattern `a/b/nu*` — multiple directory components before the filename
        // pattern. Traversal must start from `root/a/b`, not `root/a/b/nu`.
        let root = unique_test_dir("nested_literal_wildcard");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("a/b/nu_test1"));
        write_file(&root.join("a/b/nu_test2"));
        write_file(&root.join("a/b/other"));

        let result = glob_with(root.as_path(), "a/b/nu*", &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect paths");

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&expected_path(&["a", "b", "nu_test1"])));
        assert!(paths.contains(&expected_path(&["a", "b", "nu_test2"])));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_dir_prefix_literal_then_any_char() {
        // Pattern `dir/nu?` — "?" is a single-character wildcard. The "nu"
        // literal before it must not be included in the traversal start dir.
        let root = unique_test_dir("literal_any_char");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("dir/nu1"));
        write_file(&root.join("dir/nu2"));
        write_file(&root.join("dir/not"));

        let result = glob_with(root.as_path(), "dir/nu?", &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect paths");

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&expected_path(&["dir", "nu1"])));
        assert!(paths.contains(&expected_path(&["dir", "nu2"])));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_with_absolute_pattern_literal_then_wildcard() {
        // Absolute pattern like `<root>/dir/nu*` should emit absolute paths,
        // not paths relative to the current working directory.
        let root = unique_test_dir("absolute_literal_wildcard");
        fs::create_dir_all(&root).expect("failed to create root");
        write_file(&root.join("dir/nu_test1"));
        write_file(&root.join("dir/other"));

        let pattern = format!("{}/dir/nu*", root.to_string_lossy());
        let result = glob_with(root.as_path(), &pattern, &GlobWalkOptions::default())
            .expect("glob_with should succeed");
        let paths = collect_ok_paths(result).expect("failed to collect paths");

        assert_eq!(paths.len(), 1);
        assert!(Path::new(&paths[0]).is_absolute());

        #[cfg(windows)]
        let expected = root
            .join("dir")
            .join("nu_test1")
            .to_string_lossy()
            .replace('/', "\\");
        #[cfg(not(windows))]
        let expected = root
            .join("dir")
            .join("nu_test1")
            .to_string_lossy()
            .into_owned();

        assert_eq!(paths[0], expected);

        let _ = fs::remove_dir_all(&root);
    }

    /// dc-glob with [`GlobWalkOptions::nu_glob`] returns the same paths as nu-glob, for every
    /// option set a command passes: none (`open`, external arguments), a literal leading dot
    /// (`rm`, `cp`, `mv`, `du`) and no hidden names in recursion (`ls`). Order is not compared.
    ///
    /// The one difference kept on purpose: for a component that starts with `.*`, nu-glob also
    /// returns `.` and `..`, and dc-glob doesn't, like zsh, fish and bash 5.2. Those two entries
    /// are dropped from nu-glob's results before comparing.
    #[cfg(unix)]
    #[test]
    fn dc_glob_matches_legacy_nu_glob() {
        let root = unique_test_dir("nu_glob_parity");
        for file in [
            "top.txt",
            ".hidden",
            ".dot.txt",
            "a/one.rs",
            "a/.hidden_in_a",
            "a/b/two.rs",
            "a/b/c/three.rs",
            "a/.hd/inner.rs",
            "a/b/.cache/f.rs",
            ".hdir/x.rs",
            "a]c",
            "a[c",
            "a,b",
            "a{b}",
            "sp ace.txt",
        ] {
            write_file(&root.join(file));
        }
        fs::create_dir_all(root.join("empty")).expect("create empty dir");
        std::os::unix::fs::symlink("top.txt", root.join("link_file")).expect("file symlink");
        std::os::unix::fs::symlink("a", root.join("link_dir")).expect("dir symlink");
        std::os::unix::fs::symlink("nope", root.join("dangling")).expect("dangling symlink");

        let option_sets = [
            ("default", crate::MatchOptions::default()),
            (
                "literal leading dot",
                crate::MatchOptions {
                    require_literal_leading_dot: true,
                    ..crate::MatchOptions::default()
                },
            ),
            (
                "no hidden recursion",
                crate::MatchOptions {
                    recursive_match_hidden_dir: false,
                    ..crate::MatchOptions::default()
                },
            ),
        ];
        let patterns = [
            "*",
            "**",
            "**/*",
            "*/*",
            "a/*",
            "a/**",
            "a/**/*",
            "**/*.rs",
            "a/**/*.rs",
            "?",
            "??",
            "[ab]*",
            "[!a]*",
            "**/.h*",
            "**/.*",
            "**/b",
            "**/b/*",
            "**/*/*",
            "**/b/.cache/*",
            "**/.cache/*",
            "**/b/.*",
            "**/*/.*",
            "a/**/.cache",
            "*/b/**",
            "*/**",
            "*/",
            "a/*/",
            "a/b/",
            "a/one.rs/",
            "link_dir/",
            "**/",
            "a/**/",
            "**/*/**",
            "?/**",
            "[at]*/**",
            "link_dir/*",
            "link_dir/**",
            "empty/*",
            "nope*",
            "*.nope",
            "**/nope",
            "a]c",
            "a{b}",
            "a,b",
            "a/b/../*",
            ".*",
            "a/.*",
            "*/.*",
        ];

        for (name, options) in option_sets {
            for pattern in patterns {
                let absolute = format!("{}/{pattern}", root.display());
                let mut legacy: Vec<PathBuf> =
                    crate::glob_with(&absolute, options, crate::Uninterruptible)
                        .expect("legacy pattern should parse")
                        .map(|path| path.expect("legacy glob entry"))
                        .filter(|path| {
                            let path = path.to_string_lossy();
                            !path.ends_with("/.") && !path.ends_with("/..")
                        })
                        .collect();
                let mut dc: Vec<PathBuf> =
                    glob_with(&root, &absolute, &GlobWalkOptions::nu_glob(options))
                        .expect("dc-glob pattern should compile")
                        .map(|path| path.expect("dc-glob entry"))
                        .collect();
                // nu-glob returns some paths twice, such as `a/b/c` for `**/*/**`.
                legacy.sort();
                legacy.dedup();
                dc.sort();
                assert_eq!(dc, legacy, "`{pattern}` with {name} options");
            }
        }

        let _ = fs::remove_dir_all(&root);
    }

    /// dc-glob rejects the patterns nu-glob rejects, so a mistyped `rm ***` fails instead of
    /// expanding like `**/*`.
    #[test]
    fn dc_glob_rejects_what_nu_glob_rejects() {
        let root = unique_test_dir("nu_glob_rejects");
        fs::create_dir_all(&root).expect("create root");
        let options = GlobWalkOptions::nu_glob(crate::MatchOptions::default());

        // `a/[b/c]` is one valid class as a whole, but nu-glob also parses each component. Braces
        // are literal to nu-glob, so `**` can't be part of `{...}`.
        for pattern in [
            "***", "a**", "**a", "a/b**/*", "[a", "a/[b/c]", "{**,a}", "{a,**}/b",
        ] {
            assert!(
                crate::glob_with(
                    pattern,
                    crate::MatchOptions::default(),
                    crate::Uninterruptible
                )
                .is_err(),
                "nu-glob should reject `{pattern}`"
            );
            assert!(
                glob_with(&root, pattern, &options).is_err(),
                "dc-glob should reject `{pattern}`"
            );
        }
        // A literal prefix escaped the way `glob_from` escapes it still parses.
        let escaped = format!("{}/*", escape("[x]{a,b}**"));
        assert!(glob_with(&root, &escaped, &options).is_ok(), "{escaped}");

        let _ = fs::remove_dir_all(&root);
    }

    /// `du --exclude` and `watch --glob` match paths with [`DcPattern`] under dc-glob and with
    /// nu-glob's [`crate::Pattern`] otherwise, so the two agree on everything but braces.
    #[test]
    fn dc_pattern_matches_like_legacy_pattern() {
        let cases: &[(&str, &[&str])] = &[
            ("*.txt", &["a.txt", "x/a.txt", "/abs/x/a.txt", "a.rs"]),
            ("target/", &["target", "target/x"]),
            ("**", &["", "a", "a/b"]),
            ("a/**", &["a", "a/b", "a/b/c", "a/f.txt", "ab"]),
            (
                "**/target/**",
                &["target", "x/target", "x/target/f", "target/a/b"],
            ),
            ("*/**", &["a", "a/b"]),
            ("**/*.rs", &["a.rs", "x/a.rs", "x/a.txt"]),
            ("/tmp/x/**", &["/tmp/x", "/tmp/x/y"]),
            ("a{b}", &["a{b}", "ab"]),
            ("<a:2>", &["<a:2>", "aa"]),
            ("[ab]?", &["ax", "cx", "a"]),
        ];
        #[cfg(not(windows))]
        let cases = [cases, &[("a\\*", &["a\\b", "a*", "ab"][..])]].concat();
        for (pattern, paths) in cases.iter() {
            let legacy = crate::Pattern::new(pattern).expect("nu-glob pattern");
            let dc = DcPattern::new(pattern).expect("dc-glob pattern");
            for path in paths.iter() {
                let path = Path::new(path);
                assert_eq!(
                    dc.matches_path(path),
                    legacy.matches_path(path),
                    "`{pattern}` against `{}`",
                    path.display()
                );
            }
        }
        // As with nu-glob, a malformed pattern is an error.
        assert!(DcPattern::new("***").is_err());
        assert!(DcPattern::new("{**,a}").is_err());

        // Braces alternate, and each choice matches the way nu-glob does.
        let pattern = DcPattern::new("*.{rs,toml}").expect("dc-glob pattern");
        assert!(pattern.matches_path(Path::new("/abs/x/a.toml")));
        assert!(!pattern.matches_path(Path::new("/abs/x/a.txt")));
        let pattern = DcPattern::new("{src,tests}/*").expect("dc-glob pattern");
        assert!(pattern.matches_path(Path::new("src/a/b.rs")));
        assert!(!pattern.matches_path(Path::new("docs/b.rs")));
        assert!(
            DcPattern::new("a{b}")
                .expect("dc-glob pattern")
                .matches_path(Path::new("a{b}"))
        );
    }

    /// The brace expansion behind [`DcPattern`] follows the parser's rules.
    #[test]
    fn expand_alternatives_follows_the_brace_rules() {
        let expand = |pattern| parser::expand_alternatives(pattern).expect("expand");
        assert_eq!(expand("a{b,c}d"), ["abd", "acd"]);
        assert_eq!(expand("{a,{b,c}}"), ["a", "b", "c"]);
        assert_eq!(expand("{x,y}/{1,2}"), ["x/1", "x/2", "y/1", "y/2"]);
        assert_eq!(expand("{,a}.rs"), [".rs", "a.rs"]);
        // Without a comma or a closing brace, braces are literal; in a class they are members.
        assert_eq!(expand("a{b}"), ["a{b}"]);
        assert_eq!(expand("{a{b,c}}"), ["{ab}", "{ac}"]);
        assert_eq!(expand("{a,b"), ["{a,b"]);
        assert_eq!(expand("[{,]{x,y}"), ["[{,]x", "[{,]y"]);
        assert_eq!(expand("{[,}],z}"), ["[,}]", "z"]);
        // A pattern that spells out too many alternatives is an error, not a slow match.
        assert!(parser::expand_alternatives(&"{a,b}".repeat(11)).is_err());
        let many = format!("{{{}}}", vec!["a"; 2000].join(","));
        assert!(parser::expand_alternatives(&many).is_err());
    }

    /// `{a,b}` alternatives work wherever the pattern goes, but only arguments with `*`, `?` or
    /// `[` are expanded at all, so words like `HEAD@{1}` reach commands unchanged.
    #[test]
    fn braces_alternate_when_they_hold_a_comma() {
        let root = unique_test_dir("braces");
        for file in [
            "a.rs",
            "b.toml",
            "c.txt",
            "src/x.rs",
            "tests/y.rs",
            "docs/z.rs",
            "a{b}.txt",
            "{}",
            "{a,b}",
        ] {
            write_file(&root.join(file));
        }
        let options = GlobWalkOptions::nu_glob(crate::MatchOptions::default());
        let glob = |pattern: &str| {
            let mut paths = collect_ok_paths(
                glob_with(&root, pattern, &options).expect("dc-glob pattern should compile"),
            )
            .expect("collect paths");
            paths.sort();
            paths
        };

        assert_eq!(glob("*.{rs,toml}"), ["a.rs", "b.toml"]);
        assert_eq!(
            glob("{src,tests}/*.rs"),
            [
                expected_path(&["src", "x.rs"]),
                expected_path(&["tests", "y.rs"])
            ]
        );
        assert_eq!(glob("{c,{d,e}}.txt"), ["c.txt"]);
        assert_eq!(glob("{,a}.rs"), ["a.rs"]);
        // Without a comma, the braces are literal.
        assert_eq!(glob("a{b}*"), ["a{b}.txt"]);
        assert_eq!(glob("{}"), ["{}"]);
        // An escaped name matches itself.
        assert_eq!(glob(&escape("{a,b}")), ["{a,b}"]);
        // Excludes take braces too.
        let options = GlobWalkOptions {
            excludes: vec!["{src,tests}/*".to_string()],
            ..GlobWalkOptions::nu_glob(crate::MatchOptions::default())
        };
        let mut paths = collect_ok_paths(
            glob_with(&root, "**/*.rs", &options).expect("dc-glob pattern should compile"),
        )
        .expect("collect paths");
        paths.sort();
        assert_eq!(
            paths,
            ["a.rs".to_string(), expected_path(&["docs", "z.rs"])]
        );

        assert!(!is_glob("HEAD@{1}"));
        assert!(!is_glob("{}"));
        assert!(!is_glob("{a,b}"));
        assert!(is_glob_component("{a,b}"));
        assert!(!is_glob_component("src"));

        let _ = fs::remove_dir_all(&root);
    }
}
