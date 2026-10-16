//! Completion regressions for cursor normalization.

use super::support::completions_helpers::new_engine_helper;
use nu_cli::CompletionEngine;

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
