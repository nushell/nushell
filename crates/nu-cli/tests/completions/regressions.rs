//! Completion regressions for cursor normalization and command-head replacement.

use super::support::completions_helpers::{new_engine_helper, new_external_engine};
use nu_cli::CompletionEngine;
use nu_protocol::engine::Stack;
use nu_protocol::{SuggestionKind, engine::CommandType};

#[test]
fn file_completion_floors_a_cursor_inside_utf8() {
    let directory = tempfile::tempdir().expect("temporary completion directory");
    std::fs::write(directory.path().join("hello"), "").expect("completion candidate");
    let (_, _, engine_state, stack) = new_engine_helper(
        directory
            .path()
            .to_path_buf()
            .try_into()
            .expect("temporary directory is absolute"),
    );
    let engine = CompletionEngine::new(&engine_state, &stack);
    let source = "ls hé";

    let values = |source: &str, position| {
        engine
            .fetch_completions_within_file("file.nu", position, source)
            .into_iter()
            .map(|item| item.suggestion.value)
            .collect::<Vec<_>>()
    };
    assert!(values(source, 4).contains(&"hello".to_string()));
    assert_eq!(values(source, 5), values(source, 4)); // byte 5 lies inside `é`

    // An oversized cursor clamps to EOF, where a real candidate must still match.
    let source = "ls h";
    assert!(values(source, source.len()).contains(&"hello".to_string()));
    assert_eq!(values(source, usize::MAX), values(source, source.len()));
}

#[test]
fn file_command_completion_preserves_following_arguments() {
    let (_, _, engine_state, stack) = super::support::new_engine();
    let engine = CompletionEngine::new(&engine_state, &stack);
    let source = "fake-cm one";
    let suggestion = engine
        .fetch_completions_within_file("file.nu", 6, source)
        .into_iter()
        .find(|item| item.suggestion.value == "fake-cmd")
        .expect("fake-cmd completion");
    let span = suggestion.suggestion.span;
    assert_eq!((span.start, span.end), (0, 7));
    let mut accepted = source.to_string();
    accepted.replace_range(span.start..span.end, &suggestion.suggestion.value);
    assert_eq!(accepted, "fake-cmd one");

    // The same edit must consume the suffix when no argument follows it.
    let source = "fake-cZZ";
    let suggestion = engine
        .fetch_completions_within_file("file.nu", 6, source)
        .into_iter()
        .find(|item| item.suggestion.value == "fake-cmd")
        .expect("completion in a lone command head");
    assert_eq!(
        (
            suggestion.suggestion.span.start,
            suggestion.suggestion.span.end
        ),
        (0, source.len())
    );

    let source = "fake-cZZ one";
    let suggestion = engine
        .fetch_completions_within_file("file.nu", 6, source)
        .into_iter()
        .find(|item| item.suggestion.value == "fake-cmd")
        .expect("completion matches the typed prefix, not the untyped suffix");
    let span = suggestion.suggestion.span;
    assert_eq!((span.start, span.end), (0, 8));
    let mut accepted = source.to_string();
    accepted.replace_range(span.start..span.end, &suggestion.suggestion.value);
    assert_eq!(accepted, "fake-cmd one");
}

#[test]
fn interactive_multiword_command_still_replaces_the_typed_line() {
    let (_, _, engine_state, stack) = super::support::new_engine();
    let engine = CompletionEngine::new(&engine_state, &stack);
    let source = "config n";
    let suggestion = engine
        .fetch_completions_at(source, source.len())
        .into_iter()
        .find(|item| item.suggestion.value == "config nu")
        .expect("multiword command completion");
    assert_eq!(
        (
            suggestion.suggestion.span.start,
            suggestion.suggestion.span.end
        ),
        (0, source.len())
    );
}

#[test]
fn file_completion_before_a_command_sigil_is_empty() {
    let (_, _, engine_state, stack) = super::support::new_engine();
    let engine = CompletionEngine::new(&engine_state, &stack);
    for (prefix, suffix) in [("", ""), ("print hi; ", ""), ("\n", ""), ("do { ", " }")] {
        for head in ["^", "^ one", "^sle one", "%", "%ls one", "^zzz one"] {
            let source = format!("{prefix}{head}{suffix}");
            assert!(
                engine
                    .fetch_completions_within_file("file.nu", prefix.len(), &source)
                    .is_empty(),
                "unexpected completion before the sigil in {source:?}",
            );
        }
    }
}

#[test]
fn file_completion_after_external_sigil_matches_only_the_typed_prefix() {
    let engine_state = new_external_engine();
    let stack = Stack::new();
    let engine = CompletionEngine::new(&engine_state, &stack);
    let executable = if cfg!(windows) { "sleep.exe" } else { "sleep" };
    for source in ["^slZZ one", "^slZZ"] {
        let suggestions = engine.fetch_completions_within_file("file.nu", 3, source);
        assert_eq!(suggestions.len(), 1, "{source:?}");
        let suggestion = &suggestions[0].suggestion;
        assert_eq!(suggestion.value, executable);
        let span = suggestion.span;
        assert_eq!((span.start, span.end), (1, 5));
        assert!(span.start <= 3 && 3 <= span.end);
        let mut accepted = source.to_string();
        accepted.replace_range(span.start..span.end, &suggestion.value);
        assert_eq!(accepted, source.replace("slZZ", executable));
    }
    assert!(
        engine
            .fetch_completions_within_file("file.nu", 4, "^zzz one")
            .is_empty()
    );
}

#[test]
fn file_completion_after_builtin_sigil_preserves_scope_and_arguments() {
    let (_, _, mut engine_state, mut stack) = super::support::new_engine();
    super::support::merge_input(b"def confiding [] {}", &mut engine_state, &mut stack)
        .expect("custom command sharing the builtin prefix");
    let engine = CompletionEngine::new(&engine_state, &stack);
    let source = "%confiZZ one";
    let suggestions = engine.fetch_completions_within_file("file.nu", 6, source);
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().all(|item| {
        matches!(
            item.kind,
            Some(SuggestionKind::Command(CommandType::Builtin, _))
        ) && item.suggestion.value.starts_with("confi")
            && item.suggestion.span.start <= 6
            && 6 <= item.suggestion.span.end
    }));
    let suggestion = suggestions
        .iter()
        .find(|item| item.suggestion.value == "config nu")
        .expect("builtin subcommand completion");
    let span = suggestion.suggestion.span;
    assert_eq!((span.start, span.end), (1, 8));
    let mut accepted = source.to_string();
    accepted.replace_range(span.start..span.end, &suggestion.suggestion.value);
    assert_eq!(accepted, "%config nu one");
}

#[test]
fn file_completion_for_wrapped_commands_ignores_the_untyped_suffix() {
    let (_, _, engine_state, stack) = super::support::new_engine();
    let engine = CompletionEngine::new(&engine_state, &stack);
    for wrapper in ["sudo", "doas"] {
        for tail in ["", " one"] {
            let source = format!("{wrapper} fake-cZZ{tail}");
            let start = wrapper.len() + 1;
            let cursor = start + "fake-c".len();
            let suggestions = engine.fetch_completions_within_file("file.nu", cursor, &source);
            let suggestion = suggestions
                .iter()
                .find(|item| item.suggestion.value == "fake-cmd")
                .unwrap_or_else(|| panic!("missing wrapped command completion for {source:?}"));
            let span = suggestion.suggestion.span;
            assert_eq!((span.start, span.end), (start, start + "fake-cZZ".len()));
            let mut accepted = source.clone();
            accepted.replace_range(span.start..span.end, &suggestion.suggestion.value);
            assert_eq!(accepted, format!("{wrapper} fake-cmd{tail}"));
        }
    }
}
