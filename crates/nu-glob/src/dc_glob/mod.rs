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
#[derive(Debug, Clone, Default)]
pub struct GlobWalkOptions {
    /// Maximum recursion depth. `None` means unbounded.
    pub max_depth: Option<usize>,
    /// Follow directory symlinks while traversing.
    pub follow_symlinks: bool,
    /// Exclusion glob patterns used to filter and prune traversal.
    pub excludes: Vec<String>,
    /// Optional interrupt flag. When set to `true` the traversal stops as soon
    /// as possible and the iterator ends.
    pub interrupt: Option<Arc<AtomicBool>>,
}

/// Return true if the given pattern contains glob metacharacters.
pub fn is_glob(pattern: &str) -> bool {
    pattern.contains(GLOB_CHARS)
}

/// Escape glob metacharacters so a pattern is matched literally.
pub fn escape(pattern: &str) -> String {
    crate::Pattern::escape(pattern)
}

/// Expand `pattern` relative to `relative_to` using the experimental dc-glob backend.
pub fn glob_from(
    relative_to: impl AsRef<Path>,
    pattern: impl AsRef<str>,
) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<PathBuf>> + Send>> {
    let pattern = pattern.as_ref().to_owned();
    let parsed = parser::parse(&pattern);
    let fast_path = detect_recursive_suffix_fast_path(&parsed);
    let compiled = compiler::compile(&parsed)?;
    Ok(Box::new(globber::glob(
        relative_to.as_ref().to_path_buf(),
        Arc::new(compiled),
        fast_path,
    )))
}

/// Like [`glob_from`] but accepts an interrupt flag so the caller can cancel the traversal.
pub fn glob_from_interruptible(
    relative_to: impl AsRef<Path>,
    pattern: impl AsRef<str>,
    interrupt: Option<Arc<AtomicBool>>,
) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<PathBuf>> + Send>> {
    let pattern = pattern.as_ref().to_owned();
    let parsed = parser::parse(&pattern);
    let fast_path = detect_recursive_suffix_fast_path(&parsed);
    let compiled = compiler::compile(&parsed)?;
    Ok(Box::new(globber::glob_with_options(
        relative_to.as_ref().to_path_buf(),
        Arc::new(compiled),
        globber::TraversalOptions::default(),
        vec![],
        globber::InterruptFlag(interrupt),
        fast_path,
    )))
}

/// Expand `pattern` relative to `relative_to` using dc-glob with traversal options.
pub fn glob_with(
    relative_to: impl AsRef<Path>,
    pattern: impl AsRef<str>,
    options: &GlobWalkOptions,
) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<PathBuf>> + Send>> {
    let pattern = pattern.as_ref().to_owned();
    let parsed = parser::parse(&pattern);
    let fast_path = detect_recursive_suffix_fast_path(&parsed);
    let include_program = compiler::compile(&parsed)?;

    let exclude_programs = options
        .excludes
        .iter()
        .map(|exclude| Pattern::new(exclude).map_err(anyhow::Error::from))
        .collect::<Vec<_>>();
    let exclude_programs = exclude_programs
        .into_iter()
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
    Ok(matcher::path_matches(path.as_ref(), &compiled).valid_as_complete_match)
}

/// A compiled dc-glob pattern for repeated path-matching without filesystem traversal.
///
/// Unlike the free-function `debug_matches`, this compiles the pattern once and can be
/// reused efficiently across many paths.
#[derive(Clone)]
pub struct DcPattern {
    pattern: String,
    compiled: std::sync::Arc<compiler::Program>,
}

impl std::fmt::Debug for DcPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DcPattern")
            .field("pattern", &self.pattern)
            .finish()
    }
}

impl DcPattern {
    /// Compile a glob pattern. Returns an error if the pattern is invalid.
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
        let parsed = parser::parse(pattern);
        let compiled = compiler::compile(&parsed)?;
        Ok(DcPattern {
            pattern: pattern.to_owned(),
            compiled: std::sync::Arc::new(compiled),
        })
    }

    /// Return `true` if the given path matches this pattern.
    pub fn matches_path(&self, path: &Path) -> bool {
        matcher::path_matches(path, &self.compiled).valid_as_complete_match
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
            paths
                .iter()
                .any(|p| p == &expected_path(&["src", "lib.rs"])),
            "absolute recursive pattern should include nested files"
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
            AstNode::Characters(classes) => {
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
            AstNode::Characters(classes) => {
                assert_eq!(classes.len(), 3);
                assert!(classes.contains(&CharacterClass::Single('a')));
                assert!(classes.contains(&CharacterClass::Single('b')));
                assert!(classes.contains(&CharacterClass::Single('c')));
            }
            other => panic!("expected Characters, got {other:?}"),
        }
    }

    #[test]
    fn parser_repeat_exact() {
        let p = parser::parse("<*:3>");
        assert_eq!(p.nodes.len(), 1);
        match &p.nodes[0] {
            AstNode::Repeat { min, max, .. } => {
                assert_eq!(*min, 3);
                assert_eq!(*max, 3);
            }
            other => panic!("expected Repeat, got {other:?}"),
        }
    }

    #[test]
    fn parser_repeat_range() {
        let p = parser::parse("<*:1,4>");
        assert_eq!(p.nodes.len(), 1);
        match &p.nodes[0] {
            AstNode::Repeat { min, max, .. } => {
                assert_eq!(*min, 1);
                assert_eq!(*max, 4);
            }
            other => panic!("expected Repeat, got {other:?}"),
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

    #[test]
    fn compiler_repeat_produces_increment_and_branch_instructions() {
        let prog = compile_pattern("<*:2,4>");
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::Increment(_))),
            "repeat must produce Increment instruction"
        );
        assert!(
            prog.instructions
                .iter()
                .any(|i| matches!(i, Instruction::BranchIfLessThan(..))),
            "repeat must produce BranchIfLessThan instruction"
        );
    }

    // ── matcher tests ─────────────────────────────────────────────────────────

    fn compile_for_match(pattern: &str) -> Program {
        let parsed = parser::parse(pattern);
        compiler::compile(&parsed).expect("compile should succeed")
    }

    fn matches_complete(pattern: &str, path: &str) -> bool {
        let prog = compile_for_match(pattern);
        path_matches(Path::new(path), &prog).valid_as_complete_match
    }

    fn matches_prefix(pattern: &str, path: &str) -> bool {
        let prog = compile_for_match(pattern);
        path_matches(Path::new(path), &prog).valid_as_prefix
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
    fn matcher_repeat_exact_count() {
        // <a:3> means repeat literal "a" exactly 3 times → matches "aaa" but not "aa"
        assert!(matches_complete("<a:3>", "aaa"));
        assert!(!matches_complete("<a:3>", "aa"));
        assert!(!matches_complete("<a:3>", "aaaa"));
    }
}
