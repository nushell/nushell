use std::{
    fs::File,
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    process::{Child, Command, Stdio},
    sync::Arc,
    thread,
    time::Duration,
};

use nu_ansi_term::Style;
use nu_parser::parse;
use nu_protocol::{
    ast::Expr,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_utils::time::Instant;
use reedline::{
    AutoPairs, CwdAwareHinter, DefaultPrompt, EditCommand, Emacs, FileBackedHistory, Hinter,
    History, HistoryItem, Reedline, Signal,
};

use super::{AutoPairHintPolicy, ExternalHinter};

const CHILD_SCENARIO: &str = "NU_REEDLINE_HINT_PTY_SCENARIO";
const TEST_FILTER: &str = "hints::auto_pair_integration_tests::reedline_hint_policy_pty";
const TIMEOUT: Duration = Duration::from_secs(10);
const PAIRS: [(char, char); 6] = [
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('"', '"'),
    ('\'', '\''),
    ('`', '`'),
];

#[test]
fn reedline_hint_policy_pty() {
    if let Ok(scenario) = std::env::var(CHILD_SCENARIO) {
        run_read_line_child(&scenario);
        return;
    }

    let mut cwd = spawn_pty_child("cwd");
    send_and_expect(&mut cwd, b"", "(gs", 3, "tat).branch");
    send_and_expect(&mut cwd, b"\x1b[1;5C", "(gstat", 6, ").branch");
    send_and_expect(&mut cwd, b"\x1b[1;5C", "(gstat)", 7, ".branch");
    send_and_expect(
        &mut cwd,
        b"\x06",
        "(gstat).branch",
        "(gstat).branch".len(),
        "(gstat).branch",
    );
    send_and_wait(&mut cwd, b"\r", "__REEDLINE_RESULT__(gstat).branch");
    cwd.wait_success();

    let mut cwd_whole = spawn_pty_child("cwd-whole");
    send_and_expect(&mut cwd_whole, b"", "(gs", 3, "tat).branch");
    send_and_expect(
        &mut cwd_whole,
        b"\x06",
        "(gstat).branch",
        "(gstat).branch".len(),
        "(gstat).branch",
    );
    send_and_wait(&mut cwd_whole, b"\r", "__REEDLINE_RESULT__(gstat).branch");
    cwd_whole.wait_success();

    let mut external = spawn_pty_child("external");
    send_and_expect(&mut external, b"", "f()", 2, r#"")").field"#);
    send_and_expect(&mut external, b"\x1b[1;5C", r#"f(")")"#, 5, ").field");
    send_and_expect(&mut external, b"\x1b[1;5C", r#"f(")")"#, 6, ".field");
    send_and_expect(
        &mut external,
        b"\x06",
        r#"f(")").field"#,
        r#"f(")").field"#.len(),
        r#"f(")").field"#,
    );
    send_and_wait(&mut external, b"\r", r#"__REEDLINE_RESULT__f(")").field"#);
    external.wait_success();

    let mut external_whole = spawn_pty_child("external-whole");
    send_and_expect(&mut external_whole, b"", "f()", 2, r#"")").field"#);
    send_and_expect(
        &mut external_whole,
        b"\x06",
        r#"f(")").field"#,
        r#"f(")").field"#.len(),
        r#"f(")").field"#,
    );
    send_and_wait(
        &mut external_whole,
        b"\r",
        r#"__REEDLINE_RESULT__f(")").field"#,
    );
    external_whole.wait_success();

    for (index, quote) in ['"', '\'', '`'].into_iter().enumerate() {
        let scenario = format!("quote-{index}");
        let mut child = spawn_pty_child(&scenario);
        let hint = format!("lo{quote}");
        let result = format!("echo {quote}hello{quote}");
        let prefix = format!("echo {quote}hel");
        send_and_expect(&mut child, &[], &prefix, prefix.len(), &hint);
        match index {
            0 => {
                send_and_expect(&mut child, b"\x06", &result, result.len(), &result);
            }
            1 => {
                let with_text = format!("echo {quote}hello");
                send_and_expect(
                    &mut child,
                    b"\x1b[1;5C",
                    &with_text,
                    with_text.len(),
                    &quote.to_string(),
                );
                send_and_expect(&mut child, b"\x1b[1;5C", &result, result.len(), &result);
            }
            _ => {
                let with_text = format!("echo {quote}hello");
                send_and_expect(
                    &mut child,
                    b"\x1b[1;5C",
                    &with_text,
                    with_text.len(),
                    &quote.to_string(),
                );
                send_and_expect(&mut child, b"\x06", &result, result.len(), &result);
            }
        }
        send_and_wait(&mut child, b"\r", &format!("__REEDLINE_RESULT__{result}"));
        child.wait_success();
    }
}

fn run_read_line_child(scenario: &str) {
    let mut history = FileBackedHistory::new(16).expect("in-memory history");
    let (source, cursor, candidate, external) = match scenario {
        "cwd" | "cwd-whole" => ("(gs)".to_owned(), 3, "(gstat).branch".to_owned(), false),
        "external" | "external-whole" => ("f()".to_owned(), 2, String::new(), true),
        "quote-0" => quote_case('"'),
        "quote-1" => quote_case('\''),
        "quote-2" => quote_case('`'),
        other => panic!("unknown PTY scenario {other}"),
    };

    if !external {
        history
            .save(HistoryItem::from_command_line(candidate))
            .expect("seed history");
    }

    let hinter: Box<dyn Hinter> = if external {
        Box::new(make_external_hinter())
    } else {
        Box::new(CwdAwareHinter::default())
    };
    let hinter = RecordingHinter { inner: hinter };

    let mut editor = Reedline::create()
        .with_edit_mode(Box::<Emacs>::default())
        .with_history(Box::new(history))
        .with_auto_pairs(AutoPairs::new(PAIRS))
        .with_hint_policy(Box::new(AutoPairHintPolicy::new(external)))
        .with_hinter(Box::new(hinter))
        .with_cwd(Some("/tmp".to_owned()))
        .with_ansi_colors(false);
    editor.run_edit_commands(&[
        EditCommand::InsertString(source),
        EditCommand::MoveToPosition {
            position: cursor,
            select: false,
        },
    ]);

    let signal = editor
        .read_line(&DefaultPrompt::default())
        .expect("read line through the PTY");
    let Signal::Success(buffer) = signal else {
        panic!("expected successful submission, got {signal:?}");
    };
    drop(editor);
    match scenario {
        "cwd" => {
            assert_eq!(buffer, "(gstat).branch");
        }
        "cwd-whole" => {
            assert_eq!(buffer, "(gstat).branch");
        }
        "external" => {
            assert_eq!(buffer, "f(\")\").field");
        }
        "external-whole" => {
            assert_eq!(buffer, "f(\")\").field");
        }
        _ => {
            let quote = match scenario {
                "quote-0" => '"',
                "quote-1" => '\'',
                "quote-2" => '`',
                _ => unreachable!(),
            };
            assert_eq!(buffer, format!("echo {quote}hello{quote}"));
        }
    }
    println!("__REEDLINE_RESULT__{buffer}");
}

fn quote_case(quote: char) -> (String, usize, String, bool) {
    let source = format!("echo {quote}hel{quote}");
    let cursor = source.len() - quote.len_utf8();
    let candidate = format!("echo {quote}hello{quote}");
    (source, cursor, candidate, false)
}

fn make_external_hinter() -> ExternalHinter {
    let mut engine_state = nu_cmd_lang::create_default_context();
    let closure = parse_closure(
        &mut engine_state,
        r#"{|ctx|
            if $ctx.line == 'f()' and $ctx.pos == 2 {
                {hint: '")").field', next_token: '")"'}
            } else if $ctx.line == 'f(")")' and $ctx.pos == 5 {
                {hint: ').field', next_token: ')'}
            } else if $ctx.line == 'f(")")' and $ctx.pos == 6 {
                {hint: '.field', next_token: '.'}
            } else {
                {hint: '', next_token: ''}
            }
        }"#,
    );
    ExternalHinter::new(
        Arc::new(engine_state),
        Arc::new(Stack::new()),
        closure,
        Style::new(),
    )
}

fn parse_closure(engine_state: &mut EngineState, source: &str) -> nu_protocol::engine::Closure {
    let mut working_set = StateWorkingSet::new(engine_state);
    let parsed = parse(&mut working_set, None, source.as_bytes(), false);
    assert!(
        working_set.parse_errors.is_empty(),
        "{:#?}",
        working_set.parse_errors
    );
    let expression = &parsed.pipelines[0].elements[0].expr.expr;
    let Expr::Closure(block_id) = expression else {
        panic!("expected a closure expression");
    };
    let closure = nu_protocol::engine::Closure {
        block_id: *block_id,
        captures: vec![],
    };
    engine_state
        .merge_delta(working_set.render())
        .expect("merge parsed closure");
    closure
}

struct RecordingHinter {
    inner: Box<dyn Hinter>,
}

impl Hinter for RecordingHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        history: &dyn History,
        use_ansi_coloring: bool,
        cwd: &str,
    ) -> String {
        let rendered = self
            .inner
            .handle(line, pos, history, use_ansi_coloring, cwd);
        let marker = format!("\r\n__REEDLINE_QUERY__{line:?}@{pos}\r\n");
        std::io::stderr()
            .write_all(marker.as_bytes())
            .expect("write query marker to PTY");
        rendered
    }

    fn complete_hint(&self) -> String {
        self.inner.complete_hint()
    }

    fn next_hint_token(&self) -> String {
        self.inner.next_hint_token()
    }
}

struct PtyChild {
    child: Child,
    master: File,
}

impl PtyChild {
    fn write_keys(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).expect("write PTY input");
        self.master.flush().expect("flush PTY input");
    }

    fn wait_for(
        &mut self,
        expected: &str,
        wait_for_paint: bool,
        visible_hint: Option<&str>,
    ) -> Vec<u8> {
        let deadline = Instant::now() + TIMEOUT;
        let mut output = Vec::new();
        let mut answered_queries = 0;
        while Instant::now() < deadline {
            let transcript = String::from_utf8_lossy(&output);
            if let Some(marker) = transcript.find(expected) {
                let after_marker = &transcript[marker + expected.len()..];
                let paint_seen = !wait_for_paint || after_marker.contains("\x1b[?25h");
                if paint_seen {
                    if let Some(hint) = visible_hint {
                        assert!(
                            after_marker.contains(hint),
                            "marker {expected:?} did not paint hint {hint:?}; output: {transcript}"
                        );
                    }
                    return output;
                }
            }
            let mut descriptor = libc::pollfd {
                fd: self.master.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let remaining = deadline.saturating_duration_since(Instant::now());
            let timeout_ms = remaining.as_millis().min(100) as i32;
            // SAFETY: `descriptor` points to one initialized pollfd for the live PTY master.
            let ready = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
            if ready < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                panic!(
                    "poll PTY output failed: {}",
                    std::io::Error::last_os_error()
                );
            }
            if ready > 0 && descriptor.revents & libc::POLLIN != 0 {
                let mut chunk = [0; 4096];
                match self.master.read(&mut chunk) {
                    Ok(0) => panic!("PTY child closed output before {expected:?}"),
                    Ok(count) => {
                        output.extend_from_slice(&chunk[..count]);
                        self.answer_cursor_queries(&output, &mut answered_queries);
                        continue;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => panic!("read PTY output failed: {error}"),
                }
            }
            if let Some(status) = self.child.try_wait().expect("check PTY child") {
                panic!(
                    "PTY child exited with {status} before {expected:?}; output: {}",
                    String::from_utf8_lossy(&output)
                );
            }
        }
        panic!(
            "timed out waiting for {expected:?}; output: {}",
            String::from_utf8_lossy(&output)
        );
    }

    fn answer_cursor_queries(&mut self, output: &[u8], answered_queries: &mut usize) {
        let found = output
            .windows(4)
            .filter(|window| *window == b"\x1b[6n")
            .count();
        while *answered_queries < found {
            self.master
                .write_all(b"\x1b[1;1R")
                .expect("answer terminal cursor query");
            self.master.flush().expect("flush cursor response");
            *answered_queries += 1;
        }
    }

    fn wait_success(&mut self) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().expect("wait for PTY child") {
                assert!(status.success(), "PTY child failed with {status}");
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for PTY child");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for PtyChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn spawn_pty_child(scenario: &str) -> PtyChild {
    let mut master = -1;
    let mut slave = -1;
    let size = libc::winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: openpty initializes both output descriptors and receives valid pointers to
    // writable descriptor integers, an optional null name/termios, and a winsize value.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
        )
    };
    assert_eq!(
        result,
        0,
        "open PTY failed: {}",
        std::io::Error::last_os_error()
    );

    // SAFETY: openpty returned newly owned descriptors that are transferred to these Files.
    let master = unsafe { File::from_raw_fd(master) };
    // SAFETY: openpty returned a distinct newly owned slave descriptor.
    let slave = unsafe { File::from_raw_fd(slave) };
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .arg("--exact")
        .arg(TEST_FILTER)
        .arg("--test-threads")
        .arg("1")
        .arg("--no-capture")
        .env(CHILD_SCENARIO, scenario)
        .env("TERM", "xterm-256color")
        .stdin(Stdio::from(
            slave.try_clone().expect("clone PTY slave for stdin"),
        ))
        .stdout(Stdio::from(
            slave.try_clone().expect("clone PTY slave for stdout"),
        ))
        .stderr(Stdio::from(slave));
    let child = command.spawn().expect("spawn PTY child");
    PtyChild { child, master }
}

fn send_and_wait(child: &mut PtyChild, keys: &[u8], expected: &str) {
    child.write_keys(keys);
    child.wait_for(expected, false, None);
}

fn send_and_expect(
    child: &mut PtyChild,
    keys: &[u8],
    query_line: &str,
    query_pos: usize,
    visible_hint: &str,
) {
    child.write_keys(keys);
    let marker = format!("__REEDLINE_QUERY__{query_line:?}@{query_pos}");
    child.wait_for(&marker, true, Some(visible_hint));
}
