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

fn char_matches_classes(ch: char, classes: &[CharacterClass]) -> bool {
    classes.iter().any(|class| match class {
        CharacterClass::Single(single) => *single == ch,
        CharacterClass::Range(start, end) => *start <= ch && ch <= *end,
    })
}

#[derive(Debug, Clone)]
struct ProgramState<'a> {
    pc: ProgramOffset,
    path_components: Peekable<Components<'a>>,
    current_string: Option<&'a [u8]>,
    fresh_string: bool,
    counters: Vec<u32>,
}

impl<'a> ProgramState<'a> {
    fn new(path_components: Components<'a>, num_counters: u16) -> ProgramState<'a> {
        ProgramState {
            pc: ProgramOffset(0),
            path_components: path_components.peekable(),
            current_string: None,
            fresh_string: false,
            counters: vec![0; num_counters as usize],
        }
    }
}

#[derive(Debug, Clone)]
struct Matcher<'a> {
    state: ProgramState<'a>,
    alternatives: Vec<ProgramState<'a>>,
    result: MatchResult,
}

impl<'a> Matcher<'a> {
    fn advance(&mut self, program: &Program) -> bool {
        match &program.instructions[self.state.pc.0] {
            Instruction::Separator if !self.has_string() => {
                self.state.current_string = None;
                if self.state.path_components.peek().is_some() {
                    self.next()
                } else {
                    self.end_of_input()
                }
            }
            // Collapse multiple separators with no consumption in between
            Instruction::Separator if self.state.fresh_string => self.next(),
            Instruction::Prefix(string) if !self.has_string() => {
                match self.state.path_components.next() {
                    Some(Component::Prefix(prefix_component))
                        if prefix_component.as_os_str() == &string[..] =>
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
            Instruction::CurDir if !self.has_string() => match self.state.path_components.next() {
                Some(Component::CurDir) => self.next(),
                Some(_) => self.try_alternative(),
                None => self.end_of_input(),
            },
            Instruction::ParentDir if !self.has_string() => match self.state.path_components.next()
            {
                Some(Component::ParentDir) => self.next(),
                Some(_) => self.try_alternative(),
                None => self.end_of_input(),
            },
            Instruction::LiteralString(bytes) => match next_string(
                &mut self.state.path_components,
                &mut self.state.current_string,
                &mut self.state.fresh_string,
            ) {
                NextString::Normal(current_string) if current_string.starts_with(&bytes[..]) => {
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
            Instruction::Characters(classes) => {
                match next_string(
                    &mut self.state.path_components,
                    &mut self.state.current_string,
                    &mut self.state.fresh_string,
                ) {
                    NextString::Normal(current_string) => {
                        let mut iter = current_string.utf8_chunks();
                        let Some(chunk) = iter.next() else {
                            return self.try_alternative();
                        };

                        let mut chars = chunk.valid().chars();
                        let Some(ch) = chars.next() else {
                            return self.try_alternative();
                        };

                        if char_matches_classes(ch, classes) {
                            *current_string = &current_string[ch.len_utf8()..];
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
            Instruction::Increment(counter_id) => {
                self.state.counters[counter_id.0 as usize] += 1;
                self.next()
            }
            Instruction::BranchIfLessThan(index, counter_id, value) => {
                if self.state.counters[counter_id.0 as usize] < *value {
                    self.state.pc = *index;
                    true
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

pub fn path_matches(path: &Path, program: &Program) -> MatchResult {
    let mut matcher = Matcher {
        state: ProgramState::new(path.components(), program.counters),
        alternatives: vec![],
        result: MatchResult {
            valid_as_prefix: false,
            valid_as_complete_match: false,
        },
    };

    while !matcher.result.valid_as_prefix || !matcher.result.valid_as_complete_match {
        if !matcher.advance(program) {
            break;
        }
    }
    matcher.result
}
