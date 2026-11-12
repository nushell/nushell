use super::compiler::{Instruction, Program, ProgramOffset};
use super::parser::CharacterClass;
use std::{
    iter::Peekable,
    path::{Component, Components, Path},
};

// #[cfg(test)]
// mod tests;

#[derive(Debug, Clone, Copy)]
pub struct MatchResult {
    /// True if the match could be made valid with more path components.
    ///
    /// A complete match may not be valid as a prefix if there's no way the pattern could accept
    /// any more path components.
    pub valid_as_prefix: bool,
    /// True if the match is completely valid as a match for the glob.
    pub valid_as_complete_match: bool,
}

#[derive(Debug)]
enum NextString<'a, 'b> {
    Normal(&'a mut &'b [u8]),
    NotNormal,
    EndOfInput,
}

fn next_string<'a, 'b>(
    path_components: &mut Peekable<Components<'b>>,
    current_string: &'a mut Option<&'b [u8]>,
    fresh_string: &mut bool,
) -> NextString<'a, 'b> {
    match current_string {
        Some(s) => NextString::Normal(s),
        None => {
            if let Some(component) = path_components.peek() {
                match component {
                    Component::Normal(_) => {
                        let Some(Component::Normal(normal_str)) = path_components.next() else {
                            unreachable!()
                        };
                        *fresh_string = true;
                        NextString::Normal(current_string.insert(normal_str.as_encoded_bytes()))
                    }
                    _ => NextString::NotNormal,
                }
            } else {
                NextString::EndOfInput
            }
        }
    }
}

/// Whether a path component that starts with `name` is hidden, that is, starts with `.`.
pub(super) fn is_hidden(name: &[u8]) -> bool {
    name.first() == Some(&b'.')
}

fn length_of_first_char(string: &[u8]) -> Option<usize> {
    string.utf8_chunks().next().map(|chunk| {
        chunk
            .valid()
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1)
    })
}

fn char_matches_classes(ch: char, classes: &[CharacterClass], case_insensitive: bool) -> bool {
    if classes.iter().any(|class| match class {
        CharacterClass::Single(single) => *single == ch,
        CharacterClass::Range(start, end) => *start <= ch && ch <= *end,
    }) {
        return true;
    }
    if case_insensitive {
        let swapped = if ch.is_ascii_lowercase() {
            ch.to_ascii_uppercase()
        } else if ch.is_ascii_uppercase() {
            ch.to_ascii_lowercase()
        } else {
            return false;
        };
        classes.iter().any(|class| match class {
            CharacterClass::Single(single) => *single == swapped,
            CharacterClass::Range(start, end) => *start <= swapped && swapped <= *end,
        })
    } else {
        false
    }
}

#[derive(Debug, Clone)]
struct ProgramState<'a> {
    pc: ProgramOffset,
    path_components: Peekable<Components<'a>>,
    current_string: Option<&'a [u8]>,
    fresh_string: bool,
}

impl<'a> ProgramState<'a> {
    fn new(path_components: Components<'a>) -> ProgramState<'a> {
        ProgramState {
            pc: ProgramOffset(0),
            path_components: path_components.peekable(),
            current_string: None,
            fresh_string: false,
        }
    }
}

#[derive(Debug, Clone)]
struct Matcher<'a> {
    state: ProgramState<'a>,
    alternatives: Vec<ProgramState<'a>>,
    result: MatchResult,
    /// Whether the path's last component is a directory. The components before it are.
    last_is_dir: bool,
}

impl<'a> Matcher<'a> {
    fn advance(&mut self, program: &Program) -> bool {
        let instruction = &program.instructions[self.state.pc.0];
        match instruction {
            // Separator between path components. Require that the previous
            // component was opened and fully consumed (`Some(b"")`). A bare
            // `None` means no component was bound yet — allowing Separator to
            // succeed here used to make `*/*` match a single path segment by
            // combining an empty `*` with a free Separator (issue #18600).
            //
            // At EOF after a finished component we do **not** advance the PC:
            // that would let a following empty `*` complete-match (regressing
            // #18600). Prefixed trailing `**` (`foo/**`) is handled by the
            // compiler, which compiles the Separator before a terminal recurse
            // to `ComponentEnd`, so path `foo` can complete without a Separator.
            Instruction::Separator if !self.has_string() => match self.state.current_string {
                // Finished matching inside a component; advance to the next path
                // component when one remains.
                Some([]) => {
                    self.state.current_string = None;
                    self.state.fresh_string = false;
                    if self.state.path_components.peek().is_some() {
                        self.next()
                    } else {
                        self.end_of_input()
                    }
                }
                // No finished component — do not invent path segments.
                None => self.try_alternative(),
                // Unreachable under `!has_string()` (non-empty is handled above).
                Some(_) => {
                    debug_assert!(
                        false,
                        "Separator with non-empty current_string under !has_string()"
                    );
                    self.try_alternative()
                }
            },
            // Collapse multiple separators when a new component was just loaded.
            Instruction::Separator if self.state.fresh_string => self.next(),
            Instruction::Prefix(string) if !self.has_string() => {
                match self.state.path_components.next() {
                    Some(Component::Prefix(prefix_component))
                        if prefix_component.as_os_str() == &string[..]
                            || (program.case_insensitive
                                && prefix_component
                                    .as_os_str()
                                    .as_encoded_bytes()
                                    .eq_ignore_ascii_case(string.as_bytes())) =>
                    {
                        self.next()
                    }
                    Some(_) => self.try_alternative(),
                    None => self.end_of_input(),
                }
            }
            Instruction::RootDir if !self.has_string() => match self.state.path_components.next() {
                Some(Component::RootDir) => self.next(),
                Some(_) => self.try_alternative(),
                None => self.end_of_input(),
            },
            Instruction::CurDir if !self.has_string() => {
                // Path components omit `.` except at the start of a relative
                // path, and stripping the traversal base removes even that.
                if matches!(self.state.path_components.peek(), Some(Component::CurDir)) {
                    self.state.path_components.next();
                }
                self.state.current_string = Some(&[]);
                self.state.fresh_string = false;
                self.next()
            }
            Instruction::ParentDir if !self.has_string() => match self.state.path_components.next()
            {
                Some(Component::ParentDir) => {
                    // The following separator requires a consumed component,
                    // just as it does after a literal file or directory name.
                    self.state.current_string = Some(&[]);
                    self.state.fresh_string = false;
                    self.next()
                }
                Some(_) => self.try_alternative(),
                None => self.end_of_input(),
            },
            Instruction::LiteralString(bytes) => match next_string(
                &mut self.state.path_components,
                &mut self.state.current_string,
                &mut self.state.fresh_string,
            ) {
                NextString::Normal(current_string)
                    if current_string.starts_with(&bytes[..])
                        || (program.case_insensitive
                            && current_string.len() >= bytes.len()
                            && current_string[..bytes.len()].eq_ignore_ascii_case(&bytes[..])) =>
                {
                    *current_string = &current_string[bytes.len()..];
                    self.state.fresh_string = false;
                    self.next()
                }
                NextString::Normal(_) | NextString::NotNormal => self.try_alternative(),
                NextString::EndOfInput => self.end_of_input(),
            },
            Instruction::AnyCharacter => {
                match next_string(
                    &mut self.state.path_components,
                    &mut self.state.current_string,
                    &mut self.state.fresh_string,
                ) {
                    NextString::Normal(current_string) => {
                        if program.literal_leading_dot
                            && self.state.fresh_string
                            && is_hidden(current_string)
                        {
                            return self.try_alternative();
                        }
                        // consume the first actual UTF-8 character
                        if let Some(length) = length_of_first_char(current_string) {
                            *current_string = &current_string[length..];
                            self.state.fresh_string = false;
                            self.next()
                        } else {
                            self.try_alternative()
                        }
                    }
                    NextString::NotNormal => self.try_alternative(),
                    NextString::EndOfInput => self.end_of_input(),
                }
            }
            Instruction::AnyString => {
                match next_string(
                    &mut self.state.path_components,
                    &mut self.state.current_string,
                    &mut self.state.fresh_string,
                ) {
                    NextString::Normal(_) => {
                        // consume the entire string
                        self.state.current_string = Some(b"");
                        self.state.fresh_string = false;
                        self.next()
                    }
                    NextString::NotNormal => self.try_alternative(),
                    NextString::EndOfInput => self.end_of_input(),
                }
            }
            Instruction::Characters { classes, negated } => {
                match next_string(
                    &mut self.state.path_components,
                    &mut self.state.current_string,
                    &mut self.state.fresh_string,
                ) {
                    NextString::Normal(current_string) => {
                        if program.literal_leading_dot
                            && self.state.fresh_string
                            && is_hidden(current_string)
                        {
                            return self.try_alternative();
                        }
                        let mut iter = current_string.utf8_chunks();
                        let Some(chunk) = iter.next() else {
                            return self.try_alternative();
                        };

                        let length = match chunk.valid().chars().next() {
                            Some(ch)
                                if char_matches_classes(ch, classes, program.case_insensitive)
                                    != *negated =>
                            {
                                ch.len_utf8()
                            }
                            // An undecodable byte is not a class member; consume it like `?`.
                            None if *negated => 1,
                            _ => return self.try_alternative(),
                        };
                        *current_string = &current_string[length..];
                        self.state.fresh_string = false;
                        self.next()
                    }
                    NextString::NotNormal => self.try_alternative(),
                    NextString::EndOfInput => self.end_of_input(),
                }
            }
            Instruction::Jump(index) => {
                self.state.pc = *index;
                true
            }
            Instruction::Alternative(index) => {
                // Save a snapshot so we can try it later
                self.alternatives.push(ProgramState {
                    pc: *index,
                    ..self.state.clone()
                });
                self.next()
            }
            Instruction::ComponentBoundary => {
                // Require a path-component boundary: no unconsumed bytes in the
                // current component. Clear a finished `Some([])` to `None`.
                if self.has_string() {
                    self.try_alternative()
                } else {
                    self.state.current_string = None;
                    self.state.fresh_string = false;
                    self.next()
                }
            }
            Instruction::ComponentEnd => {
                if self.state.current_string == Some(&[]) {
                    self.state.current_string = None;
                    self.state.fresh_string = false;
                    self.next()
                } else {
                    self.try_alternative()
                }
            }
            Instruction::NotHiddenDir => {
                if program.skip_hidden_in_recursion && self.next_is_hidden_dir() {
                    self.try_alternative()
                } else {
                    self.next()
                }
            }
            Instruction::Complete => self.complete(),
            _ => self.try_alternative(),
        }
    }

    fn has_string(&self) -> bool {
        self.state.current_string.is_some_and(|s| !s.is_empty())
    }

    /// Whether the matcher stands before a path component that is a hidden directory.
    fn next_is_hidden_dir(&mut self) -> bool {
        if self.state.current_string.is_some() {
            return false;
        }
        let hidden = matches!(
            self.state.path_components.peek(),
            Some(Component::Normal(name)) if is_hidden(name.as_encoded_bytes())
        );
        // Every component but the last is a directory, since the path goes on below it.
        hidden && (self.last_is_dir || self.state.path_components.clone().nth(1).is_some())
    }

    fn next(&mut self) -> bool {
        self.state.pc.0 += 1;
        true
    }

    fn try_alternative(&mut self) -> bool {
        if let Some(alternative_state) = self.alternatives.pop() {
            self.state = alternative_state;
            true
        } else {
            false
        }
    }

    fn end_of_input(&mut self) -> bool {
        self.result.valid_as_prefix = true;
        self.try_alternative()
    }

    fn complete(&mut self) -> bool {
        if !self.has_string() && self.state.path_components.next().is_none() {
            self.result.valid_as_complete_match = true;
        }
        self.try_alternative()
    }
}

/// Match `path` against `program`.
///
/// `last_is_dir` says whether the last component of `path` is a directory. Only
/// [`Program::skip_hidden_in_recursion`] reads it, so callers that don't set that flag can pass
/// `false`.
pub fn path_matches(path: &Path, last_is_dir: bool, program: &Program) -> MatchResult {
    let mut matcher = Matcher {
        state: ProgramState::new(path.components()),
        alternatives: vec![],
        result: MatchResult {
            valid_as_prefix: false,
            valid_as_complete_match: false,
        },
        last_is_dir,
    };

    while !matcher.result.valid_as_prefix || !matcher.result.valid_as_complete_match {
        if !matcher.advance(program) {
            break;
        }
    }
    matcher.result
}
