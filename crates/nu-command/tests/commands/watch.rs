use std::time::Duration;

use rstest::rstest;

use nu_protocol::test_table;
use nu_test_support::{fs::Stub, prelude::*};

const STREAM_TIMEOUT: &str = r#"
    # cut off the input stream after `$duration`
    def stream-timeout [wait: duration]: list -> list {
        wrap item
        | append {stop: true}
        | interleave {
            generate {|d|
                sleep $d
                {out: {stop: true}}
            } $wait
        }
        | take while { $in has "item" }
        | get item
    }
"#;

#[rstest]
#[case::within_time(Duration::ZERO, [0, 1, 2, 3, 4])]
#[case::timed_out(Duration::from_millis(100), [0, 1])]
#[cfg_attr(
    target_os = "macos",
    ignore = "anything involving timing is unreliable on macos in CI"
)]
fn stream_timeout(#[case] delay: Duration, #[case] expected: impl IntoValue) -> Result {
    let mut tester = test();
    let () = tester.run(STREAM_TIMEOUT)?;
    let () = tester.run_with_data("let delay = $in", delay)?;

    let code = "
        0..<5
        | each { sleep $delay; $in }
        | stream-timeout 250ms
    ";
    tester.run(code).expect_value_eq(expected)
}

#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "file operations or anything involving timing is unreliable on macos in CI"
)]
fn watch_stream() -> Result {
    Playground::setup("streaming_watch_fs", |dirs, _| {
        let foo_txt = &*dirs.test().join("foo.txt");
        let bar_txt = &*dirs.test().join("bar.txt");

        let code = r#"
            [
                {|| touch foo.txt }
                {|| "meow" | save -f foo.txt }
                {|| mv foo.txt bar.txt }
                {|| rm bar.txt }
            ]
            | each {|fn| null; do $fn; {}}
            | zip { watch . --debounce 200ms --quiet | stream-timeout 5sec }
            | each { into record }
        "#;

        #[cfg(not(target_os = "macos"))]
        let expected = test_table![
            ["operation",  "path", "new_path"];
            [   "Create", foo_txt,         ()],
            [    "Write", foo_txt,         ()],
            [   "Rename", foo_txt,    bar_txt],
            [   "Remove", bar_txt,         ()],
        ];

        // https://github.com/notify-rs/notify/issues/900
        #[cfg(target_os = "macos")]
        let expected = test_table![
            ["operation",  "path", "new_path"];
            [   "Create", foo_txt,         ()],
            [   "Create", foo_txt,         ()],
            [   "Create", bar_txt,         ()],
            [   "Remove", bar_txt,         ()],
        ];

        let mut tester = test().cwd(dirs.test());
        let () = tester.run(STREAM_TIMEOUT)?;
        tester.run(code).expect_value_eq(expected)
    })
}

#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "file operations that involve paths outside the watched directory do not work properly on macos"
)]
fn watch_stream_outside() -> Result {
    Playground::setup("streaming_watch_fs_outside_watched_dir", |dirs, sandbox| {
        sandbox
            .mkdir("watched_dir")
            .with_files(&[Stub::EmptyFile("foo.txt")]);

        let mut foo_txt = dirs.test().to_owned();
        foo_txt.push("watched_dir");
        foo_txt.push("foo.txt");
        let foo_txt = &*foo_txt;

        let code = "
            [
                {|| mv ../foo.txt ./ }
                {|| mv foo.txt ../ }
            ]
            | each {|fn| null; do $fn; {}}
            | zip { watch . --debounce 200ms --quiet | stream-timeout 5sec }
            | each { into record }
        ";

        let expected = test_table![
            ["operation",  "path", "new_path"];
            [   "Rename",      (),    foo_txt],
            [   "Rename", foo_txt,         ()],
        ];

        let mut tester = test().cwd(dirs.test().join("watched_dir"));
        let () = tester.run(STREAM_TIMEOUT)?;
        tester.run(code).expect_value_eq(expected)
    })
}
