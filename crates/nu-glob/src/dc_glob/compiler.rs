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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterId(pub u16);

impl std::fmt::Display for CounterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
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
    Characters(Box<[CharacterClass]>),
    /// Succeed only at a path-component boundary (no unconsumed bytes left in
    /// the current component). Clears a finished `Some([])` to `None`.
    ///
    /// Used at the start of a terminal `**` gadget so `foo/**` matches `foo`
    /// but not `foobar` (leftover bytes in the same component).
    ComponentBoundary,
    Jump(ProgramOffset),
    Alternative(ProgramOffset),
    Increment(CounterId),
    BranchIfLessThan(ProgramOffset, CounterId, u32),
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
            Instruction::Characters(character_classes) => {
                write!(f, "{:<WIDTH$} {:?}", "characters", character_classes)
            }
            Instruction::ComponentBoundary => f.write_str("component-boundary"),
            Instruction::Jump(index) => {
                write!(f, "{:<WIDTH$} {:>05}", "jump", index)
            }
            Instruction::Alternative(index) => {
                write!(f, "{:<WIDTH$} {:>05}", "alternative", index)
            }
            Instruction::Increment(counter_id) => {
                write!(f, "{:<WIDTH$} {}", "increment", counter_id)
            }
            Instruction::BranchIfLessThan(index, counter_id, value) => {
                write!(
                    f,
                    "{:<WIDTH$} {:>05}, {} < {}",
                    "branch-if", index, counter_id, value
                )
            }
            Instruction::Complete => f.write_str("complete"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub counters: u16,
    pub absolute_prefix: Option<PathBuf>,
    pub case_insensitive: bool,
    /// True when the pattern ends with a bare recursive `**` (optionally after a
    /// trailing separator). Such patterns match directories at any depth, including
    /// the start directory, but not regular files — matching rust-lang/glob /
    /// nu-glob recursive-ending behavior.
    pub trailing_recursive: bool,
}

impl Program {
    fn here(&self) -> ProgramOffset {
        ProgramOffset(self.instructions.len())
    }
}

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "# counters={}, absolute_prefix={:?}",
            self.counters, self.absolute_prefix
        )?;

        for (index, instruction) in self.instructions.iter().enumerate() {
            writeln!(f, "{}: {}", ProgramOffset(index), instruction)?;
        }
        Ok(())
    }
}

/// Compile a sequence of AST nodes with the same `**/` folding and terminal-`**`
/// handling as the top-level [`compile`] loop. Used for nested patterns inside
/// alternatives and repeats so `{**}` / `foo/{**}` get a terminal recurse gadget.
fn append_nodes(out: &mut Program, nodes: &[AstNode]) -> anyhow::Result<()> {
    let mut i = 0;
    while i < nodes.len() {
        match &nodes[i] {
            // Separator immediately before a trailing-recursive suffix (`**`,
            // `{**}`, `foo/{**}`, …) is not emitted. The terminal recurse gadget
            // starts with `ComponentBoundary` so path `foo` matches `foo/**`
            // without requiring Separator to succeed at EOF (which would break
            // `*/*` min-depth — issue #18600).
            AstNode::Separator if pattern_is_trailing_recursive(&nodes[i + 1..]) => {}
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
        AstNode::Characters(character_classes) => {
            out.instructions
                .push(Instruction::Characters(character_classes.as_slice().into()));
            Ok(())
        }
        AstNode::Wildcard => append_wildcard_gadget(out),
        // Bare Recurse without surrounding-node lookahead: non-terminal form.
        // Prefer [`append_nodes`] / [`compile`] which fold `**/` and terminal `**`.
        AstNode::Recurse => append_nonterminal_recurse_gadget(out),
        AstNode::Alternatives { choices } => append_alternatives(out, choices),
        AstNode::Repeat { min, max, pattern } => append_repeat(out, *min, *max, pattern),
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
fn append_nonterminal_recurse_gadget(out: &mut Program) -> anyhow::Result<()> {
    // (AnyString Separator)*
    let start = out.here();
    out.instructions.push(Instruction::Alternative(start + 2));
    out.instructions.push(Instruction::Jump(start + 5)); // skip body
    out.instructions.push(Instruction::AnyString);
    out.instructions.push(Instruction::Separator);
    out.instructions.push(Instruction::Jump(start));
    Ok(())
}

/// Terminal bare `**`: match zero or more path components (final component needs no trailing `/`).
///
/// Layout:
/// ```text
///         ComponentBoundary                 // require finished component / boundary
/// start:  Alternative(take) ; Jump(end)     // stop (empty or done)
/// take:   AnyString
///         Alternative(more) ; Jump(end)     // this component was the last
/// more:   Separator ; Jump(start)           // more components follow
/// end:
/// ```
///
/// `ComponentBoundary` ensures `foo/**` matches `foo` and `foo/bar` but not
/// `foobar` (leftover bytes in the same component after `Literal "foo"`).
fn append_terminal_recurse_gadget(out: &mut Program) -> anyhow::Result<()> {
    out.instructions.push(Instruction::ComponentBoundary);
    let start = out.here();
    out.instructions.push(Instruction::Alternative(start + 2)); // take
    out.instructions.push(Instruction::Jump(start + 7)); // end
    // take:
    out.instructions.push(Instruction::AnyString);
    out.instructions.push(Instruction::Alternative(start + 5)); // more
    out.instructions.push(Instruction::Jump(start + 7)); // end after last component
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
/// Also true for alternatives where **every** choice is trailing-recursive
/// (e.g. `{**}`, `foo/{**}`, `{a/**,b/**}`), but not mixed choices like
/// `{**,README.md}` so non-directory matches are still emitted.
fn pattern_is_trailing_recursive(nodes: &[AstNode]) -> bool {
    let mut end = nodes.len();
    while end > 0 && matches!(&nodes[end - 1], AstNode::Separator) {
        end -= 1;
    }
    if end == 0 {
        return false;
    }
    match &nodes[end - 1] {
        AstNode::Recurse => true,
        AstNode::Alternatives { choices } => {
            !choices.is_empty()
                && choices
                    .iter()
                    .all(|choice| pattern_is_trailing_recursive(&choice.nodes))
        }
        _ => false,
    }
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

fn append_repeat(out: &mut Program, min: u32, max: u32, pattern: &Pattern) -> anyhow::Result<()> {
    if out.counters == u16::MAX {
        anyhow::bail!("Exceeded the number of repeats allowed in a glob pattern");
    }

    let counter_id = CounterId(out.counters);
    out.counters += 1;

    let start = out.here();

    // This is the loop start - increase the counter
    out.instructions.push(Instruction::Increment(counter_id));

    append_nodes(out, &pattern.nodes)?;

    // If we have less than the minimum, another loop is required
    out.instructions
        .push(Instruction::BranchIfLessThan(start, counter_id, min));

    if max > min {
        // If the counter is still below the maximum, set up an alternative with the start of the
        // loop
        let here = out.here();
        out.instructions
            .push(Instruction::BranchIfLessThan(here + 2, counter_id, max));
        out.instructions.push(Instruction::Jump(here + 3));
        out.instructions.push(Instruction::Alternative(start));
    }

    Ok(())
}

pub fn compile(pattern: &Pattern) -> anyhow::Result<Program> {
    let mut program = Program::default();
    append_nodes(&mut program, pattern.nodes.as_slice())?;
    program.instructions.push(Instruction::Complete);
    program.trailing_recursive = pattern_is_trailing_recursive(pattern.nodes.as_slice());
    Ok(program)
}

pub fn compile_with_options(pattern: &Pattern, case_insensitive: bool) -> anyhow::Result<Program> {
    let mut program = compile(pattern)?;
    program.case_insensitive = case_insensitive;
    Ok(program)
}
