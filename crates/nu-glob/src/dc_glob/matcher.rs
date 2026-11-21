use super::compiler::{Instruction, Program, ProgramOffset};
use super::parser::CharacterClass;
use std::{
    collections::HashSet,
    iter::Peekable,
    path::{Component, Components, Path},
};

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

const ORDINARY_COMPONENT: u8 = 1;
const HIDDEN_DIRECTORY: u8 = 2;
const RESUMED_MATCH_STEP_BUDGET: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MatchPosition {
    instruction: usize,
    // An explicit component can start a new walk even if its suffix shares
    // instructions with a recursive walk that was already in progress.
    finite_depth: usize,
}

#[derive(Hash, PartialEq, Eq)]
struct MatchState {
    instruction: usize,
    depth: usize,
    remaining_bytes: Option<usize>,
    fresh_string: bool,
    finite_depth: usize,
    skip_hidden_child: bool,
}

/// The instructions that can consume the next component below a directory,
/// with flags for which kinds of child each instruction can accept.
#[derive(Clone)]
pub(super) struct Continuations(Vec<(MatchPosition, u8)>);

impl Continuations {
    pub(super) fn subtract(&mut self, covered: &Self) {
        self.0.retain_mut(|(position, kinds)| {
            if let Ok(index) = covered
                .0
                .binary_search_by_key(position, |&(position, _)| position)
            {
                *kinds &= !covered.0[index].1;
            }
            *kinds != 0
        });
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug)]
enum NextString<'a, 'b> {
    Normal(&'a mut &'b [u8]),
    NotNormal,
    EndOfInput,
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
    skip_hidden_child: bool,
    depth: usize,
    finite_depth: usize,
}

impl<'a> ProgramState<'a> {
    fn new(path_components: Components<'a>) -> ProgramState<'a> {
        ProgramState {
            pc: ProgramOffset(0),
            path_components: path_components.peekable(),
            current_string: None,
            fresh_string: false,
            skip_hidden_child: false,
            depth: 0,
            finite_depth: 0,
        }
    }

    fn next_component<const TRACK: bool, const FINITE: bool>(&mut self) -> Option<Component<'a>> {
        let component = self.path_components.next();
        if TRACK && component.is_some() {
            self.depth += 1;
            if FINITE {
                self.finite_depth = self.depth;
            }
        }
        component
    }

    fn next_string<const TRACK: bool, const FINITE: bool>(&mut self) -> NextString<'_, 'a> {
        if self.current_string.is_none() {
            match self.path_components.peek() {
                Some(Component::Normal(_)) => {
                    let Some(Component::Normal(name)) = self.next_component::<TRACK, FINITE>()
                    else {
                        unreachable!()
                    };
                    self.fresh_string = true;
                    return NextString::Normal(self.current_string.insert(name.as_encoded_bytes()));
                }
                Some(_) => return NextString::NotNormal,
                None => return NextString::EndOfInput,
            }
        }
        match self.current_string.as_mut() {
            Some(string) => NextString::Normal(string),
            None => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
struct Matcher<'a, const COLLECT_CONTINUATIONS: bool> {
    state: ProgramState<'a>,
    alternatives: Vec<ProgramState<'a>>,
    result: MatchResult,
    /// Whether the path's last component is a directory. The components before it are.
    last_is_dir: bool,
    continuations: Vec<(MatchPosition, u8)>,
}

impl<'a, const COLLECT_CONTINUATIONS: bool> Matcher<'a, COLLECT_CONTINUATIONS> {
    fn new(path: &'a Path, last_is_dir: bool) -> Self {
        Self {
            state: ProgramState::new(path.components()),
            alternatives: vec![],
            result: MatchResult {
                valid_as_prefix: false,
                valid_as_complete_match: false,
            },
            last_is_dir,
            continuations: vec![],
        }
    }

    fn advance<const TRACK_POSITION: bool>(&mut self, program: &Program) -> bool {
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
                    // Continuation collection assumes a child component will
                    // follow. Record the instruction that consumes it, so a
                    // recursive jump back to the same instruction compares equal.
                    if COLLECT_CONTINUATIONS || self.state.path_components.peek().is_some() {
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
                match self.state.next_component::<TRACK_POSITION, true>() {
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
            Instruction::RootDir if !self.has_string() => {
                match self.state.next_component::<TRACK_POSITION, true>() {
                    Some(Component::RootDir) => self.next(),
                    Some(_) => self.try_alternative(),
                    None => self.end_of_input(),
                }
            }
            Instruction::CurDir if !self.has_string() => {
                // Path components omit `.` except at the start of a relative
                // path, and stripping the traversal base removes even that.
                if matches!(self.state.path_components.peek(), Some(Component::CurDir)) {
                    self.state.next_component::<TRACK_POSITION, true>();
                }
                self.state.current_string = Some(&[]);
                self.state.fresh_string = false;
                self.next()
            }
            Instruction::ParentDir if !self.has_string() => {
                match self.state.next_component::<TRACK_POSITION, true>() {
                    Some(Component::ParentDir) => {
                        // The following separator requires a consumed component,
                        // just as it does after a literal file or directory name.
                        self.state.current_string = Some(&[]);
                        self.state.fresh_string = false;
                        self.next()
                    }
                    Some(_) => self.try_alternative(),
                    None => self.end_of_input(),
                }
            }
            Instruction::LiteralString(bytes) => {
                match self.state.next_string::<TRACK_POSITION, true>() {
                    NextString::Normal(current_string)
                        if current_string.starts_with(&bytes[..])
                            || (program.case_insensitive
                                && current_string.len() >= bytes.len()
                                && current_string[..bytes.len()]
                                    .eq_ignore_ascii_case(&bytes[..])) =>
                    {
                        *current_string = &current_string[bytes.len()..];
                        self.state.fresh_string = false;
                        self.next()
                    }
                    NextString::Normal(_) | NextString::NotNormal => self.try_alternative(),
                    NextString::EndOfInput => self.end_of_input(),
                }
            }
            Instruction::AnyCharacter => {
                let fresh_string = self.state.fresh_string || self.state.current_string.is_none();
                match self.state.next_string::<TRACK_POSITION, true>() {
                    NextString::Normal(current_string) => {
                        if program.literal_leading_dot && fresh_string && is_hidden(current_string)
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
                match self.state.next_string::<TRACK_POSITION, false>() {
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
                let fresh_string = self.state.fresh_string || self.state.current_string.is_none();
                match self.state.next_string::<TRACK_POSITION, true>() {
                    NextString::Normal(current_string) => {
                        if program.literal_leading_dot && fresh_string && is_hidden(current_string)
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
                    // Alternatives can reach the same instruction with different
                    // requirements on the next directory's visibility.
                    if COLLECT_CONTINUATIONS
                        && program.skip_hidden_in_recursion
                        && self.state.current_string.is_none()
                        && self.state.path_components.peek().is_none()
                    {
                        self.state.skip_hidden_child = true;
                    }
                    self.next()
                }
            }
            Instruction::Complete => self.complete(),
            _ => self.try_alternative(),
        }
    }

    fn advance_unique(&mut self, program: &Program, visited: &mut HashSet<MatchState>) -> bool {
        loop {
            // Each matcher has a fixed input. The component index and remaining
            // byte count identify its cursor, with None marking a component boundary.
            let state = MatchState {
                instruction: self.state.pc.0,
                depth: self.state.depth,
                remaining_bytes: self.state.current_string.map(<[u8]>::len),
                fresh_string: self.state.fresh_string,
                finite_depth: if COLLECT_CONTINUATIONS {
                    self.state.finite_depth
                } else {
                    0
                },
                skip_hidden_child: self.state.skip_hidden_child,
            };
            if visited.insert(state) {
                return self.advance::<true>(program);
            }
            if !self.try_alternative() {
                return false;
            }
        }
    }

    fn can_stop(&self) -> bool {
        !COLLECT_CONTINUATIONS && self.result.valid_as_prefix && self.result.valid_as_complete_match
    }

    /// Finish every alternative at one input depth before advancing to the next.
    fn run_by_component(&mut self, program: &Program) {
        let mut visited = HashSet::new();
        let mut next_component = Vec::new();
        loop {
            let depth = self.state.depth;
            while self.advance_unique(program, &mut visited) {
                if self.can_stop() {
                    return;
                }
                if self.state.depth > depth {
                    // A step consumes at most one component. Park the state so
                    // the remaining alternatives can finish the current depth.
                    next_component.push(self.state.clone());
                    if !self.try_alternative() {
                        break;
                    }
                }
            }

            // Forward transitions cannot return to an exhausted depth.
            std::mem::swap(&mut self.alternatives, &mut next_component);
            if !self.try_alternative() {
                return;
            }
            visited.clear();
        }
    }

    fn resume_from(&mut self, continuations: &Continuations) -> bool {
        let kind = if self.next_is_hidden_dir() {
            HIDDEN_DIRECTORY
        } else {
            ORDINARY_COMPONENT
        };
        self.alternatives.extend(
            continuations
                .0
                .iter()
                .filter(|&&(_, kinds)| kinds & kind != 0)
                .map(|&(position, _)| ProgramState {
                    pc: ProgramOffset(position.instruction),
                    finite_depth: position.finite_depth,
                    ..self.state.clone()
                }),
        );
        self.try_alternative()
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
        if COLLECT_CONTINUATIONS {
            let kinds = if self.state.skip_hidden_child {
                ORDINARY_COMPONENT
            } else {
                ORDINARY_COMPONENT | HIDDEN_DIRECTORY
            };
            self.continuations.push((
                MatchPosition {
                    instruction: self.state.pc.0,
                    finite_depth: self.state.finite_depth,
                },
                kinds,
            ));
        }
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
    let mut matcher = Matcher::<false>::new(path, last_is_dir);
    while !matcher.can_stop() && matcher.advance::<false>(program) {}
    matcher.result
}

pub(super) fn path_matches_from(
    path: &Path,
    last_is_dir: bool,
    program: &Program,
    start: &Continuations,
) -> MatchResult {
    let mut matcher = Matcher::<false>::new(path, last_is_dir);
    if !matcher.resume_from(start) {
        return matcher.result;
    }
    // A child of a resumed directory often has only one short component to
    // match. Avoid allocating a visited set until the match needs more work.
    for _ in 0..RESUMED_MATCH_STEP_BUDGET {
        if !matcher.advance::<false>(program) || matcher.can_stop() {
            return matcher.result;
        }
    }

    // Restart from the checkpoint so the component walk starts with all
    // alternatives at the same input depth.
    let mut matcher = Matcher::<false>::new(path, last_is_dir);
    if matcher.resume_from(start) {
        matcher.run_by_component(program);
    }
    matcher.result
}

pub(super) fn continuation_positions(
    path: &Path,
    program: &Program,
    start: Option<(&Continuations, usize)>,
) -> Continuations {
    if let Some((start, _)) = start
        && path.as_os_str().is_empty()
    {
        return start.clone();
    }
    let mut matcher = Matcher::<true>::new(path, true);
    if start.is_none_or(|(start, depth)| {
        matcher.state.depth = depth;
        matcher.resume_from(start)
    }) {
        matcher.run_by_component(program);
    }
    matcher
        .continuations
        .sort_unstable_by_key(|&(position, _)| position);
    matcher.continuations.dedup_by(|later, earlier| {
        if later.0 == earlier.0 {
            earlier.1 |= later.1;
            true
        } else {
            false
        }
    });
    Continuations(matcher.continuations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dc_glob::{compiler, parser};

    #[test]
    fn continuation_collection_shares_equivalent_alternatives() {
        let pattern = format!("{}/**", ["{a,a}"; 16].join("/"));
        let program = compiler::compile(&parser::parse(&pattern)).expect("compile pattern");
        let path = ["a"; 16].join("/");
        let mut matcher = Matcher::<true>::new(Path::new(&path), true);
        let mut visited = HashSet::new();
        let mut expanded = 0;
        while matcher.advance_unique(&program, &mut visited) {
            expanded += 1;
            // The alternatives share a prefix of the same input at every join.
            assert!(expanded <= program.instructions.len());
        }
        assert_eq!(matcher.continuations.len(), 1);
    }

    #[test]
    fn resumed_match_finishes_after_the_step_budget() {
        let pattern = format!("{}/marker.txt", ["{a,a}"; 16].join("/"));
        let program = compiler::compile(&parser::parse(&pattern)).expect("compile pattern");
        let start = continuation_positions(Path::new(""), &program, None);
        let directories = ["a"; 16].join("/");

        for (name, complete) in [("marker.txt", true), ("unmatched.txt", false)] {
            let path = Path::new(&directories).join(name);
            let mut matcher = Matcher::<false>::new(&path, false);
            assert!(matcher.resume_from(&start));
            for _ in 0..RESUMED_MATCH_STEP_BUDGET {
                assert!(matcher.advance::<false>(&program) && !matcher.can_stop());
            }

            let matches = path_matches_from(&path, false, &program, &start);
            assert_eq!(matches.valid_as_complete_match, complete);
            assert!(!matches.valid_as_prefix);
        }
    }
}
