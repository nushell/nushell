use super::compiler::Program;
use super::matcher::{MatchResult, path_matches};
use crate::{MatchOptions, Pattern};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SendError, SyncSender, sync_channel};

use anyhow::anyhow;

// Spawn Rayon tasks near the top of the tree, then recurse inline to reduce
// scheduler overhead for deeply nested directory-heavy traversals.
const PARALLEL_DEPTH_CUTOFF: usize = 2;

#[derive(Debug, Clone)]
pub(crate) enum RecursiveFastPath {
    Suffix(Box<[u8]>),
    BasenamePattern(Box<[BasenameToken]>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BasenameToken {
    Literal(Box<[u8]>),
    Wildcard,
    AnyCharacter,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TraversalOptions {
    pub max_depth: Option<usize>,
    pub follow_symlinks: bool,
}

/// An interrupt flag that can be set to stop an in-progress glob traversal.
///
/// When the flag is set to `true` the background rayon thread will stop spawning
/// new work and close the channel, which causes the consumer iterator to drain and
/// end naturally.
#[derive(Debug, Clone, Default)]
pub struct InterruptFlag(pub Option<Arc<AtomicBool>>);

impl InterruptFlag {
    /// Return `true` if the interrupt has been signalled.
    #[inline]
    pub fn is_set(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }
}

pub fn glob(
    relative_to: impl Into<PathBuf>,
    program: Arc<Program>,
    fast_path: Option<RecursiveFastPath>,
) -> impl Iterator<Item = anyhow::Result<PathBuf>> + Send {
    glob_with_options(
        relative_to,
        program,
        TraversalOptions::default(),
        vec![],
        InterruptFlag::default(),
        fast_path,
    )
}

pub fn glob_with_options(
    relative_to: impl Into<PathBuf>,
    program: Arc<Program>,
    options: TraversalOptions,
    exclude_patterns: Vec<Pattern>,
    interrupt: InterruptFlag,
    fast_path: Option<RecursiveFastPath>,
) -> impl Iterator<Item = anyhow::Result<PathBuf>> + Send {
    let (tx, rx) = sync_channel(4096);

    let base_dir = relative_to.into();
    let traversal_base = program
        .absolute_prefix
        .clone()
        .unwrap_or_else(|| base_dir.clone());
    let current_dir = traversal_start_dir(&program, &traversal_base);

    rayon::spawn(move || {
        // For relative programs we can start traversal at a static prefix, but we
        // still emit and match paths relative to the original base directory.
        let output_relative_to = if program.absolute_prefix.is_some() {
            &current_dir
        } else {
            base_dir.as_path()
        };
        let match_relative_to = if program.absolute_prefix.is_some() {
            Path::new("")
        } else {
            output_relative_to
        };
        let needs_parent_probe = program
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, super::compiler::Instruction::ParentDir));

        let state = WalkState {
            tx: &tx,
            interrupt: &interrupt,
            output_relative_to,
            match_relative_to,
            options,
            exclude_patterns: &exclude_patterns,
            has_excludes: !exclude_patterns.is_empty(),
            needs_parent_probe,
            program: &program,
            fast_path: fast_path.as_ref(),
        };

        glob_to(&current_dir, 0, state)
    });
    rx.into_iter()
}

fn traversal_start_dir(program: &Program, base: &Path) -> PathBuf {
    let mut out = base.to_path_buf();
    let mut i = 0;
    while i < program.instructions.len() {
        match &program.instructions[i] {
            // Already represented in `base`
            super::compiler::Instruction::RootDir | super::compiler::Instruction::Prefix(_) => {
                i += 1;
            }
            super::compiler::Instruction::Separator => {
                i += 1;
            }
            // Collect literal path components until the first dynamic token.
            super::compiler::Instruction::LiteralString(bytes) => {
                let component = String::from_utf8_lossy(bytes);
                if component.is_empty() {
                    break;
                }
                out.push(component.as_ref());
                i += 1;
            }
            super::compiler::Instruction::CurDir => {
                i += 1;
            }
            super::compiler::Instruction::ParentDir => {
                out.pop();
                i += 1;
            }
            _ => break,
        }
    }

    out
}

fn glob_to(target: &Path, depth: usize, state: WalkState<'_>) {
    match fs::read_dir(target) {
        Ok(results) => rayon::scope(|scope| -> Result<(), SendError<_>> {
            // Parent-path probing is only needed for patterns containing `..`.
            if depth > 0 && state.needs_parent_probe {
                if state.interrupt.is_set() {
                    return Ok(());
                }
                let parent_path = target.join("..");

                handle_path_candidate(&parent_path, None, depth, true, state, scope)?;
            }

            // All of the real results from the directory listing
            for result in results {
                if state.interrupt.is_set() {
                    break;
                }
                match result {
                    Ok(dir_entry) => {
                        let dir_entry_path = dir_entry.path();
                        let file_type = match dir_entry.file_type() {
                            Ok(file_type) => file_type,
                            Err(err) => {
                                let wrapped_err = anyhow!("{}: {}", dir_entry_path.display(), err);
                                state.tx.send(Err(wrapped_err))?;
                                continue;
                            }
                        };

                        handle_path_candidate(
                            &dir_entry_path,
                            Some(file_type),
                            depth,
                            false,
                            state,
                            scope,
                        )?;
                    }
                    Err(err) => {
                        let wrapped_err = anyhow!("{}: {}", target.display(), err);
                        state.tx.send(Err(wrapped_err))?;
                    }
                }
            }

            Ok(())
        })
        .unwrap_or(()),
        Err(err) => {
            let wrapped_err = anyhow!("{}: {}", target.display(), err);
            let _ = state.tx.send(Err(wrapped_err));
        }
    }
}

#[derive(Clone, Copy)]
struct WalkState<'a> {
    tx: &'a SyncSender<anyhow::Result<PathBuf>>,
    interrupt: &'a InterruptFlag,
    output_relative_to: &'a Path,
    match_relative_to: &'a Path,
    options: TraversalOptions,
    exclude_patterns: &'a [Pattern],
    has_excludes: bool,
    needs_parent_probe: bool,
    program: &'a Program,
    fast_path: Option<&'a RecursiveFastPath>,
}

#[inline]
fn basename_length_of_first_char(string: &[u8]) -> Option<usize> {
    string.utf8_chunks().next().map(|chunk| {
        chunk
            .valid()
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1)
    })
}

fn basename_pattern_matches(name: &[u8], tokens: &[BasenameToken]) -> bool {
    let mut name_index = 0usize;
    let mut token_index = 0usize;
    let mut star_state = None;

    while name_index < name.len() {
        match tokens.get(token_index) {
            Some(BasenameToken::Literal(literal)) if name[name_index..].starts_with(literal) => {
                name_index += literal.len();
                token_index += 1;
            }
            Some(BasenameToken::AnyCharacter) => {
                let Some(length) = basename_length_of_first_char(&name[name_index..]) else {
                    return false;
                };
                name_index += length;
                token_index += 1;
            }
            Some(BasenameToken::Wildcard) => {
                star_state = Some((token_index, name_index));
                token_index += 1;
            }
            _ => {
                let Some((star_token_index, star_name_index)) = star_state else {
                    return false;
                };
                let Some(length) = basename_length_of_first_char(&name[star_name_index..]) else {
                    return false;
                };
                let next_name_index = star_name_index + length;
                star_state = Some((star_token_index, next_name_index));
                token_index = star_token_index + 1;
                name_index = next_name_index;
            }
        }
    }

    while matches!(tokens.get(token_index), Some(BasenameToken::Wildcard)) {
        token_index += 1;
    }

    token_index == tokens.len()
}

#[inline]
fn recursive_fast_path_match(
    path: &Path,
    fast_path: &RecursiveFastPath,
    is_parent: bool,
) -> MatchResult {
    let valid_as_complete_match = !is_parent
        && path.file_name().is_some_and(|name| match fast_path {
            RecursiveFastPath::Suffix(suffix) => name.as_encoded_bytes().ends_with(suffix),
            RecursiveFastPath::BasenamePattern(tokens) => {
                basename_pattern_matches(name.as_encoded_bytes(), tokens)
            }
        });

    MatchResult {
        // For recursive basename-pattern fast paths, every directory is a valid
        // prefix because deeper descendants can still satisfy the file-name match.
        valid_as_prefix: true,
        valid_as_complete_match,
    }
}

fn handle_path_candidate<'a>(
    path: &Path,
    file_type: Option<fs::FileType>,
    depth: usize,
    is_parent: bool,
    state: WalkState<'a>,
    scope: &rayon::Scope<'a>,
) -> Result<(), SendError<anyhow::Result<PathBuf>>> {
    let output_relative_to = state.output_relative_to;
    let match_relative_to = state.match_relative_to;
    let options = state.options;
    let exclude_patterns = state.exclude_patterns;
    let has_excludes = state.has_excludes;
    let program = state.program;
    let fast_path = state.fast_path;

    let output_path_candidate = path.strip_prefix(output_relative_to).unwrap_or(path);
    let match_path_candidate = path.strip_prefix(match_relative_to).unwrap_or(path);
    let candidate_depth = if is_parent {
        depth.saturating_sub(1)
    } else {
        depth + 1
    };

    let result = if let Some(fast_path) = fast_path {
        recursive_fast_path_match(match_path_candidate, fast_path, is_parent)
    } else {
        path_matches(match_path_candidate, program)
    };
    let match_options = MatchOptions::default();

    let excluded = has_excludes
        && exclude_patterns
            .iter()
            .any(|exclude| exclude.matches_path_with(match_path_candidate, match_options));

    let file_type = match file_type {
        Some(file_type) => file_type,
        None => match fs::symlink_metadata(path) {
            Ok(metadata) => metadata.file_type(),
            Err(err) => {
                let wrapped_err = anyhow!("{}: {}", path.display(), err);
                state.tx.send(Err(wrapped_err))?;
                return Ok(());
            }
        },
    };

    let is_symlink = file_type.is_symlink();
    let is_dir = if is_symlink {
        options.follow_symlinks && fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        file_type.is_dir()
    };

    let should_prune = if is_dir && has_excludes {
        let probe_candidate = match_path_candidate.join("__dc_glob_probe__");
        excluded
            || exclude_patterns
                .iter()
                .any(|exclude| exclude.matches_path_with(&probe_candidate, match_options))
    } else {
        excluded
    };

    // If it is a valid prefix and a dir, recurse
    if is_dir
        && !is_parent
        && !should_prune
        && result.valid_as_prefix
        && options.max_depth.is_none_or(|max| candidate_depth < max)
    {
        let recurse_path = path.to_owned();
        if candidate_depth <= PARALLEL_DEPTH_CUTOFF {
            scope.spawn(move |_| glob_to(&recurse_path, candidate_depth, state));
        } else {
            glob_to(&recurse_path, candidate_depth, state);
        }
    }

    // If it is valid as a complete match, send it out
    if result.valid_as_complete_match
        && !excluded
        && options.max_depth.is_none_or(|max| candidate_depth <= max)
    {
        state.tx.send(Ok(output_path_candidate.to_owned()))?;
    }

    Ok(())
}
