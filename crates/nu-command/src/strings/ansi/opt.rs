use ansiopt::{
    Minifier, Options,
    align::token_at,
    describe::{self, describe, visible},
    report::{
        Report, Tally, heat_lines, percent_saved, rewrites,
        term::{Paint, shade},
    },
    trace::Fate,
};
use nu_engine::command_prelude::*;
use nu_protocol::{
    DataSource, PipelineMetadata,
    shell_error::{bridge::ShellErrorBridge, generic::GenericError, io::IoError},
};
use std::{
    io::{ErrorKind, Read},
    ops::Range,
};

#[derive(Clone)]
pub struct AnsiOpt;

/// What `ansi opt` returns in place of the shortened input. Each has a flag of the same name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Show,
    Report(ReportView),
}

/// The views built from a [`Report`] of how the input and the output line up.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReportView {
    Stats,
    Diff,
    Explain,
    Hotspot,
    Html,
}

impl View {
    const ALL: [(&str, View); 6] = [
        ("stats", View::Report(ReportView::Stats)),
        ("show", View::Show),
        ("diff", View::Report(ReportView::Diff)),
        ("explain", View::Report(ReportView::Explain)),
        ("hotspot", View::Report(ReportView::Hotspot)),
        ("html", View::Report(ReportView::Html)),
    ];
}

impl Command for AnsiOpt {
    fn name(&self) -> &str {
        "ansi opt"
    }

    fn signature(&self) -> Signature {
        Signature::build("ansi opt")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::Binary),
                (Type::String, Type::record()),
                (Type::Binary, Type::record()),
                (Type::String, Type::table()),
                (Type::Binary, Type::table()),
                (Type::Binary, Type::String),
            ])
            .switch(
                "visual",
                "Only keep what is visible: spaces, line feeds and erased areas don't get colors they can't show.",
                None,
            )
            .switch(
                "rep",
                "Write runs of one character with REP (CSI n b).",
                None,
            )
            .switch(
                "compat",
                "Avoid shorthand that old terminals may reject: empty parameters, BEL ending strings.",
                Some('c'),
            )
            .switch(
                "stats",
                "Return the sizes and what kinds of bytes were saved, as a record.",
                Some('s'),
            )
            .switch(
                "show",
                "Return the result with its escape sequences made visible.",
                None,
            )
            .switch(
                "diff",
                "Return each place between two pieces of text where the sequences changed, and the bytes saved there, as a table.",
                None,
            )
            .switch(
                "explain",
                "Return what happened to every input sequence, and why, as a table, including sequences kept as they were.",
                None,
            )
            .switch(
                "hotspot",
                "Return each line with the escape bytes spent on it, its text shaded by cost, as a table.",
                None,
            )
            .switch(
                "html",
                "Return an HTML report combining all the views.",
                None,
            )
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "Shorten the ANSI escape sequences in a string without changing what a terminal shows."
    }

    fn extra_description(&self) -> &str {
        "Repeated resets, colors that are set again before anything uses them, and cursor movements that cancel out are removed, and what is left is written in its shortest form. Anything ansi opt doesn't understand is kept byte for byte. It assumes nothing about the terminal's state, so the result also looks the same wherever it is printed.

--visual --rep together give the smallest output, on terminals that support REP (what the ansiopt tool calls --aggressive). Neither is safe everywhere: --visual can drop colors from blank cells that some tools still copy or record, and a terminal without REP drops the repeated characters.

The result is meant to be written to a terminal as a whole. It can rely on colors carried over from one line to the next, so splitting it into lines (for example with `lines`) can lose colors.

A byte stream is shortened as it arrives. In that case the result is not guaranteed to be shorter than the input, which a whole string or binary is.

The flags --stats, --show, --diff, --explain, --hotspot and --html each return a view of what changed in place of the result, and only one can be used at a time."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "minify", "optimize", "shrink", "compress", "escape", "sgr", "color",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let options = options(engine_state, stack, call)?;
        let view = view(engine_state, stack, call)?;

        let metadata = input.metadata_ref().cloned();
        let (bytes, is_string) = match input {
            PipelineData::ByteStream(stream, metadata) => match view {
                // Without a view the stream can be shortened as it arrives.
                None => {
                    return Ok(PipelineData::byte_stream(
                        minify_stream(stream, options),
                        metadata,
                    ));
                }
                // A view looks the same for a string or binary input.
                Some(_) => (stream.into_bytes()?, false),
            },
            PipelineData::Value(Value::String { val, .. }, ..) => (val.into_bytes(), true),
            PipelineData::Value(Value::Binary { val, .. }, ..) => (val.into_owned(), false),
            PipelineData::Value(Value::Error { error, .. }, ..) => return Err(*error),
            input => return Err(input.unsupported_input_error("string or binary", head)),
        };

        let wants_trace = matches!(
            view,
            Some(View::Report(ReportView::Explain | ReportView::Html))
        );
        let minified = ansiopt::minify_detailed(&bytes, &options, wants_trace);
        let view = match view {
            None => {
                let value = if is_string {
                    // The optimizer never splits a character, so this only fails on a bug.
                    let output = String::from_utf8(minified.output).map_err(|_| {
                        ShellError::NonUtf8Custom {
                            msg: "shortening ANSI escape sequences produced invalid UTF-8".into(),
                            span: head,
                        }
                    })?;
                    Value::string(output, head)
                } else {
                    Value::binary(minified.output, head)
                };
                return Ok(value.into_pipeline_data_with_metadata(metadata));
            }
            Some(View::Show) => {
                return Ok(Value::string(show(&minified.output), head).into_pipeline_data());
            }
            Some(View::Report(view)) => view,
        };

        let report = Report::new(&bytes, &minified.output, minified.fell_back, minified.trace)
            .map_err(|mismatch| {
                ShellError::Generic(
                    GenericError::new(
                        "ansi opt could not line up its input with its output",
                        format!(
                            "they disagree at input byte {} and output byte {}",
                            mismatch.input_offset, mismatch.output_offset
                        ),
                        head,
                    )
                    .with_help("this is a bug in ansiopt; the result without a view is unaffected"),
                )
            })?;
        let value = match view {
            ReportView::Stats => stats(&report, head),
            ReportView::Diff => diff(&report, head),
            ReportView::Explain => explain(&report, head),
            ReportView::Hotspot => {
                let color = stack
                    .get_config(engine_state)
                    .use_ansi_coloring
                    .get(engine_state);
                hotspot(&report, color, head)
            }
            ReportView::Html => {
                let settings = ansiopt::report::html::Settings {
                    title: title(metadata.as_ref()),
                    options,
                    width: None,
                    height: None,
                };
                Value::string(ansiopt::report::html::html(&report, &settings), head)
            }
        };
        Ok(value.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let folded = |sequence, meaning, output| {
            Value::test_record(record! {
                "sequence" => Value::test_string(sequence),
                "meaning" => Value::test_string(meaning),
                "outcome" => Value::test_string("folded"),
                "output" => Value::test_string(output),
                "why" => Value::test_nothing(),
            })
        };
        vec![
            Example {
                description: "Shorten a string: the second reset does nothing, and the first and the red can be one sequence",
                example: "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)' | ansi opt",
                result: Some(Value::test_string("\u{1b}[;31mhi\u{1b}[m")),
            },
            Example {
                description: "Shorten an external command's colored output as it streams",
                example: "^ls --color=always | ansi opt",
                result: None,
            },
            Example {
                description: "Show the result with its escape sequences made visible",
                example: "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)' | ansi opt --show",
                result: Some(Value::test_string(r"\e[;31mhi\e[m")),
            },
            Example {
                description: "Only keep what is visible: spaces show no foreground color, so the words can stay red across them",
                example: r#""\e[0m\e[31mfoo \e[0m \e[31mbar\e[0m" | ansi opt --visual --show"#,
                result: Some(Value::test_string(r"\e[;31mfoo  bar\e[m")),
            },
            Example {
                description: "Write a run of one character with REP, for a terminal that supports it",
                example: "'────────' | ansi opt --rep --show",
                result: Some(Value::test_string(r"─\e[7b")),
            },
            Example {
                description: "Shorten as far as possible for a terminal that supports REP: spaces don't need their color, and repeated characters become REP",
                example: r#""\e[0m\e[31m──── \e[0m \e[31m────\e[0m" | ansi opt --visual --rep --show"#,
                result: Some(Value::test_string(r"\e[;31m─\e[3b  ─\e[3b\e[m")),
            },
            Example {
                description: "Avoid shorthand that old terminals may reject: the reset is written as 0 instead of left empty",
                example: "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)' | ansi opt --compat --show",
                result: Some(Value::test_string(r"\e[0;31mhi\e[m")),
            },
            Example {
                description: "See how many bytes were saved",
                example: "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)' | ansi opt --stats | get input output saved",
                result: Some(Value::test_list(vec![
                    Value::test_filesize(19),
                    Value::test_filesize(11),
                    Value::test_filesize(8),
                ])),
            },
            Example {
                description: "List each place where the sequences changed, and the bytes saved there",
                example: "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)' | ansi opt --diff",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "from" => Value::test_string(r"\e[0m \e[0m \e[31m"),
                        "to" => Value::test_string(r"\e[;31m"),
                        "saved" => Value::test_filesize(7),
                    }),
                    Value::test_record(record! {
                        "from" => Value::test_string(r"\e[0m"),
                        "to" => Value::test_string(r"\e[m"),
                        "saved" => Value::test_filesize(1),
                    }),
                ])),
            },
            Example {
                description: "List what happened to every escape sequence, and why any was left out",
                example: "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)' | ansi opt --explain",
                result: Some(Value::test_list(vec![
                    folded(r"\e[0m", "reset", r"\e[;31m"),
                    Value::test_record(record! {
                        "sequence" => Value::test_string(r"\e[0m"),
                        "meaning" => Value::test_string("reset"),
                        "outcome" => Value::test_string("no effect"),
                        "output" => Value::test_nothing(),
                        "why" => Value::test_string("already in effect"),
                    }),
                    folded(r"\e[31m", "fg red", r"\e[;31m"),
                    folded(r"\e[0m", "reset", r"\e[m"),
                ])),
            },
            Example {
                // No result: `text` is shaded only when coloring is on, which by default depends
                // on whether stdout is a terminal.
                description: "Find the lines of an external command's output that spend the most bytes on escape sequences",
                example: "^ls --color=always | ansi opt --hotspot | sort-by escapes_in --reverse | first 5",
                result: None,
            },
            Example {
                description: "Write an HTML report of what changed in a captured file",
                example: "open --raw capture.ansi | ansi opt --html | save report.html",
                result: None,
            },
        ]
    }
}

/// The optimizer options set by the flags.
fn options(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Options, ShellError> {
    Ok(Options {
        visual: call.has_flag(engine_state, stack, "visual")?,
        rep: call.has_flag(engine_state, stack, "rep")?,
        compat: call.has_flag(engine_state, stack, "compat")?,
        ..Options::default()
    })
}

/// The view flag given, if any. More than one is an error, naming the first two as typed.
fn view(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Option<View>, ShellError> {
    let mut chosen = Vec::new();
    for (flag, view) in View::ALL {
        if call.has_flag(engine_state, stack, flag)? {
            let span = call.get_flag_span(stack, flag).unwrap_or(call.head);
            chosen.push((span, flag, view));
        }
    }
    chosen.sort_by_key(|(span, ..)| span.start);
    match chosen[..] {
        [] => Ok(None),
        [(_, _, view)] => Ok(Some(view)),
        [(left_span, first, _), (right_span, second, _), ..] => {
            Err(ShellError::IncompatibleParameters {
                left_message: format!("--{first} returns one view"),
                left_span,
                right_message: format!("--{second} returns another"),
                right_span,
            })
        }
    }
}

/// Shortens a stream as it arrives, bringing the output up to date after every chunk, since the
/// producer may pause after any of them.
fn minify_stream(stream: ByteStream, options: Options) -> ByteStream {
    let span = stream.span();
    let signals = stream.signals().clone();
    let type_ = stream.type_();
    let Some(mut reader) = stream.reader() else {
        return ByteStream::read(std::io::empty(), span, signals, type_);
    };
    // `None` once the stream has ended.
    let mut minifier = Some(Minifier::new(options));
    let mut buf = vec![0; 64 * 1024];
    ByteStream::from_fn(span, signals, type_, move |out| {
        let Some(active) = minifier.as_mut() else {
            return Ok(false);
        };
        // A read cut short by a signal is tried again, as `ByteStream::into_bytes` does. An
        // error from upstream, or Ctrl-C, comes through the reader as an I/O error.
        let read = loop {
            match reader.read(&mut buf) {
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                read => break read,
            }
        }
        .map_err(|err| match ShellErrorBridge::try_from(err) {
            Ok(ShellErrorBridge(err)) => err,
            Err(err) => IoError::new(err, span, None).into(),
        })?;
        if read > 0 {
            out.extend(active.feed(&buf[..read]));
            out.extend(active.flush());
            return Ok(true);
        }
        // Writes out whatever the end of the stream left unfinished.
        if let Some(finished) = minifier.take() {
            out.extend(finished.finish());
        }
        Ok(false)
    })
}

/// The output with its escapes made visible, one line per line it prints, each line feed shown
/// as `\n` at the end of its line. Unlike `ansiopt --show`, a line feed at the end of the output
/// doesn't start an empty line.
fn show(output: &[u8]) -> String {
    output
        .split_inclusive(|&b| b == b'\n')
        .map(visible)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The report title: the name of the file the input came from, if known.
fn title(metadata: Option<&PipelineMetadata>) -> String {
    metadata
        .and_then(|metadata| match &metadata.data_source {
            DataSource::FilePath(path) => path.file_name(),
            _ => None,
        })
        .map_or_else(
            || "input".into(),
            |name| name.to_string_lossy().into_owned(),
        )
}

/// The sizes of the input and the output, the bytes and sequences of each kind in both, and
/// each kind of change with the bytes it saved, as a record.
fn stats(report: &Report, span: Span) -> Value {
    let (input, output) = (Tally::of(report.input), Tally::of(report.output));
    let breakdown = describe::Category::ALL
        .into_iter()
        .filter(|&category| input.bytes(category) + output.bytes(category) > 0)
        .map(|category| {
            Value::record(
                record! {
                    "category" => Value::string(category.label(), span),
                    "input" => Value::filesize(input.bytes(category) as i64, span),
                    "output" => Value::filesize(output.bytes(category) as i64, span),
                    "sequences_in" => Value::int(input.count(category) as i64, span),
                    "sequences_out" => Value::int(output.count(category) as i64, span),
                },
                span,
            )
        })
        .collect();
    // The numbers come first: `from` can be a long run of sequences, and a narrow table cuts
    // off the columns after it.
    let changes = rewrites(report, true)
        .into_iter()
        .map(|rewrite| {
            Value::record(
                record! {
                    "saved" => Value::filesize(rewrite.saved as i64, span),
                    "count" => Value::int(rewrite.count as i64, span),
                    "from" => Value::string(rewrite.from, span),
                    "to" => Value::string(rewrite.to, span),
                },
                span,
            )
        })
        .collect();
    let (bytes_in, bytes_out) = (report.input.len(), report.output.len());
    Value::record(
        record! {
            "input" => Value::filesize(bytes_in as i64, span),
            "output" => Value::filesize(bytes_out as i64, span),
            "saved" => Value::filesize(bytes_in as i64 - bytes_out as i64, span),
            "percent" => Value::float(percent_saved(bytes_in, bytes_out), span),
            "sequences_in" => Value::int(input.sequences() as i64, span),
            "sequences_out" => Value::int(output.sequences() as i64, span),
            "fell_back" => Value::bool(report.fell_back, span),
            "breakdown" => Value::list(breakdown, span),
            "changes" => Value::list(changes, span),
        },
        span,
    )
}

/// The sequences at `ranges` of `bytes`, made visible and joined by spaces.
fn sequences(bytes: &[u8], ranges: &[Range<usize>]) -> String {
    ranges
        .iter()
        .map(|range| visible(&bytes[range.clone()]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One row for each place between two pieces of text where the sequences changed: what the input
/// had there, what the output has, and the bytes saved. `--explain` says what each sequence
/// means.
fn diff(report: &Report, span: Span) -> Value {
    let rows = report
        .changed_gaps()
        .map(|(_, gap)| {
            Value::record(
                record! {
                    "from" => Value::string(sequences(report.input, &gap.input), span),
                    "to" => Value::string(sequences(report.output, &gap.output), span),
                    "saved" => Value::filesize(
                        gap.input_bytes() as i64 - gap.output_bytes() as i64,
                        span,
                    ),
                },
                span,
            )
        })
        .collect();
    Value::list(rows, span)
}

/// One row for each sequence of the input: what it means, what happened to it, what the output
/// has in its place and, where ansiopt gives one, why.
fn explain(report: &Report, span: Span) -> Value {
    let Some(trace) = &report.trace else {
        return Value::list(vec![], span);
    };
    let rows = trace
        .entries
        .iter()
        .filter(|entry| entry.fate != Fate::Text)
        .map(|entry| {
            let (outputs, why): (&[Range<usize>], Option<&str>) = match &entry.fate {
                Fate::Rewritten { output, why } => (std::slice::from_ref(output), Some(why)),
                Fate::Folded { outputs } => (outputs, None),
                Fate::Merged { output } => (std::slice::from_ref(output), None),
                Fate::NoEffect { why } | Fate::Superseded { why } | Fate::Barrier { why } => {
                    (&[], Some(why))
                }
                Fate::Pending | Fate::Text | Fate::Kept => (&[], None),
            };
            let sequence = visible(&report.input[entry.input.clone()]);
            let meaning = describe(&token_at(report.input, entry.input.clone()));
            let output = if outputs.is_empty() {
                Value::nothing(span)
            } else {
                Value::string(sequences(report.output, outputs), span)
            };
            // `why` is last: it is empty on most rows and a long sentence on the rest, so a
            // narrow table cuts it off rather than wrap every column around it.
            Value::record(
                record! {
                    "sequence" => Value::string(sequence, span),
                    "meaning" => Value::string(meaning, span),
                    "outcome" => Value::string(entry.fate.kind(), span),
                    "output" => output,
                    "why" => why.map_or(Value::nothing(span), |why| Value::string(why, span)),
                },
                span,
            )
        })
        .collect();
    Value::list(rows, span)
}

/// The totals come before the text, like the gutter of `ansiopt --hotspot`, so a line too long
/// for the table wraps or is cut off instead of the numbers.
fn hotspot(report: &Report, color: bool, span: Span) -> Value {
    let rows = heat_lines(report)
        .into_iter()
        .enumerate()
        .map(|(i, mut line)| {
            let (escapes_in, escapes_out) = (line.input(), line.output());
            // `shade` marks escape bytes after the last character with `⏎`, which reads as a
            // line feed; the totals still count them.
            line.end_input = 0;
            Value::record(
                record! {
                    "line" => Value::int((i + 1) as i64, span),
                    "escapes_in" => Value::filesize(escapes_in as i64, span),
                    "escapes_out" => Value::filesize(escapes_out as i64, span),
                    "text" => Value::string(shade(&line, Paint { color }), span),
                },
                span,
            )
        })
        .collect();
    Value::list(rows, span)
}

#[cfg(test)]
mod tests {
    use std::io::{self, ErrorKind, Read};

    use ansiopt::Options;
    use nu_protocol::{ByteStream, ByteStreamType, Signals, Span};

    use super::{AnsiOpt, minify_stream};

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(AnsiOpt)
    }

    /// Shortens a stream of these chunks, one read each.
    fn minify_chunks(chunks: &[&'static str]) -> ByteStream {
        let stream = ByteStream::from_iter(
            chunks.to_vec(),
            Span::test_data(),
            Signals::empty(),
            ByteStreamType::String,
        );
        minify_stream(stream, Options::default())
    }

    /// The first chunk's reset is written before the second chunk is read, and the second
    /// reset, split between the chunks, is put back together and dropped. The whole string at
    /// once would be `\e[;31mhi`.
    #[test]
    fn stream_writes_each_chunk_as_it_arrives() {
        let mut reader = minify_chunks(&["\x1b[0m\x1b[", "0m\x1b[31mhi"])
            .reader()
            .expect("a reader");
        let mut buf = vec![0; 64 * 1024];
        let reads: Vec<String> = std::iter::from_fn(|| match reader.read(&mut buf) {
            Ok(0) => None,
            Ok(read) => Some(String::from_utf8_lossy(&buf[..read]).into_owned()),
            Err(err) => panic!("{err}"),
        })
        .collect();
        assert_eq!(reads, ["\x1b[m", "\x1b[31mhi"]);
    }

    /// Reads `bytes`, after one read cut short by a signal.
    struct InterruptedOnce {
        interrupted: bool,
        bytes: &'static [u8],
    }

    impl Read for InterruptedOnce {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(ErrorKind::Interrupted.into());
            }
            self.bytes.read(buf)
        }
    }

    #[test]
    fn stream_retries_a_read_cut_short_by_a_signal() {
        let reader = InterruptedOnce {
            interrupted: false,
            bytes: b"\x1b[0m\x1b[0mhi",
        };
        let stream = ByteStream::read(
            reader,
            Span::test_data(),
            Signals::empty(),
            ByteStreamType::String,
        );
        let output = minify_stream(stream, Options::default()).into_bytes();
        assert_eq!(output.expect("no error"), b"\x1b[mhi");
    }
}
