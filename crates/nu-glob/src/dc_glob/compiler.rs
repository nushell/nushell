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
        AstNode::Recurse => append_recurse_gadget(out),
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

fn append_recurse_gadget(out: &mut Program) -> anyhow::Result<()> {
    // The recurse gadget involves creating an alternative loop  with AnyString + Separator
    let start = out.here();
    out.instructions.push(Instruction::Alternative(start + 2));
    out.instructions.push(Instruction::Jump(start + 5)); // the non-alternative target
    out.instructions.push(Instruction::AnyString);
    out.instructions.push(Instruction::Separator);
    out.instructions.push(Instruction::Jump(start));
    Ok(())
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
        for node in &choice.nodes {
            append_program(out, node)?;
        }
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

    for node in &pattern.nodes {
        append_program(out, node)?;
    }

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
    for node in &pattern.nodes {
        append_program(&mut program, node)?;
    }
    program.instructions.push(Instruction::Complete);
    Ok(program)
}
