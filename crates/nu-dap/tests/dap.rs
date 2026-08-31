//! End-to-end integration tests: spawn the built `nu` binary in adapter mode
//! (`nu --dap`) and drive it over the Debug Adapter Protocol against the
//! scripts in `tests/fixtures/`.
//!
//! The fixtures are test inputs, not documentation: assertions below pin exact
//! line numbers in them, so they live here rather than in `example/` (which is
//! a curated, freely editable showcase for users).
//!
//! This harness plays the "editor" side of DAP. Pure-logic unit tests live
//! inline in the library modules; these cover protocol behaviour end to end
//! (real engine, real pauses, real stepping) — the full correctness suite.

#![allow(clippy::unwrap_used)] // tests

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use nu_test_support::prelude::*;
use nu_utils::time::Instant;
use serde_json::{Value, json};

/// Absolute path to a fixture script (in this crate's `tests/fixtures/` dir).
fn example(name: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push(name);
    p.to_string_lossy().into_owned()
}

/// A minimal DAP client over the adapter's stdio.
struct Dap {
    child: Arc<Mutex<Child>>,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    seq: i64,
    pending: Vec<Value>,
    alive: Arc<AtomicBool>,
}

impl Dap {
    /// `nu --dap`: the adapter is embedded in the main binary, so there is no
    /// separate `nu-dap` executable to spawn. Tests that use this must declare
    /// `#[deps(NU)]` so the harness builds `nu` first.
    ///
    /// `--no-config-file` keeps runs hermetic: the adapter evaluates the user's
    /// startup files like any other `nu`, and a dev machine's config.nu must not
    /// leak into assertions. `spawn_with_args` opts back in.
    fn command() -> Command {
        let mut command = Command::new(NU.path());
        command.args(["--dap", "--no-config-file"]);
        command
    }

    fn spawn() -> Dap {
        Self::spawn_command(Self::command())
    }

    /// Spawn `nu --dap <args>` without the hermetic `--no-config-file`.
    fn spawn_with_args<I, S>(args: I) -> Dap
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(NU.path());
        command.arg("--dap").args(args);
        Self::spawn_command(command)
    }

    fn spawn_with_env(key: &str, value: impl AsRef<OsStr>) -> Dap {
        let mut command = Self::command();
        command.env(key, value);
        Self::spawn_command(command)
    }

    fn spawn_command(mut command: Command) -> Dap {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn nu --dap");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let child = Arc::new(Mutex::new(child));
        let alive = Arc::new(AtomicBool::new(true));

        // Watchdog: kill the process if a test hangs, so a regression fails
        // (stdout EOF) instead of blocking `cargo test` forever.
        {
            let child = Arc::clone(&child);
            let alive = Arc::clone(&alive);
            std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(60);
                while Instant::now() < deadline {
                    if !alive.load(Ordering::SeqCst) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                let _ = child.lock().unwrap().kill();
            });
        }

        Dap {
            child,
            stdin,
            out,
            seq: 0,
            pending: Vec::new(),
            alive,
        }
    }

    fn send(&mut self, command: &str, arguments: Value) {
        self.seq += 1;
        let msg = json!({
            "seq": self.seq, "type": "request",
            "command": command, "arguments": arguments,
        });
        let payload = serde_json::to_vec(&msg).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", payload.len()).unwrap();
        self.stdin.write_all(&payload).unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_one(&mut self) -> Option<Value> {
        let mut len = None;
        loop {
            let mut line = String::new();
            if self.out.read_line(&mut line).ok()? == 0 {
                return None; // EOF
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(rest) = line.strip_prefix("Content-Length:") {
                len = rest.trim().parse::<usize>().ok();
            }
        }
        let len = len?;
        let mut buf = vec![0u8; len];
        self.out.read_exact(&mut buf).ok()?;
        serde_json::from_slice(&buf).ok()
    }

    /// First message (queued or freshly read) matching `pred`; others queue.
    fn recv_until(&mut self, pred: impl Fn(&Value) -> bool) -> Option<Value> {
        if let Some(i) = self.pending.iter().position(&pred) {
            return Some(self.pending.remove(i));
        }
        loop {
            let m = self.read_one()?;
            if pred(&m) {
                return Some(m);
            }
            self.pending.push(m);
        }
    }

    fn response(&mut self, command: &str) -> Value {
        let c = command.to_string();
        self.recv_until(|m| m["type"] == "response" && m["command"] == c.as_str())
            .expect("response")
    }

    fn event(&mut self, event: &str) -> Value {
        let e = event.to_string();
        self.recv_until(|m| m["type"] == "event" && m["event"] == e.as_str())
            .expect("event")
    }

    fn stop_or_term(&mut self) -> Value {
        self.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "stopped" || m["event"] == "terminated")
        })
        .expect("stopped/terminated")
    }

    fn initialize(&mut self) -> Value {
        self.send("initialize", json!({ "adapterID": "nushell" }));
        let resp = self.response("initialize");
        self.event("initialized");
        resp
    }

    /// initialize + launch + setBreakpoints + configurationDone.
    fn start(&mut self, script: &str, launch_extra: Value, bps: &[i64]) {
        self.initialize();
        let mut args = json!({ "program": script, "stopOnEntry": false });
        merge(&mut args, launch_extra);
        self.send("launch", args);
        self.response("launch");
        if !bps.is_empty() {
            self.send(
                "setBreakpoints",
                json!({
                    "source": { "path": script },
                    "breakpoints": bps.iter().map(|l| json!({ "line": l })).collect::<Vec<_>>(),
                }),
            );
            self.response("setBreakpoints");
        }
        self.send("configurationDone", json!({}));
        self.response("configurationDone");
    }

    fn top_line(&mut self) -> i64 {
        self.send("stackTrace", json!({ "threadId": 1 }));
        let r = self.response("stackTrace");
        r["body"]["stackFrames"][0]["line"].as_i64().unwrap_or(0)
    }

    fn top_frame_name(&mut self) -> String {
        self.send("stackTrace", json!({ "threadId": 1 }));
        let r = self.response("stackTrace");
        r["body"]["stackFrames"][0]["name"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }

    /// Locals (scope ref 1) as name -> rendered value.
    fn locals(&mut self) -> std::collections::HashMap<String, String> {
        self.variables(1)
    }

    fn variables(&mut self, reference: i64) -> std::collections::HashMap<String, String> {
        self.send("variables", json!({ "variablesReference": reference }));
        let r = self.response("variables");
        r["body"]["variables"]
            .as_array()
            .map(|vs| {
                vs.iter()
                    .map(|v| {
                        (
                            v["name"].as_str().unwrap_or("").to_string(),
                            v["value"].as_str().unwrap_or("").to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn cont(&mut self) {
        self.send("continue", json!({ "threadId": 1 }));
        self.response("continue");
    }
}

impl Drop for Dap {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        let _ = self.child.lock().unwrap().kill();
    }
}

/// Shallow-merge object `b` into `a`.
fn merge(a: &mut Value, b: Value) {
    if let (Some(a), Some(b)) = (a.as_object_mut(), b.as_object()) {
        for (k, v) in b {
            a.insert(k.clone(), v.clone());
        }
    }
}

// --------------------------------------------------------------------------

#[test]
#[deps(NU)]
fn initialize_advertises_capabilities() {
    let mut d = Dap::spawn();
    let resp = d.initialize();
    let caps = &resp["body"];
    assert_eq!(caps["supportsConfigurationDoneRequest"], true);
    assert_eq!(caps["supportsConditionalBreakpoints"], true);
    assert_eq!(caps["supportsExceptionInfoRequest"], true);
    assert_eq!(caps["supportsStepBack"], true, "time travel advertised");
    assert_eq!(caps["supportsRestartRequest"], true);
}

#[test]
#[deps(NU)]
fn breakpoints_scopes_variables_and_visualize() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[17]);

    // The IR panel event is emitted just before `stopped`, with the current
    // instruction index present in the listing.
    let ir = d.event("nuDapIr");
    let idx = ir["body"]["instructionIndex"].as_i64().unwrap();
    let text = ir["body"]["text"].as_str().unwrap();
    assert!(
        !text.is_empty() && text.contains(&format!("{idx}:")),
        "IR listing"
    );

    let ev = d.event("stopped");
    assert_eq!(ev["body"]["reason"], "breakpoint");
    assert_eq!(d.top_line(), 17);

    // Top frame reports its source file.
    d.send("stackTrace", json!({ "threadId": 1 }));
    let st = d.response("stackTrace");
    let path = st["body"]["stackFrames"][0]["source"]["path"]
        .as_str()
        .unwrap_or("");
    assert!(path.contains("demo.nu"), "frame source path: {path}");

    // Situational scopes (Pipeline / Registers / Process) are hidden when
    // empty; at this stop only Locals and Globals have content.
    d.send("scopes", json!({ "frameId": 0 }));
    let scopes = d.response("scopes");
    let names: Vec<String> = scopes["body"]["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["Locals", "Globals"]);

    // Locals: `return` (latest = classify "small"), plus files/total.
    let loc = d.locals();
    assert_eq!(loc.get("return").map(String::as_str), Some("\"small\""));
    assert!(loc.contains_key("files") && loc.contains_key("total"));
    let files = &loc["files"];
    assert!(files.contains("table 3 rows"), "files preview: {files}");

    // Visualize `files` -> full 3-record table.
    d.send("variables", json!({ "variablesReference": 1 }));
    let vars = d.response("variables");
    let files_ref = vars["body"]["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "files")
        .and_then(|v| v["variablesReference"].as_i64())
        .filter(|r| *r > 0)
        .expect("files expandable");
    d.send("nuDapVisualize", json!({ "variablesReference": files_ref }));
    let viz = d.response("nuDapVisualize");
    let rows = viz["body"]["value"].as_array().expect("array");
    assert_eq!(rows.len(), 3);
    assert!(rows[0].get("name").is_some() && rows[0].get("size").is_some());

    // Evaluate real expressions against captured variables.
    d.send(
        "evaluate",
        json!({ "expression": "$total + 100", "context": "repl" }),
    );
    assert_eq!(d.response("evaluate")["body"]["result"], "100");
    d.send(
        "evaluate",
        json!({ "expression": "$files | length", "context": "repl" }),
    );
    assert_eq!(d.response("evaluate")["body"]["result"], "3");

    // Clear the bp (it fires once per loop iteration) so continue terminates.
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
}

#[test]
#[deps(NU)]
fn custom_command_frame_and_parameters() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[5]); // inside `classify`

    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    d.send("stackTrace", json!({ "threadId": 1 }));
    let frames = d.response("stackTrace");
    let frames = frames["body"]["stackFrames"].as_array().unwrap();
    assert_eq!(frames[0]["name"], "classify");
    assert_eq!(frames[0]["line"], 5);
    assert_eq!(frames[1]["line"], 16, "caller at the call site");

    // Parameter captured from the call site, and usable in evaluate.
    assert_eq!(d.locals().get("size").map(String::as_str), Some("120"));
    d.send(
        "evaluate",
        json!({ "expression": "$size * 2", "context": "watch" }),
    );
    assert_eq!(d.response("evaluate")["body"]["result"], "240");
}

#[test]
#[deps(NU)]
fn closure_params_and_in_are_visible() {
    // Reading the real Stack (nushell #18708) exposes a closure's own
    // parameter — impossible under the old IR shadow reconstruction, which
    // never saw the callee's bindings.
    let script = example("closure.nu");
    let mut d = Dap::spawn();
    d.start(&script, json!({}), &[3]); // `let doubled = $elt * 2`, first iteration

    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    let loc = d.locals();
    assert_eq!(
        loc.get("elt").map(String::as_str),
        Some("10"),
        "closure param visible: {loc:?}"
    );
    assert_eq!(
        loc.get("in").map(String::as_str),
        Some("10"),
        "$in is the current element: {loc:?}"
    );
    // And it resolves in watch expressions through the scratch engine.
    d.send(
        "evaluate",
        json!({ "expression": "$elt + 5", "context": "watch" }),
    );
    assert_eq!(d.response("evaluate")["body"]["result"], "15");
}

#[test]
#[deps(NU)]
fn closure_rows_show_source_and_captures() {
    // A closure-valued local reads as the literal the user wrote, not
    // `<closure>`. The source text comes from the block's span, which only the
    // eval thread can resolve — so this also covers the snapshot plumbing that
    // carries it to the server thread.
    let script = example("closure_value.nu");
    let mut d = Dap::spawn();
    d.start(&script, json!({}), &[4]); // `print "done"`, both closures bound

    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    let loc = d.locals();
    assert_eq!(
        loc.get("double").map(String::as_str),
        Some("{|x| $x * 2}"),
        "closure body in the row: {loc:?}"
    );
    // `$n` is closed over, and the count comes off the value itself.
    assert_eq!(
        loc.get("scaled").map(String::as_str),
        Some("{|x| $x * $n} +1 capture"),
        "captures counted: {loc:?}"
    );

    // The count is only a summary — expanding the closure shows the captured
    // value itself, named as it was in the enclosing scope.
    d.send("variables", json!({ "variablesReference": 1 }));
    let locals = d.response("variables");
    let scaled_ref = locals["body"]["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "scaled")
        .and_then(|v| v["variablesReference"].as_i64())
        .expect("scaled is expandable");
    assert!(scaled_ref > 0, "capturing closure has a reference");

    d.send("variables", json!({ "variablesReference": scaled_ref }));
    let captures = d.response("variables");
    let captures: Vec<(String, String)> = captures["body"]["variables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["name"].as_str().unwrap().to_string(),
                v["value"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(captures, vec![("n".to_string(), "10".to_string())]);
}

#[test]
#[deps(NU)]
fn stepping_never_lands_on_line_one() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({ "stopOnEntry": true }), &[]);
    assert_eq!(d.event("stopped")["body"]["reason"], "entry");

    let mut lines = vec![d.top_line()];
    let mut terminated = false;
    for _ in 0..100 {
        d.send("next", json!({ "threadId": 1 }));
        d.response("next");
        let ev = d.stop_or_term();
        if ev["event"] == "terminated" {
            terminated = true;
            break;
        }
        lines.push(d.top_line());
    }
    assert!(terminated, "ran to termination");
    assert!(
        !lines.contains(&1) && !lines.contains(&0),
        "no line 1/0: {lines:?}"
    );
}

#[test]
#[deps(NU)]
fn exception_breakpoint_pauses_at_the_raising_line() {
    let err = example("err.nu");
    let mut d = Dap::spawn();
    d.start(&err, json!({}), &[]);

    let ev = d.event("stopped");
    assert_eq!(ev["body"]["reason"], "exception");
    assert!(ev["body"]["text"].as_str().unwrap().contains("boom"));
    assert_eq!(d.top_line(), 4);

    d.send("exceptionInfo", json!({ "threadId": 1 }));
    let info = d.response("exceptionInfo");
    assert!(
        info["body"]["description"]
            .as_str()
            .unwrap()
            .contains("boom")
    );
    // The id is nushell's diagnostic code, not a scraped variant name.
    let id = info["body"]["exceptionId"].as_str().unwrap();
    assert!(id.starts_with("nu::"), "exceptionId: {id}");

    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
}

#[test]
#[deps(NU)]
fn breakpoint_in_sourced_file_hits() {
    let multi = example("multi.nu");
    let helper = example("helper.nu");
    let mut d = Dap::spawn();
    d.initialize();
    d.send("launch", json!({ "program": multi, "stopOnEntry": false }));
    d.response("launch");
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": helper }, "breakpoints": [{ "line": 2 }] }),
    );
    d.response("setBreakpoints");
    d.send("configurationDone", json!({}));
    d.response("configurationDone");

    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.top_frame_name(), "double");
    assert_eq!(d.locals().get("x").map(String::as_str), Some("21"));
}

#[test]
#[deps(NU)]
fn breakpoint_verification_snaps_to_next_line() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[13]); // blank line

    // The eval thread reconciles the bp after parse -> `breakpoint` event.
    let ev = d.recv_until(|m| m["type"] == "event" && m["event"] == "breakpoint");
    let bp = &ev.unwrap()["body"]["breakpoint"];
    assert_eq!(bp["verified"], true);
    assert_eq!(bp["line"], 14, "snapped 13 -> 14");
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.top_line(), 14);
}

#[test]
#[deps(NU)]
fn conditional_breakpoint_and_logpoint() {
    let demo = example("demo.nu");

    // Conditional: only pause when $total > 4000 (third loop pass).
    let mut d = Dap::spawn();
    d.initialize();
    d.send("launch", json!({ "program": demo, "stopOnEntry": false }));
    d.response("launch");
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo },
                "breakpoints": [{ "line": 17, "condition": "$total > 4000" }] }),
    );
    d.response("setBreakpoints");
    d.send("configurationDone", json!({}));
    d.response("configurationDone");
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.locals().get("total").map(String::as_str), Some("4216"));

    // Logpoint: three interpolated messages, no pause.
    let mut d = Dap::spawn();
    d.initialize();
    d.send("launch", json!({ "program": demo, "stopOnEntry": false }));
    d.response("launch");
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo },
                "breakpoints": [{ "line": 16, "logMessage": "file {$f.name} total {$total}" }] }),
    );
    d.response("setBreakpoints");
    d.send("configurationDone", json!({}));
    d.response("configurationDone");
    let mut logs = Vec::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        if ev["body"]["category"] == "console" {
            let o = ev["body"]["output"].as_str().unwrap();
            if !o.starts_with("nu-dap ") {
                logs.push(o.trim().to_string());
            }
        }
    }
    assert_eq!(logs.len(), 3, "logs: {logs:?}");
    assert_eq!(logs[0], "file a.txt total 0");
    assert_eq!(logs[2], "file c.log total 4216");

    // Conditional logpoint: DAP allows `condition` and `logMessage` on the same
    // breakpoint, and VS Code offers both in one editor. The condition gates the
    // logging — still no pause.
    let mut d = Dap::spawn();
    d.initialize();
    d.send("launch", json!({ "program": demo, "stopOnEntry": false }));
    d.response("launch");
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo },
                "breakpoints": [{ "line": 16,
                                  "condition": "$total > 4000",
                                  "logMessage": "late {$f.name} total {$total}" }] }),
    );
    d.response("setBreakpoints");
    d.send("configurationDone", json!({}));
    d.response("configurationDone");
    let mut logs = Vec::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        if ev["body"]["category"] == "console" {
            let o = ev["body"]["output"].as_str().unwrap();
            if !o.starts_with("nu-dap ") {
                logs.push(o.trim().to_string());
            }
        }
    }
    assert_eq!(logs, vec!["late c.log total 4216".to_string()], "{logs:?}");

    // Logpoint written in nushell's own interpolation syntax (`$"...($x)"`)
    // instead of DAP `{expr}` — must interpolate the same way.
    let mut d = Dap::spawn();
    d.initialize();
    d.send("launch", json!({ "program": demo, "stopOnEntry": false }));
    d.response("launch");
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo },
                "breakpoints": [{ "line": 16,
                    "logMessage": "$\"file ($f.name) total ($total)\"" }] }),
    );
    d.response("setBreakpoints");
    d.send("configurationDone", json!({}));
    d.response("configurationDone");
    let mut nu_logs = Vec::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        if ev["body"]["category"] == "console" {
            let o = ev["body"]["output"].as_str().unwrap();
            if !o.starts_with("nu-dap ") {
                nu_logs.push(o.trim().to_string());
            }
        }
    }
    assert_eq!(nu_logs.len(), 3, "nu logs: {nu_logs:?}");
    assert_eq!(nu_logs[0], "file a.txt total 0");
    assert_eq!(nu_logs[2], "file c.log total 4216");
}

#[test]
#[deps(NU)]
fn step_into_pipeline_stage_shows_input() {
    let pipeline = example("pipeline.nu");
    let mut d = Dap::spawn();
    d.start(&pipeline, json!({}), &[4]); // `$nums | each {|n| $n * 2}`
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");

    // Clear the bp so the next stop is attributable to the step.
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": pipeline }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");

    // F11 stops at the `each` call with `in → each` = the list.
    d.send("stepIn", json!({ "threadId": 1 }));
    d.response("stepIn");
    assert_eq!(d.stop_or_term()["event"], "stopped");
    let pipe = d.variables(2); // Pipeline scope
    assert_eq!(pipe.get("in → each").map(String::as_str), Some("[1, 2, 3]"));

    // F11 again descends into the same-line closure (one frame deeper).
    d.send("stepIn", json!({ "threadId": 1 }));
    d.response("stepIn");
    assert_eq!(d.stop_or_term()["event"], "stopped");
    d.send("stackTrace", json!({ "threadId": 1 }));
    let frames = d.response("stackTrace");
    assert!(frames["body"]["stackFrames"].as_array().unwrap().len() >= 2);
    assert_eq!(frames["body"]["stackFrames"][0]["line"], 4);
}

#[test]
#[deps(NU)]
fn globals_scope_exposes_nu_and_env() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[17]);
    d.event("stopped");

    d.send("variables", json!({ "variablesReference": 6 })); // Globals
    let g = d.response("variables");
    let g = g["body"]["variables"].as_array().unwrap();
    let nu_ref = g
        .iter()
        .find(|v| v["name"] == "$nu")
        .and_then(|v| v["variablesReference"].as_i64())
        .filter(|r| *r > 0)
        .expect("$nu expandable");
    assert!(g.iter().any(|v| v["name"] == "$env"));
    let nu = d.variables(nu_ref);
    assert!(
        ["pid", "os-info", "temp-path", "home-path", "config-path"]
            .iter()
            .any(|k| nu.contains_key(*k)),
        "$nu fields: {:?}",
        nu.keys().collect::<Vec<_>>()
    );
}

#[test]
#[deps(NU)]
fn time_travel_step_back_through_history() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[17]);
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.top_line(), 17);
    assert!(d.locals().contains_key("total"), "total live");

    // Step Back through recorded history: earlier lines, earlier state, and
    // never resumes (no terminated).
    let mut saw_earlier_line = false;
    let mut saw_before_total = false;
    for _ in 0..15 {
        d.send("stepBack", json!({ "threadId": 1 }));
        d.response("stepBack");
        let ev = d.stop_or_term();
        assert_eq!(ev["event"], "stopped", "stepBack never resumes");
        if d.top_line() < 17 {
            saw_earlier_line = true;
        }
        if !d.locals().contains_key("total") {
            saw_before_total = true;
        }
    }
    assert!(saw_earlier_line, "stepped back to earlier lines");
    assert!(saw_before_total, "reached a point before `total` existed");

    // Reverse-continue: no earlier breakpoint recorded, so it lands at the
    // first recorded moment and stays paused (never resumes execution).
    d.send("reverseContinue", json!({ "threadId": 1 }));
    d.response("reverseContinue");
    assert_eq!(
        d.stop_or_term()["event"],
        "stopped",
        "reverseContinue stays paused"
    );

    // Continue from the past returns to the live frontier (line 17), then a
    // live continue after clearing the bp runs to termination.
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "stopped");
    assert_eq!(d.top_line(), 17);
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
}

#[test]
#[deps(NU)]
fn lazy_top_level_pipeline_breakpoints_hit() {
    let script = example("lazy_iter.nu");
    let mut d = Dap::spawn();
    d.start(&script, json!({}), &[6]); // inside `each { print … }`

    let mut ins = Vec::new();
    for _ in 0..3 {
        let ev = d.stop_or_term();
        if ev["event"] != "stopped" {
            break;
        }
        assert_eq!(d.top_line(), 6);
        ins.push(d.locals().get("in").cloned().unwrap_or_default());
        d.cont();
    }
    assert_eq!(ins, ["10", "20", "30"], "element visible as $in each pass");
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_STDERR_FAIL)]
fn failing_external_attaches_stderr() -> nu_test_support::Result {
    Playground::setup("nu_dap_extfail", |dirs, _| {
        let script = dirs.test().join("nu_dap_extfail.nu");
        let testbin = TESTBIN_ECHO_ENV_STDERR_FAIL
            .path()
            .to_string_lossy()
            .to_string();
        let testbin = serde_json::to_string(&testbin).unwrap();

        std::fs::write(
            &script,
            format!(
                "print \"start\"\n$env.FOO = \"AADSTS-detail\"\nlet testbin = {testbin}\n^$testbin FOO\nprint \"unreachable\"\n"
            ),
        )?;
        let script = script.to_string_lossy().into_owned();

        let mut d = Dap::spawn();
        d.start(&script, json!({}), &[]);
        let ev = d.event("stopped");
        assert_eq!(ev["body"]["reason"], "exception");
        let text = ev["body"]["text"].as_str().unwrap();
        assert!(text.to_lowercase().contains("non-zero exit"));
        assert!(text.contains("AADSTS-detail"), "stderr attached: {text}");

        // Process scope (ref 5) exposes the stderr tail.
        d.send("variables", json!({ "variablesReference": 5 }));
        let proc = d.response("variables");
        assert!(
            proc["body"]["variables"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["name"] == "last error output")
        );

        d.cont();
        assert_eq!(d.stop_or_term()["event"], "terminated");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn entry_point_runs_a_chosen_function() {
    let lib = example("lib.nu");

    // Chosen entry point with args + breakpoint inside.
    let mut d = Dap::spawn();
    d.start(
        &lib,
        json!({ "entryPoint": "greet", "args": ["world"] }),
        &[4],
    );
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.top_frame_name(), "greet");
    assert_eq!(
        d.locals().get("name").map(String::as_str),
        Some("\"world\"")
    );
    d.cont();
    let mut out = String::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        out.push_str(ev["body"]["output"].as_str().unwrap_or(""));
    }
    assert!(out.contains("hello world"), "entry ran: {out}");

    // Unknown entry point errors clearly.
    let mut d = Dap::spawn();
    d.start(&lib, json!({ "entryPoint": "nope" }), &[]);
    let mut err = String::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        if ev["body"]["category"] == "stderr" {
            err.push_str(ev["body"]["output"].as_str().unwrap_or(""));
        }
    }
    assert!(err.contains("entry point `nope`"), "clear error: {err}");

    // No entry point and no `main`: run top-level only, terminate cleanly.
    let mut d = Dap::spawn();
    d.start(&lib, json!({}), &[]);
    assert_eq!(d.stop_or_term()["event"], "terminated");
}

#[test]
#[deps(NU)]
fn main_receives_args_and_flags() {
    let script = example("main_args.nu");
    let mut d = Dap::spawn();
    d.start(
        &script,
        json!({ "args": ["bob", "7", "--verbose", "--tag", "prod"] }),
        &[2],
    );
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    let loc = d.locals();
    assert_eq!(loc.get("verbose").map(String::as_str), Some("true"));
    assert_eq!(loc.get("tag").map(String::as_str), Some("\"prod\""));

    // Launching a required-arg main with none is a friendly error, not a dump.
    let mut d = Dap::spawn();
    d.start(&script, json!({}), &[]);
    let mut err = String::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        if ev["body"]["category"] == "stderr" {
            err.push_str(ev["body"]["output"].as_str().unwrap_or(""));
        }
    }
    assert!(
        err.contains("requires the argument `name`"),
        "friendly: {err}"
    );
    assert!(!err.contains("GenericError"), "no debug dump: {err}");
}

#[test]
#[deps(NU)]
fn deep_variables_hydrate_on_demand() {
    let deep = example("deep.nu");
    let mut d = Dap::spawn();
    d.start(&deep, json!({}), &[4]);
    d.event("stopped");

    let loc = d.locals();
    let uni = &loc["uni"];
    assert!(
        uni.contains("(130 chars)") && uni.contains('…'),
        "unicode-safe: {uni}"
    );

    // Walk `deep` six levels down to the leaf — past the eager horizon, so
    // this exercises on-demand hydration.
    d.send("variables", json!({ "variablesReference": 1 }));
    let vars = d.response("variables");
    let mut reference = vars["body"]["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "deep")
        .and_then(|v| v["variablesReference"].as_i64())
        .expect("deep expandable");
    let mut leaf = None;
    for _ in 0..7 {
        d.send("variables", json!({ "variablesReference": reference }));
        let r = d.response("variables");
        let child = r["body"]["variables"][0].clone();
        match child["variablesReference"].as_i64() {
            Some(next) if next > 0 => reference = next,
            _ => {
                leaf = child["value"].as_str().map(str::to_string);
                break;
            }
        }
    }
    assert_eq!(leaf.as_deref(), Some("\"bottom\""), "hydrated to depth 6");
}

#[test]
#[deps(NU, TESTBIN_INPUT_BYTES_LENGTH)]
fn external_command_gets_empty_stdin() -> nu_test_support::Result {
    let script = example("external_stdin.nu");
    let mut d = Dap::spawn_with_env(
        "NUSHELL_TEST_INPUT_BYTES_LENGTH",
        TESTBIN_INPUT_BYTES_LENGTH.path(),
    );
    d.start(&script, json!({}), &[]);
    // Reaching `terminated` (rather than the watchdog killing a hung process)
    // is itself the "external got EOF, didn't block" assertion.
    let mut out = String::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        out.push_str(ev["body"]["output"].as_str().unwrap_or(""));
    }
    assert!(
        out.contains("before")
            && out.lines().any(|line| line.trim() == "0")
            && out.contains("after"),
        "out: {out}"
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn startup_config_reaches_the_debuggee() -> nu_test_support::Result {
    // The adapter evaluates the user's startup files before serving, so a
    // debugged script sees the same shell the user has: custom commands and
    // `$env` from config.nu, not a bare engine.
    Playground::setup("nu_dap_config", |dirs, _| {
        let config = dirs.test().join("config.nu");
        std::fs::write(
            &config,
            "def dap-config-helper [] { \"helper-ran\" }\n$env.DAP_FROM_CONFIG = \"env-ok\"\n",
        )?;

        let script = dirs.test().join("uses_config.nu");
        std::fs::write(
            &script,
            "print (dap-config-helper)\nprint $env.DAP_FROM_CONFIG\n",
        )?;

        let mut d = Dap::spawn_with_args(["--config", &config.to_string_lossy()]);
        d.start(&script.to_string_lossy(), json!({}), &[]);

        let mut out = String::new();
        loop {
            let ev = d
                .recv_until(|m| {
                    m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
                })
                .expect("output or terminated");
            if ev["event"] == "terminated" {
                break;
            }
            out.push_str(ev["body"]["output"].as_str().unwrap_or(""));
        }
        assert!(out.contains("helper-ran"), "config.nu command: {out}");
        assert!(out.contains("env-ok"), "config.nu env var: {out}");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn hot_restart_reruns_in_the_same_session() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[14]); // `mut total = 0` — hit once per run

    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.top_line(), 14);

    d.send("restart", json!({}));
    d.response("restart");
    // A fresh run re-hits the breakpoint with NO terminated event in between.
    let ev = d.stop_or_term();
    assert_eq!(ev["event"], "stopped");
    assert_eq!(ev["body"]["reason"], "breakpoint");
    assert_eq!(d.top_line(), 14);

    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
}

#[test]
#[deps(NU)]
fn last_line_breakpoint_does_not_refire() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[22]); // `print $summary` — the last statement

    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");
    assert_eq!(d.top_line(), 22);
    // Continue must terminate, not re-stop on a stray line-1 "arrival".
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
}

#[test]
#[deps(NU)]
fn builtin_pipe_stage_walk_and_env_mutation() {
    let pipeline = example("pipeline.nu");
    let mut d = Dap::spawn();
    d.start(&pipeline, json!({}), &[10]); // after `$env.NU_DAP_TEST = ...` (line 8)
    d.event("stopped");

    // Runtime `$env.X = …` mutation is visible in Globals -> $env.
    d.send("variables", json!({ "variablesReference": 6 }));
    let g = d.response("variables");
    let env_ref = g["body"]["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "$env")
        .and_then(|v| v["variablesReference"].as_i64())
        .expect("$env expandable");
    let env = d.variables(env_ref);
    assert_eq!(
        env.get("NU_DAP_TEST").map(String::as_str),
        Some("\"hello-env\"")
    );

    // Walk the all-builtin pipeline stage by stage: F11 stops at each call
    // with `in → <cmd>` showing the value flowing in.
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": pipeline }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");
    let mut stages = Vec::new();
    for _ in 0..3 {
        d.send("stepIn", json!({ "threadId": 1 }));
        d.response("stepIn");
        if d.stop_or_term()["event"] != "stopped" {
            break;
        }
        let pipe = d.variables(2);
        if let Some((k, v)) = pipe.iter().find(|(k, _)| k.starts_with("in → ")) {
            stages.push((k.clone(), v.clone()));
        }
    }
    assert_eq!(
        stages,
        vec![
            ("in → split row".to_string(), "\"a-b\"".to_string()),
            ("in → get".to_string(), "\"<list stream>\"".to_string()),
            ("in → str upcase".to_string(), "\"a\"".to_string()),
        ]
    );
}

#[test]
#[deps(NU)]
fn leaf_values_visualize_by_container_and_name() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[28]); // blob / payload / markup in scope
    d.event("stopped");

    // Binary leaf: inline hex preview, and nuDapVisualize -> a hex marker.
    let loc = d.locals();
    let blob = &loc["blob"];
    assert!(
        blob.starts_with("0x[de ad be ef") && blob.contains("(24 bytes)"),
        "blob: {blob}"
    );
    d.send(
        "nuDapVisualize",
        json!({ "containerReference": 1, "name": "blob" }),
    );
    let viz = d.response("nuDapVisualize");
    assert_eq!(viz["body"]["value"]["length"], 24);
    assert_eq!(
        viz["body"]["value"]["$nuBinary"],
        "deadbeefcafebabe00112233445566778899aabbccddeeff"
    );

    // String leaf: nuDapVisualize returns the full string.
    d.send(
        "nuDapVisualize",
        json!({ "containerReference": 1, "name": "payload" }),
    );
    let viz = d.response("nuDapVisualize");
    let s = viz["body"]["value"].as_str().unwrap();
    assert!(s.starts_with('{') && s.contains("\"user\""), "payload: {s}");
}

#[test]
#[deps(NU)]
fn time_travel_reaches_pipe_stages_and_survives_a_tiny_buffer() {
    let demo = example("demo.nu");

    // Pipe-stage granularity going back: break after `$files | length`
    // (line 22) and Step Back until a Pipeline `in → cmd` is exposed.
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[22]);
    d.event("stopped");
    let mut saw_pipe_stage = false;
    for _ in 0..20 {
        d.send("stepBack", json!({ "threadId": 1 }));
        d.response("stepBack");
        if d.stop_or_term()["event"] != "stopped" {
            break;
        }
        if d.variables(2).keys().any(|k| k.starts_with("in → ")) {
            saw_pipe_stage = true;
            break;
        }
    }
    assert!(
        saw_pipe_stage,
        "Step Back lands on a pipe stage with `in → cmd`"
    );
    drop(d);

    // A tiny ring buffer must not crash; stepBack still works and the run
    // still finishes.
    let mut d = Dap::spawn();
    d.start(&demo, json!({ "timeTravelMaxSteps": 2 }), &[17]);
    d.event("stopped");
    for _ in 0..5 {
        d.send("stepBack", json!({ "threadId": 1 }));
        d.response("stepBack");
        d.recv_until(|m| m["type"] == "event" && m["event"] == "stopped");
    }
    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");
    d.cont();
    // May land back at the frontier once before terminating.
    let ev = d.stop_or_term();
    if ev["event"] == "stopped" {
        d.cont();
        assert_eq!(d.stop_or_term()["event"], "terminated");
    } else {
        assert_eq!(ev["event"], "terminated");
    }
}

#[test]
#[deps(NU)]
fn interactive_input_becomes_a_prompt() {
    let dir = std::env::temp_dir();
    let script = dir.join("nu_dap_input.nu");
    std::fs::write(
        &script,
        "let x = ([alpha beta gamma] | input list \"pick\")\nprint $\"picked:($x)\"\n",
    )
    .unwrap();
    let script = script.to_string_lossy().into_owned();

    let mut d = Dap::spawn();
    d.start(&script, json!({}), &[]);
    let ui = d.event("nuDapUi");
    assert_eq!(ui["body"]["kind"], "list");
    assert_eq!(ui["body"]["items"][1], "beta");
    let id = ui["body"]["id"].clone();
    d.send("nuDapUiReply", json!({ "id": id, "index": 1 }));

    let mut out = String::new();
    loop {
        let ev = d.recv_until(|m| {
            m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
        });
        let ev = ev.unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        out.push_str(ev["body"]["output"].as_str().unwrap_or(""));
    }
    assert!(out.contains("picked:beta"), "answer reached script: {out}");
    let _ = std::fs::remove_file(&script);
}

#[test]
#[deps(NU)]
fn input_box_returns_typed_text_and_listen_is_unsupported() {
    let dir = std::env::temp_dir();

    // `input` -> an input box whose answer flows back into the script.
    let box_script = dir.join("nu_dap_inputbox.nu");
    std::fs::write(
        &box_script,
        "let n = (input \"name?\")\nprint $\"hi:($n)\"\n",
    )
    .unwrap();
    let box_script = box_script.to_string_lossy().into_owned();
    let mut d = Dap::spawn();
    d.start(&box_script, json!({}), &[]);
    let ui = d.event("nuDapUi");
    assert_eq!(ui["body"]["kind"], "text");
    d.send(
        "nuDapUiReply",
        json!({ "id": ui["body"]["id"].clone(), "value": "world" }),
    );
    let mut out = String::new();
    loop {
        let ev = d
            .recv_until(|m| {
                m["type"] == "event" && (m["event"] == "output" || m["event"] == "terminated")
            })
            .unwrap();
        if ev["event"] == "terminated" {
            break;
        }
        out.push_str(ev["body"]["output"].as_str().unwrap_or(""));
    }
    assert!(out.contains("hi:world"), "input box answer: {out}");
    let _ = std::fs::remove_file(&box_script);

    // `input listen` (raw key events) has no VS Code equivalent -> clean error.
    let listen_script = dir.join("nu_dap_listen.nu");
    std::fs::write(&listen_script, "input listen\n").unwrap();
    let listen_script = listen_script.to_string_lossy().into_owned();
    let mut d = Dap::spawn();
    d.start(&listen_script, json!({}), &[]);
    let ev = d.event("stopped");
    assert_eq!(ev["body"]["reason"], "exception");
    assert!(
        ev["body"]["text"]
            .as_str()
            .unwrap()
            .contains("`input listen` is not supported"),
        "listen error: {}",
        ev["body"]["text"]
    );
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
    let _ = std::fs::remove_file(&listen_script);
}

/// Two requested breakpoints that snap to the same steppable line: the first
/// wins, the second is reported unverified (never silently dropped, and never
/// overwriting the winner).
#[test]
#[deps(NU)]
fn breakpoints_colliding_on_one_line_report_the_loser() {
    let demo = example("demo.nu");
    let mut d = Dap::spawn();
    d.start(&demo, json!({}), &[21]);
    assert_eq!(d.event("stopped")["body"]["reason"], "breakpoint");

    // Line 20 is blank, so it snaps forward onto 21 — which the second
    // requested breakpoint asks for directly. Parsing is done by now, so the
    // collision is resolved in the setBreakpoints response itself.
    d.send(
        "setBreakpoints",
        json!({
            "source": { "path": demo },
            "breakpoints": [{ "line": 20 }, { "line": 21 }],
        }),
    );
    let resp = d.response("setBreakpoints");
    let bps = resp["body"]["breakpoints"].as_array().expect("breakpoints");
    assert_eq!(bps.len(), 2, "one entry per requested breakpoint: {resp}");
    assert_eq!(bps[0]["verified"], true);
    assert_eq!(bps[0]["line"], 21, "snapped forward off the blank line");
    assert_eq!(bps[1]["verified"], false, "loser is not verified: {resp}");
    assert_eq!(bps[1]["line"], 21, "reported where it was requested");
    assert!(
        bps[1]["message"].as_str().unwrap_or("").contains("21"),
        "loser explains itself: {resp}"
    );
    assert_ne!(bps[0]["id"], bps[1]["id"], "distinct ids");

    d.send(
        "setBreakpoints",
        json!({ "source": { "path": demo }, "breakpoints": [] }),
    );
    d.response("setBreakpoints");
    d.cont();
    assert_eq!(d.stop_or_term()["event"], "terminated");
}
