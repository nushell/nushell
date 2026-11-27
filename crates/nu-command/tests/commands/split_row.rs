use nu_protocol::{ByteStream, ByteStreamType, PipelineData, PipelineMetadata, Signals, Span};
use nu_test_support::prelude::*;
use rstest::rstest;
use std::sync::{Arc, atomic::AtomicBool, mpsc};

#[test]
fn split_row() -> Result {
    let sample = "importer,shipper,tariff_item,name,origin";
    let code = r#"$in | split row "," | length"#;
    test().run_with_data(code, sample).expect_value_eq(5)
}

#[test]
fn split_row_number() -> Result {
    let sample = "importer,shipper,tariff_item,name,origin";

    let code = r#"$in | split row -n 3 ",""#;
    test().run_with_data(code, sample).expect_value_eq(vec![
        "importer",
        "shipper",
        "tariff_item,name,origin",
    ])?;

    let code = r#"$in | split row -n 3 --right ",""#;
    test().run_with_data(code, sample).expect_value_eq(vec![
        "importer,shipper,tariff_item",
        "name",
        "origin",
    ])
}

#[test]
fn split_row_number_zero() -> Result {
    let code = r#""x" | split row -n 0 ",""#;
    test().run(code).expect_value_eq::<Vec<Value>>(vec![])?;
    let code = r#""x" | split row -n 0 --right ",""#;
    test().run(code).expect_value_eq::<Vec<Value>>(vec![])?;
    Ok(())
}

#[test]
fn split_row_number_error() -> Result {
    let code = r#""x" | split row -n -1 ",""#;
    test().run(code).expect_shell_error()?;
    let code = r#""x" | split row -n -1 --right ",""#;
    test().run(code).expect_shell_error()?;
    Ok(())
}

#[test]
fn to_row_no_sep_found() -> Result {
    let sample = "stuff";

    let code = r#"$in | split row "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(vec![sample])?;

    let code = r#"$in | split row -n 3 "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(vec![sample])?;

    let code = r#"$in | split row -n 3 --right "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(vec![sample])
}

#[test]
fn split_row_regex() -> Result {
    let sample = "importer      ,   shipper      ,  tariff_item,name      ,    origin";
    let code = r#"$in | split row -r '\s*,\s*' | length"#;
    test().run_with_data(code, sample).expect_value_eq(5)
}

#[test]
fn split_row_type() -> Result {
    let code = r#"
        def foo [a: list<string>] {
            $a | describe
        }

        foo (["a b", "c d"] | split row " ")
    "#;
    test().run(code).expect_value_eq("list<string>")
}

#[test]
fn byte_stream_yields_rows_before_eof() -> Result {
    let span = Span::new(10, 20);
    let caller_span = Span::new(25, 35);
    let metadata = PipelineMetadata::default().with_content_type(Some("text/plain".into()));
    let (release, gate) = mpsc::channel();
    let chunks = std::iter::once("apple--banana--").chain(std::iter::once_with(move || {
        gate.try_recv()
            .expect("first two rows must arrive before the next read");
        "cherry--date"
    }));
    let mut stream = ByteStream::from_iter(chunks, span, Signals::empty(), ByteStreamType::Unknown);
    stream.push_caller_span(caller_span);
    let input = PipelineData::byte_stream(stream, Some(metadata.clone()));
    let output = test().run_raw_with_data("split row '--'", input)?.body;
    assert_eq!(output.metadata_ref(), Some(&metadata));
    let PipelineData::ListStream(mut rows, _) = output else {
        panic!("expected a list stream");
    };
    assert_eq!(rows.span(), span);
    assert!(rows.get_caller_spans().contains(&caller_span));
    for expected in ["apple", "banana"] {
        let row = rows.next_value().unwrap();
        assert_eq!(row.span(), span);
        assert_eq!(row.as_str()?, expected);
    }
    release.send(()).unwrap();
    for expected in ["cherry", "date"] {
        assert_eq!(rows.next_value().unwrap().as_str()?, expected);
    }
    assert!(rows.next_value().is_none());
    Ok(())
}

#[test]
fn byte_stream_separator_and_unicode_cross_reads() -> Result {
    let bytes = "--🦀--α----β--".as_bytes();
    // Each read returns at most one byte, splitting the delimiter and all multibyte characters.
    let chunks = bytes.chunks(1);
    let input = ByteStream::from_iter(
        chunks,
        Span::test_data(),
        Signals::empty(),
        ByteStreamType::Unknown,
    );
    test()
        .run_raw_with_data("split row '--'", PipelineData::byte_stream(input, None))
        .and_then(|outcome| {
            outcome
                .body
                .into_value(Span::test_data())
                .map_err(Error::from)
        })
        .expect_value_eq(["", "🦀", "α", "", "β", ""])
}

#[test]
fn byte_stream_matches_string_splitting_and_eof_trimming() -> Result {
    let cases: &[(&str, &str, &[&str])] = &[
        ("split row '--'", "", &[""]),
        ("split row '--'", "\r\n\r", &[""]),
        ("split row '--'", "one", &["one"]),
        ("split row '--'", "a\r\n--b\n\r\n", &["a\r\n", "b"]),
        ("split row '--'", "a--\r\n", &["a", ""]),
        (r#"split row "\n""#, "a\nb\n\r\n", &["a", "b"]),
        (r#"split row "--\n""#, "a--\nb--\n", &["a", "b--"]),
        (r#"split row "\r\n--""#, "a\r\n--b\r\n", &["a", "b"]),
        ("split row -n 0 '--'", "a--b--c", &[]),
        ("split row -n 1 '--'", "a--b--c\r\n", &["a--b--c"]),
        ("split row -n 2 '--'", "a--b--c\r\n", &["a", "b--c"]),
        ("split row --right -n 2 '--'", "a--b--c\r\n", &["a--b", "c"]),
        ("split row --right '--'", "a--b--c", &["a", "b", "c"]),
        (
            r"split row --regex '\s*,\s*'",
            "a , b,c\r\n",
            &["a", "b", "c"],
        ),
        ("split row ''", "aβ\r\n", &["", "a", "β", ""]),
    ];
    let mut tester = test();
    for &(code, text, expected) in cases {
        let span = Span::test_data();
        let stream = ByteStream::read_string(text.into(), span, Signals::empty());
        tester
            .run_raw_with_data(code, PipelineData::byte_stream(stream, None))
            .and_then(|outcome| outcome.body.into_value(span).map_err(Error::from))
            .expect_value_eq(expected.to_vec())?;
    }
    Ok(())
}

#[test]
fn byte_stream_rejects_invalid_utf8_after_emitting_valid_rows() -> Result {
    let span = Span::new(10, 20);
    let stream = ByteStream::read_binary(b"apple--\xff--cherry", span, Signals::empty())
        .with_type(ByteStreamType::Unknown);
    let mut rows = test()
        .run_raw_with_data("split row '--'", PipelineData::byte_stream(stream, None))?
        .body
        .into_iter();
    assert_eq!(rows.next().unwrap().as_str()?, "apple");
    let error = rows.next().unwrap().unwrap_error().unwrap_err();
    assert!(matches!(
        error,
        ShellError::OnlySupportsThisInputType { exp_input_type, wrong_type, src_span, .. }
            if exp_input_type == "string" && wrong_type == "binary" && src_span == span
    ));
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn byte_stream_reports_read_error_without_emitting_unfinished_row() -> Result {
    let span = Span::test_data();
    let error = ShellError::AccessEmptyContent { span };
    let stream = ByteStream::from_result_iter(
        [Ok("apple--partial"), Err(error.clone()), Ok("discarded--")],
        span,
        Signals::empty(),
        ByteStreamType::Unknown,
    );
    let mut rows = test()
        .run_raw_with_data("split row '--'", PipelineData::byte_stream(stream, None))?
        .body
        .into_iter();
    assert_eq!(rows.next().unwrap().as_str()?, "apple");
    assert_eq!(rows.next().unwrap().unwrap_error().unwrap_err(), error);
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn byte_stream_read_error_takes_precedence_over_invalid_utf8() -> Result {
    let span = Span::test_data();
    let error = ShellError::AccessEmptyContent { span };
    let stream = ByteStream::from_result_iter(
        [Ok(b"apple--\xff--partial".as_slice()), Err(error.clone())],
        span,
        Signals::empty(),
        ByteStreamType::Unknown,
    );
    let mut rows = test()
        .run_raw_with_data("split row '--'", PipelineData::byte_stream(stream, None))?
        .body
        .into_iter();
    assert_eq!(rows.next().unwrap().as_str()?, "apple");
    assert_eq!(rows.next().unwrap().unwrap_error().unwrap_err(), error);
    assert!(rows.next().is_none());
    Ok(())
}

#[rstest]
#[case::while_buffering("partial", false)]
#[case::between_rows("apple--partial", true)]
fn byte_stream_reports_cancellation(
    #[case] chunk: &'static str,
    #[case] yields_row: bool,
) -> Result {
    let span = Span::test_data();
    let signals = Signals::new(Arc::new(AtomicBool::new(false)));
    let chunks = std::iter::once_with({
        let signals = signals.clone();
        move || {
            signals.trigger();
            chunk
        }
    })
    .chain(std::iter::once_with(|| {
        panic!("reader ran after cancellation")
    }));
    let stream = ByteStream::from_iter(chunks, span, signals, ByteStreamType::Unknown);
    let mut rows = test()
        .run_raw_with_data("split row '--'", PipelineData::byte_stream(stream, None))?
        .body
        .into_iter();
    if yields_row {
        assert_eq!(rows.next().unwrap().as_str()?, "apple");
    }
    assert_eq!(
        rows.next().unwrap().unwrap_error().unwrap_err(),
        ShellError::Interrupted { span },
    );
    assert!(rows.next().is_none());
    Ok(())
}

#[test]
fn first_drops_byte_stream_before_reading_to_eof() -> Result {
    let span = Span::test_data();
    let (reader, dropped) = mpsc::channel();
    let chunks = std::iter::once("apple--")
        .chain(std::iter::once_with(|| {
            panic!("first read past the separator")
        }))
        .inspect(move |_| reader.send(()).unwrap());
    let stream = ByteStream::from_iter(chunks, span, Signals::empty(), ByteStreamType::Unknown);
    test()
        .run_raw_with_data(
            "split row '--' | first",
            PipelineData::byte_stream(stream, None),
        )
        .and_then(|outcome| outcome.body.into_value(span).map_err(Error::from))
        .expect_value_eq("apple")?;
    dropped.try_recv().unwrap();
    assert_eq!(dropped.try_recv(), Err(mpsc::TryRecvError::Disconnected));
    Ok(())
}

#[test]
#[deps(NU)]
fn byte_stream_reports_child_exit_status() -> Result {
    let mut rows = test()
        .run_raw(format!("{FAILING_PRODUCER} | split row '--'"))?
        .body
        .into_iter();
    assert_eq!(rows.next().unwrap().as_str()?, "apple");
    let err = rows.next().unwrap().unwrap_error().unwrap_err();
    assert!(matches!(err, ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 7));
    assert!(rows.next().is_none());
    Ok(())
}

const FAILING_PRODUCER: &str = r#"^nu -n -c 'print --no-newline "apple--partial"; exit 7'"#;

#[test]
#[deps(NU, TESTBIN_RELAY)]
fn byte_stream_preserves_exit_status_through_external_stdin() -> Result {
    for pipefail in [true, false] {
        for sink in [
            "relay",
            "relay | complete | get exit_code",
            "relay | relay | complete",
        ] {
            let result: CompleteResult = test().run_with_data(
                format!("let code; ^nu -n --experimental-options '[pipefail={pipefail}]' -c $code | complete"),
                format!("{FAILING_PRODUCER} | split row '--' | {sink}"),
            )?;
            assert_eq!(
                result.exit_code, 7,
                "pipefail={pipefail}, sink={sink}: {result:?}"
            );
        }
    }
    Ok(())
}

#[test]
#[deps(NU)]
fn bare_try_handles_split_row_exit_status() -> Result {
    for pipefail in [true, false] {
        for transform in ["", " | each {|row| $row }"] {
            let result: CompleteResult = test().run_with_data(
                format!("let code; ^nu -n --experimental-options '[pipefail={pipefail}]' -c $code | complete"),
                format!("try {{ {FAILING_PRODUCER} | split row '--'{transform} }} | length"),
            )?;
            assert_eq!(result.exit_code, 0, "{result:?}");
            assert_eq!(result.stdout.trim(), "0", "{result:?}");
        }
    }
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_RELAY)]
fn byte_stream_child_exit_status_takes_precedence_over_invalid_utf8() -> Result {
    let err = test()
        .run("^nu -n -c '0x[6170706c652d2dff2d2d] | relay; exit 7' | split row '--' | length")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 7));
    Ok(())
}

#[test]
#[deps(TESTBIN_RELAY)]
fn byte_stream_preserves_child_stdin_feed_failure() -> Result {
    let err = test()
        .run(r#"[1] | each { error make {msg: "stdin failed"} } | relay | split row '--' | length"#)
        .expect_shell_error()?;
    assert_contains("stdin failed", err.to_string());
    Ok(())
}

#[test]
fn raw_file_trims_only_the_final_row(playground: Playground) -> Result {
    playground.file("rows.txt", "apple\r\n--banana\r\n")?;
    test()
        .cwd(playground.path())
        .run("open --raw rows.txt | split row '--'")
        .expect_value_eq(["apple\r\n", "banana"])
}

#[test]
#[deps(NU)]
fn do_handles_split_row_input_errors() -> Result {
    test()
        .run(format!(
            "do -i {{ {FAILING_PRODUCER} | split row '--' }} | length"
        ))
        .expect_value_eq(0)?;
    let err = test()
        .run(format!(
            "do -c {{ {FAILING_PRODUCER} | split row '--' }} | first"
        ))
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 7));
    Ok(())
}

#[cfg(unix)]
#[test]
#[deps(NU)]
fn first_emits_a_row_before_pipefail_reports_the_child() {
    use nu_utils::time::Instant;
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
        thread,
        time::Duration,
    };

    let mut child = Command::new(NU.path())
        .args([
            "-n",
            "--experimental-options",
            "[pipefail=true]",
            "-c",
            "^/bin/sh -c 'printf apple--; read gate; exit 7' | split row '--' | first",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let gate = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = send.send(result);
    });

    let timeout = Duration::from_secs(10);
    let first = receive.recv_timeout(timeout);
    let waiting = child.try_wait().unwrap().is_none();
    drop(gate);

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the pipeline did not exit after the producer was released");
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(first.unwrap().unwrap().trim_end(), "apple");
    assert!(waiting);
    assert_eq!(status.code(), Some(7));
}
