//! Compiles a glob pattern into a simple set of instructions

use super::parser::{AstNode, CharacterClass, Pattern};
use std::path::{Component, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramOffset(pub usize);

impl ProgramOffset {
    const PLACEHOLDER: ProgramOffset = ProgramOffset(usize::MAX);
}

impl std::ops::Add<usize> for ProgramOffset {
    type Output = ProgramOffset;

    fn add(self, rhs: usize) -> Self::Output {
        ProgramOffset(self.0 + rhs)
    }
}

impl std::fmt::Display for ProgramOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:>3}]", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Separator,
    Prefix(Box<str>),
    RootDir,
    CurDir,
    ParentDir,
    LiteralString(Box<[u8]>),
    AnyCharacter,
    AnyString,
    Characters {
        classes: Box<[CharacterClass]>,
        negated: bool,
    },
    /// Succeed only at a path-component boundary (no unconsumed bytes left in
    /// the current component). Clears a finished `Some([])` to `None`.
    ///
    /// Used at the start of a terminal `**` gadget, which may start the pattern or follow a root,
    /// a `**/` or a `ComponentEnd`. A `**` that shares a component with other syntax, as in
    /// `foo**`, is rejected before compiling, so in practice this only clears the component.
    ComponentBoundary,
    /// Succeed only when a path component was bound and fully consumed (`Some([])`), and clear
    /// it to `None`.
    ///
    /// Stands in for the `/` before a trailing `**`. Unlike `Separator`, it needs no path
    /// component to follow, so `foo/**` can match `foo`. Unlike `ComponentBoundary`, it fails
    /// when the component before it matched nothing, so `*/**` can't match `a` with an empty
    /// `*` and a `**` that takes `a`.
    ComponentEnd,
    /// With [`Program::skip_hidden_in_recursion`], fail when the next path component is a
    /// directory whose name starts with `.`.
    ///
    /// Each round of a `**` gadget passes this before it takes a component, and a non-terminal
    /// `**` passes it before it hands the component to the rest of the pattern too, so `**`
    /// neither descends into a hidden directory nor lets the rest of the pattern match it.
    NotHiddenDir,
    Jump(ProgramOffset),
    Alternative(ProgramOffset),
    Complete,
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const WIDTH: usize = 20;
        match self {
            Instruction::Separator => f.write_str("separator"),
            Instruction::Prefix(string) => {
                write!(f, "{:<WIDTH$} {:?}", "prefix", string)
            }
            Instruction::RootDir => f.write_str("root-dir"),
            Instruction::CurDir => f.write_str("cur-dir"),
            Instruction::ParentDir => f.write_str("parent-dir"),
            Instruction::LiteralString(bytes) => {
                let as_string = String::from_utf8_lossy(bytes);
                write!(f, "{:<WIDTH$} {:?}", "literal-string", as_string)
            }
            Instruction::AnyCharacter => f.write_str("any-character"),
            Instruction::AnyString => f.write_str("any-string"),
            Instruction::Characters { classes, negated } => {
                let name = if *negated {
                    "negated-characters"
                } else {
                    "characters"
                };
                write!(f, "{name:<WIDTH$} {classes:?}")
            }
            Instruction::ComponentBoundary => f.write_str("component-boundary"),
            Instruction::ComponentEnd => f.write_str("component-end"),
            Instruction::NotHiddenDir => f.write_str("not-hidden-dir"),
            Instruction::Jump(index) => {
                write!(f, "{:<WIDTH$} {:>05}", "jump", index)
            }
            Instruction::Alternative(index) => {
                write!(f, "{:<WIDTH$} {:>05}", "alternative", index)
            }
            Instruction::Complete => f.write_str("complete"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub absolute_prefix: Option<PathBuf>,
    pub case_insensitive: bool,
    /// True when only directories match, as with nu-glob: the pattern ends with a bare
    /// recursive `**`, or with a separator, as in `*/`.
    pub dirs_only: bool,
    /// When true, `AnyCharacter` and `Characters` (from `*`, `?` and `[...]`) don't consume a
    /// `.` at the start of a path component.
    pub literal_leading_dot: bool,
    /// When true, `**` drops a directory whose name starts with `.`: it neither descends into
    /// it nor lets the rest of the pattern match it, as with nu-glob's
    /// `recursive_match_hidden_dir: false`. `**/.*` matches `.x` files but not `.git`, while
    /// `**/b/.cache/*` still reaches `a/b/.cache/f.rs`, since a literal matches `.cache` there.
    pub skip_hidden_in_recursion: bool,
}

impl Program {
    fn here(&self) -> ProgramOffset {
        ProgramOffset(self.instructions.len())
    }
}

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# absolute_prefix={:?}", self.absolute_prefix)?;

        for (index, instruction) in self.instructions.iter().enumerate() {
            writeln!(f, "{}: {}", ProgramOffset(index), instruction)?;
        }
        Ok(())
    }
}

/// Compile a sequence of AST nodes with the same `**/` folding and terminal-`**`
/// handling as the top-level [`compile`] loop. Used for the choices of alternatives too.
fn append_nodes(out: &mut Program, nodes: &[AstNode]) -> anyhow::Result<()> {
    let mut i = 0;
    while i < nodes.len() {
        match &nodes[i] {
            // Separator immediately before a terminal `**` becomes `ComponentEnd`, so path
            // `foo` matches
            // `foo/**` without requiring Separator to succeed at EOF (which would
            // break `*/*` min-depth — issue #18600), while `*/**` still needs `*`
            // to match a whole component.
            //
            // Important: only replace it when the *remainder is itself* a terminal
            // `**`. Using
            // [`pattern_is_trailing_recursive`] here would also skip separators
            // in multi-component patterns like `a/b/**` and absolute `{cwd}/**`
            // (what `ls **` builds via `glob_from`), because those suffixes still
            // *end* with `**` even though they start with literals.
            AstNode::Separator if pattern_is_pure_trailing_recursive(&nodes[i + 1..]) => {
                out.instructions.push(Instruction::ComponentEnd);
            }
            AstNode::Recurse => {
                let terminal = is_terminal_recurse(nodes, i);
                // Fold the `/` in `**/` into the recurse gadget so a zero-length
                // `**` does not require a mandatory Separator before the rest of
                // the pattern (`**/foo` must match `foo`).
                let absorbs_separator = matches!(nodes.get(i + 1), Some(AstNode::Separator));

                if terminal {
                    append_terminal_recurse_gadget(out)?;
                } else {
                    append_nonterminal_recurse_gadget(out)?;
                }

                if absorbs_separator {
                    i += 1;
                }
            }
            other => append_program(out, other)?,
        }
        i += 1;
    }
    Ok(())
}

fn append_program(out: &mut Program, node: &AstNode) -> anyhow::Result<()> {
    match node {
        AstNode::Separator => {
            out.instructions.push(Instruction::Separator);
            Ok(())
        }
        AstNode::Prefix(prefix) => {
            out.instructions
                .push(Instruction::Prefix(prefix[..].into()));
            out.absolute_prefix
                .get_or_insert(PathBuf::new())
                .push(prefix);
            Ok(())
        }
        AstNode::RootDir => {
            out.instructions.push(Instruction::RootDir);
            out.absolute_prefix
                .get_or_insert(PathBuf::new())
                .push(Component::RootDir);
            Ok(())
        }
        AstNode::CurDir => {
            out.instructions.push(Instruction::CurDir);
            Ok(())
        }
        AstNode::ParentDir => {
            out.instructions.push(Instruction::ParentDir);
            Ok(())
        }
        AstNode::LiteralString(string) => {
            out.instructions
                .push(Instruction::LiteralString(string.as_slice().into()));
            Ok(())
        }
        AstNode::AnyCharacter => {
            out.instructions.push(Instruction::AnyCharacter);
            Ok(())
        }
        AstNode::Characters { classes, negated } => {
            out.instructions.push(Instruction::Characters {
                classes: classes.as_slice().into(),
                negated: *negated,
            });
            Ok(())
        }
        AstNode::Wildcard => append_wildcard_gadget(out),
        // Bare Recurse without surrounding-node lookahead: non-terminal form.
        // Prefer [`append_nodes`] / [`compile`] which fold `**/` and terminal `**`.
        AstNode::Recurse => append_nonterminal_recurse_gadget(out),
        AstNode::Alternatives { choices } => append_alternatives(out, choices),
    }
}

fn append_wildcard_gadget(out: &mut Program) -> anyhow::Result<()> {
    // The wildcard gadget involves creating an alternative loop with AnyCharacter
    let start = out.here();
    out.instructions.push(Instruction::Alternative(start + 2));
    out.instructions.push(Instruction::Jump(start + 4)); // the non-alternative target
    out.instructions.push(Instruction::AnyCharacter); // alternative target
    out.instructions.push(Instruction::Jump(start));
    Ok(())
}

/// Non-terminal `**` / `**/` before more pattern: zero or more full `(component /)`.
///
/// The `/` after `**` in `**/foo` is absorbed by the compiler so a zero-length
/// recurse does not require a mandatory Separator before `foo` (matching
/// rust-lang/glob: `**/foo` matches `foo`).
///
/// Each round starts with `NotHiddenDir`, ahead of both the take and the stop branch, so a
/// hidden directory that `**` reaches is neither taken nor matched by the rest of the pattern.
fn append_nonterminal_recurse_gadget(out: &mut Program) -> anyhow::Result<()> {
    // (NotHiddenDir AnyString Separator)* NotHiddenDir
    let start = out.here();
    out.instructions.push(Instruction::NotHiddenDir);
    out.instructions.push(Instruction::Alternative(start + 3));
    out.instructions.push(Instruction::Jump(start + 6)); // skip body
    out.instructions.push(Instruction::AnyString);
    out.instructions.push(Instruction::Separator);
    out.instructions.push(Instruction::Jump(start));
    Ok(())
}

/// Terminal bare `**`: match one or more path components (final component needs no trailing
/// `/`), as with nu-glob, so `foo/**` matches `foo/bar` but not `foo`.
///
/// Layout:
/// ```text
///         ComponentBoundary                 // require finished component / boundary
///         Jump(take)                        // take at least one component
/// start:  Alternative(take) ; Jump(end)     // stop (done)
/// take:   NotHiddenDir ; AnyString
///         Alternative(more) ; Jump(end)     // this component was the last
/// more:   Separator ; Jump(start)           // more components follow
/// end:
/// ```
///
/// In `foo/**` the compiler puts `ComponentEnd` ahead of the gadget for the `/`, which needs
/// `foo` to have matched a whole component.
///
/// The stop branch can only succeed at the end of the path, where there is no component left
/// to be hidden, so `NotHiddenDir` guards just the take branch.
fn append_terminal_recurse_gadget(out: &mut Program) -> anyhow::Result<()> {
    out.instructions.push(Instruction::ComponentBoundary);
    // Skip the stop branch the first time round, so at least one component is taken.
    let here = out.here();
    out.instructions.push(Instruction::Jump(here + 3)); // take
    let start = out.here();
    out.instructions.push(Instruction::Alternative(start + 2)); // take
    out.instructions.push(Instruction::Jump(start + 8)); // end
    // take:
    out.instructions.push(Instruction::NotHiddenDir);
    out.instructions.push(Instruction::AnyString);
    out.instructions.push(Instruction::Alternative(start + 6)); // more
    out.instructions.push(Instruction::Jump(start + 8)); // end after last component
    // more:
    out.instructions.push(Instruction::Separator);
    out.instructions.push(Instruction::Jump(start));
    // end: (next instruction follows)
    Ok(())
}

/// True when `nodes[index]` is `Recurse` and nothing after it (except an optional
/// trailing `Separator`) remains to match.
fn is_terminal_recurse(nodes: &[AstNode], index: usize) -> bool {
    let mut j = index + 1;
    if matches!(nodes.get(j), Some(AstNode::Separator)) {
        j += 1;
    }
    j >= nodes.len()
}

/// True when the pattern ends with a bare recursive `**` (directory-only expansion).
///
/// A `**` can't be part of `{...}`: nu-glob's syntax check rejects it before compiling.
///
/// This is used for `Program::dirs_only` (directory-only emission). It
/// is **not** the predicate for eliding separators — see
/// [`pattern_is_pure_trailing_recursive`].
fn pattern_is_trailing_recursive(nodes: &[AstNode]) -> bool {
    let mut end = nodes.len();
    while end > 0 && matches!(&nodes[end - 1], AstNode::Separator) {
        end -= 1;
    }
    if end == 0 {
        return false;
    }
    matches!(&nodes[end - 1], AstNode::Recurse)
}

/// True when `nodes` is a bare terminal `**` (optional trailing separator) with no
/// non-recursive prefix. Patterns like `b/**` start with a literal and are **not** pure.
///
/// Used only to decide whether a preceding `Separator` can be elided before the
/// terminal recurse gadget. Multi-component prefixes like `a/b/**` must keep the
/// `/` between `a` and `b`; only the separator immediately before the pure
/// trailing `**` is skipped so path `a/b` matches `a/b/**`.
fn pattern_is_pure_trailing_recursive(nodes: &[AstNode]) -> bool {
    let mut start = 0;
    while start < nodes.len() && matches!(&nodes[start], AstNode::Separator) {
        start += 1;
    }
    let nodes = &nodes[start..];
    if nodes.is_empty() {
        return false;
    }
    matches!(&nodes[0], AstNode::Recurse) && is_terminal_recurse(nodes, 0)
}

fn append_alternatives(out: &mut Program, choices: &[Pattern]) -> anyhow::Result<()> {
    // To compile alternatives, we first set up (choices.len() - 1) Alternative instructions
    let start = out.instructions.len();
    for _ in 0..choices.len().saturating_sub(1) {
        out.instructions
            .push(Instruction::Alternative(ProgramOffset::PLACEHOLDER));
    }
    let mut jumps = Vec::with_capacity(choices.len());
    for (index, choice) in choices.iter().enumerate() {
        let choice_start = out.here();
        if index > 0 {
            // For everything but the first alternative, set the target of the original Alternative
            // instruction here
            out.instructions[start + (index - 1)] = Instruction::Alternative(choice_start);
        }
        append_nodes(out, &choice.nodes)?;
        // We also put a jump to the end
        jumps.push(out.here());
        out.instructions
            .push(Instruction::Jump(ProgramOffset::PLACEHOLDER));
    }
    // Fix the jumps to the end
    for offset in jumps {
        out.instructions[offset.0] = Instruction::Jump(out.here());
    }
    Ok(())
}

pub fn compile(pattern: &Pattern) -> anyhow::Result<Program> {
    compile_with_options(pattern, false)
}

pub fn compile_with_options(pattern: &Pattern, case_insensitive: bool) -> anyhow::Result<Program> {
    let mut program = Program {
        case_insensitive,
        ..Program::default()
    };
    append_nodes(&mut program, pattern.nodes.as_slice())?;
    program.instructions.push(Instruction::Complete);
    program.dirs_only = pattern_is_trailing_recursive(pattern.nodes.as_slice());
    Ok(program)
}
