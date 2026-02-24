use itertools::Itertools;
use nu_engine::command_prelude::*;
use nu_protocol::{Config, Range};
use std::{io::Cursor, iter::Peekable, str::CharIndices, sync::Arc};

type Input<'t> = Peekable<CharIndices<'t>>;

/// Helper function to check if a character is a box drawing character.
/// Includes Unicode box drawing symbols (horizontal, vertical, intersections, corners)
/// as well as ASCII equivalents like '-' and '|'.
fn is_box_char(c: char) -> bool {
    matches!(
        c,
        // Horizontal box drawing characters (Unicode and ASCII)
        '─' | '━' | '┄' | '┅' | '┈' | '┉' | '-' | '=' |
        // Vertical box drawing characters (Unicode and ASCII)
        '│' | '┃' | '┆' | '┇' | '┊' | '┋' | '|' |
        // Box intersection and corner characters
        '+' | '├' | '┤' | '┬' | '┴' | '┼' | '┌' | '┐' | '└' | '┘'
    )
}

/// Attempts to automatically split text into multiple columns.
///
/// This command parses tabular data from strings or passes through existing tables.
/// When `--ignore-box-chars` is used, it ignores separator lines and cleans box drawing characters from tokens.
#[derive(Clone)]
pub struct DetectColumns;

impl Command for DetectColumns {
    fn name(&self) -> &str {
        "detect columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("detect columns")
            .named(
                "skip",
                SyntaxShape::Int,
                "Number of rows to skip before detecting.",
                Some('s'),
            )
            .input_output_types(vec![
                (Type::String, Type::table()),
                (Type::table(), Type::table()),
            ])
            .switch("no-headers", "Don't detect headers.", Some('n'))
            .switch(
                "ignore-box-chars",
                "Ignore lines consisting entirely of box drawing characters and clean box characters from tokens.",
                Some('i'),
            )
            .named(
                "combine-columns",
                SyntaxShape::Range,
                "Columns to be combined; listed as a range.",
                Some('c'),
            )
            .switch(
                "guess",
                "Detect columns by guessing width, it may be useful if default one doesn't work.",
                None,
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Attempt to automatically split text into multiple columns."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["split", "tabular"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "use --guess if you find default algorithm not working",
                example: r"
'Filesystem     1K-blocks      Used Available Use% Mounted on
none             8150224         4   8150220   1% /mnt/c' | detect columns --guess",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "Filesystem" => Value::test_string("none"),
                    "1K-blocks" => Value::test_string("8150224"),
                    "Used" => Value::test_string("4"),
                    "Available" => Value::test_string("8150220"),
                    "Use%" => Value::test_string("1%"),
                    "Mounted on" => Value::test_string("/mnt/c")
                })])),
            },
            Example {
                description: "detect columns with no headers",
                example: "'a b c' | detect columns  --no-headers",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                        "column0" => Value::test_string("a"),
                        "column1" => Value::test_string("b"),
                        "column2" => Value::test_string("c"),
                })])),
            },
            Example {
                description: "",
                example: "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns 0..1 ",
                result: None,
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example: "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns -2..-1 ",
                result: None,
            },
            Example {
                description: "Splits a multi-line string into columns with headers detected",
                example: "$'c1 c2 c3 c4 c5(char nl)a b c d e' | detect columns --combine-columns 2.. ",
                result: None,
            },
            Example {
                description: "Parse external ls command and combine columns for datetime",
                example: "^ls -lh | detect columns --no-headers --skip 1 --combine-columns 5..7",
                result: None,
            },
            Example {
                description: "Table literal input is passed through unchanged",
                example: "[[name, age]; [Alice, 25]] | detect columns",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::test_string("Alice"),
                    "age" => Value::test_int(25)
                })])),
            },
            Example {
                description: "List of records input is passed through unchanged",
                example: "[{name: Alice, age: 25}, {name: Bob, age: 30}] | detect columns",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "name" => Value::test_string("Alice"),
                        "age" => Value::test_int(25)
                    }),
                    Value::test_record(record! {
                        "name" => Value::test_string("Bob"),
                        "age" => Value::test_int(30)
                    }),
                ])),
            },
            Example {
                description: "Parse a box-bordered table by ignoring separator lines and using header positions",
                example: r#""+-------+-------+
| col1  | col2  |
+-------+-------+
| a     | b     |
+-------+-------+" | detect columns --ignore-box-chars"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "col1" => Value::test_string("a"),
                    "col2" => Value::test_string("b"),
                })])),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Extract command arguments
        let num_rows_to_skip: Option<usize> = call.get_flag(engine_state, stack, "skip")?;
        let noheader = call.has_flag(engine_state, stack, "no-headers")?;
        let range: Option<Range> = call.get_flag(engine_state, stack, "combine-columns")?;
        let ignore_box_chars = call.has_flag(engine_state, stack, "ignore-box-chars")?;
        let config = stack.get_config(engine_state);

        let args = Arguments {
            noheader,
            num_rows_to_skip,
            range,
            config,
            ignore_box_chars,
        };

        // Dispatch to appropriate implementation based on guess flag
        if call.has_flag(engine_state, stack, "guess")? {
            guess_width(engine_state, call, input, args)
        } else {
            detect_columns(engine_state, call, input, args)
        }
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let num_rows_to_skip: Option<usize> = call.get_flag_const(working_set, "skip")?;
        let noheader = call.has_flag_const(working_set, "no-headers")?;
        let range: Option<Range> = call.get_flag_const(working_set, "combine-columns")?;
        let ignore_box_chars = call.has_flag_const(working_set, "ignore-box-chars")?;
        let config = working_set.get_config().clone();

        let args = Arguments {
            noheader,
            num_rows_to_skip,
            range,
            config,
            ignore_box_chars,
        };

        if call.has_flag_const(working_set, "guess")? {
            guess_width(working_set.permanent(), call, input, args)
        } else {
            detect_columns(working_set.permanent(), call, input, args)
        }
    }
}

struct Arguments {
    num_rows_to_skip: Option<usize>,
    noheader: bool,
    range: Option<Range>,
    config: Arc<Config>,
    ignore_box_chars: bool,
}

fn guess_width(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    use super::guess_width::GuessWidth;
    let input_span = input.span().unwrap_or(call.head);

    let mut input = input.collect_string("", &args.config)?;
    if let Some(rows) = args.num_rows_to_skip {
        input = input.lines().skip(rows).map(|x| x.to_string()).join("\n");
    }

    // Apply box character filtering if requested
    if args.ignore_box_chars {
        let filtered_lines = filter_box_chars(input.lines().map(|s| s.to_string()));
        input = filtered_lines.join("\n");
    }

    let mut guess_width = GuessWidth::new_reader(Box::new(Cursor::new(input)));

    let result = guess_width.read_all();

    if result.is_empty() {
        return Ok(Value::nothing(input_span).into_pipeline_data());
    }
    if !args.noheader {
        let columns = result[0].clone();
        Ok(result
            .into_iter()
            .skip(1)
            .map(move |s| {
                let mut values: Vec<Value> = s
                    .into_iter()
                    .map(|v| Value::string(v, input_span))
                    .collect();
                // some rows may has less columns, fill it with ""
                for _ in values.len()..columns.len() {
                    values.push(Value::string("", input_span));
                }
                let record =
                    Record::from_raw_cols_vals(columns.clone(), values, input_span, input_span);
                match record {
                    Ok(r) => match &args.range {
                        Some(range) => merge_record(r, range, input_span),
                        None => Value::record(r, input_span),
                    },
                    Err(e) => Value::error(e, input_span),
                }
            })
            .into_pipeline_data(input_span, engine_state.signals().clone()))
    } else {
        let length = result[0].len();
        let columns: Vec<String> = (0..length).map(|n| format!("column{n}")).collect();
        Ok(result
            .into_iter()
            .map(move |s| {
                let mut values: Vec<Value> = s
                    .into_iter()
                    .map(|v| Value::string(v, input_span))
                    .collect();
                // some rows may has less columns, fill it with ""
                for _ in values.len()..columns.len() {
                    values.push(Value::string("", input_span));
                }
                let record =
                    Record::from_raw_cols_vals(columns.clone(), values, input_span, input_span);
                match record {
                    Ok(r) => match &args.range {
                        Some(range) => merge_record(r, range, input_span),
                        None => Value::record(r, input_span),
                    },
                    Err(e) => Value::error(e, input_span),
                }
            })
            .into_pipeline_data(input_span, engine_state.signals().clone()))
    }
}

/// Core function to detect columns from input data.
/// Handles different input types: passes through tables, parses strings.
/// Applies filtering and cleaning based on the ignore_box_chars flag.
fn detect_columns(
    _engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    let name_span = call.head;
    let input_span = input.span().unwrap_or(Span::unknown());

    // Handle different input types
    match input {
        // If input is already a table (list of records), pass it through unchanged
        PipelineData::Value(val, _) => {
            if let Value::List { vals, .. } = &val
                && vals.iter().all(|v| matches!(v, Value::Record { .. }))
            {
                return Ok(val.into_pipeline_data());
            }
            // Otherwise, coerce to string for parsing
            let input_str = val.coerce_str()?.to_string();
            process_string_input(input_str, args, name_span, input_span)
        }
        // Table streams are passed through directly
        PipelineData::ListStream(_, _) => Ok(input),
        // External command output is collected as string
        PipelineData::ByteStream(_, _) => {
            let input_str = input.collect_string("", &args.config)?;
            process_string_input(input_str, args, name_span, input_span)
        }
        // Empty input yields empty string
        PipelineData::Empty => Ok(PipelineData::empty()),
    }
}

/// Process string input for column detection.
fn process_string_input(
    input_str: String,
    args: Arguments,
    name_span: Span,
    input_span: Span,
) -> Result<PipelineData, ShellError> {
    // Split input string into lines and skip the specified number of rows
    let lines_iter = input_str
        .lines()
        .skip(args.num_rows_to_skip.unwrap_or_default());

    // Conditionally filter out lines consisting entirely of box drawing characters
    // and clean box characters from the remaining lines
    // This helps clean up tabular output from commands like `iptab` that use box drawings
    let filtered_lines: Vec<_> = if args.ignore_box_chars {
        filter_box_chars(lines_iter.map(|s| s.to_string()))
    } else {
        // No filtering: pass through all lines as-is
        lines_iter.map(|x| x.to_string()).collect()
    };

    let mut lines = filtered_lines.into_iter();
    let header_line = lines.next();

    if let Some(header_line) = header_line {
        if args.ignore_box_chars {
            process_with_box_filter(header_line, lines, args, name_span, input_span)
        } else {
            process_standard(header_line, lines, args, name_span, input_span)
        }
    } else {
        Ok(PipelineData::empty())
    }
}

/// Process input when ignore_box_chars is enabled.
/// Handles both position-based and whitespace-based splitting depending on table format.
fn process_with_box_filter(
    header_line: String,
    lines: impl Iterator<Item = String>,
    args: Arguments,
    name_span: Span,
    input_span: Span,
) -> Result<PipelineData, ShellError> {
    // Check if the header line contains internal | separators
    // If so, replace them with spaces so whitespace-based detection works
    let has_internal_separators = header_line.contains('|') || header_line.contains('│');

    let (processed_headers, processed_lines): (String, Vec<String>) = if has_internal_separators {
        // Replace internal | with spaces for whitespace-based splitting
        let replace_separators = |s: &str| {
            s.chars()
                .map(|c| if c == '|' || c == '│' { ' ' } else { c })
                .collect::<String>()
        };
        (
            replace_separators(&header_line),
            lines.map(|line| replace_separators(&line)).collect(),
        )
    } else {
        // No internal separators - use position-based splitting
        (header_line.clone(), lines.collect())
    };

    // Use position-based splitting for tables without internal separators (like iptab)
    if !has_internal_separators {
        let header_positions = find_header_positions(&header_line);

        if header_positions.is_empty() {
            return Ok(PipelineData::empty());
        }

        // Extract header names
        let mut header_names: Vec<String> = header_positions
            .iter()
            .map(|(_, name)| name.clone())
            .collect();

        if args.noheader {
            for (i, name) in header_names.iter_mut().enumerate() {
                *name = format!("column{i}");
            }
        }

        // Check for duplicate column names
        check_duplicate_string_headers(&header_names, input_span, name_span)?;

        // Collect all lines for processing
        let all_lines: Vec<_> = args
            .noheader
            .then_some(header_line.clone())
            .into_iter()
            .chain(processed_lines)
            .collect();

        return Ok(Value::list(
            all_lines
                .into_iter()
                .map(|line| {
                    let values = split_line_by_positions(&line, &header_positions);
                    let mut record = Record::new();

                    for (header, val) in header_names.iter().zip(values.iter()) {
                        record.push(header, Value::string(val, name_span));
                    }

                    // Fill in missing columns with empty strings
                    for header in header_names.iter().skip(values.len()) {
                        record.push(header, Value::string("", name_span));
                    }

                    Ok::<Value, ShellError>(match &args.range {
                        Some(range) => merge_record(record, range, name_span),
                        None => Value::record(record, name_span),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            name_span,
        )
        .into_pipeline_data());
    }

    // Tables with internal separators: use whitespace-based splitting on processed data
    let mut headers = find_columns(&processed_headers);

    if args.noheader {
        for header in headers.iter_mut().enumerate() {
            header.1.item = format!("column{}", header.0);
        }
    }

    // Check for duplicate column names
    check_duplicate_headers(&headers, input_span, name_span)?;

    // Collect all lines for processing
    let all_lines: Vec<_> = args
        .noheader
        .then_some(processed_headers.clone())
        .into_iter()
        .chain(processed_lines)
        .collect();

    Ok(Value::list(
        all_lines
            .into_iter()
            .map(|line| {
                let row = find_columns(&line);
                let mut record = Record::new();

                for (header, val) in headers.iter().zip(row.iter()) {
                    record.push(&header.item, Value::string(&val.item, name_span));
                }

                // Fill in missing columns with empty strings
                for header in headers.iter().skip(row.len()) {
                    record.push(&header.item, Value::string("", name_span));
                }

                Ok::<Value, ShellError>(match &args.range {
                    Some(range) => merge_record(record, range, name_span),
                    None => Value::record(record, name_span),
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        name_span,
    )
    .into_pipeline_data())
}

/// Process input with standard whitespace-based column detection.
fn process_standard(
    header_line: String,
    lines: impl Iterator<Item = String>,
    args: Arguments,
    name_span: Span,
    input_span: Span,
) -> Result<PipelineData, ShellError> {
    // Standard whitespace-based column detection
    let mut headers = find_columns(&header_line);

    if args.noheader {
        for header in headers.iter_mut().enumerate() {
            header.1.item = format!("column{}", header.0);
        }
    }

    // Check for duplicate column names - this would create an invalid record
    check_duplicate_headers(&headers, input_span, name_span)?;

    // Collect remaining lines
    let remaining_lines: Vec<_> = lines.collect();

    // Check if column detection is working: if the first data row doesn't match
    // the header structure, detection has failed and we should output all lines
    // in a consistent "data" column to preserve the original data.
    let detection_failed = remaining_lines
        .first()
        .is_some_and(|first_line| find_columns(first_line).len() != headers.len());

    // When detection fails, include ALL original lines (including the first "header" line)
    // When detection succeeds, only include header line if --no-headers was specified
    let all_lines: Vec<_> = if detection_failed {
        // Include the original first line since detection failed
        std::iter::once(header_line.clone())
            .chain(remaining_lines)
            .collect()
    } else {
        // Detection succeeded - only include first line if --no-headers
        args.noheader
            .then_some(header_line.clone())
            .into_iter()
            .chain(remaining_lines)
            .collect()
    };

    Ok(Value::list(
        all_lines
            .into_iter()
            .map(move |x| {
                let row = find_columns(&x);

                let mut record = Record::new();

                if !detection_failed && headers.len() == row.len() {
                    for (header, val) in headers.iter().zip(row.iter()) {
                        record.push(&header.item, Value::string(&val.item, name_span));
                    }
                } else {
                    // Output the raw data - either detection failed or row doesn't match
                    record.push("data", Value::string(&x, name_span));
                }

                Ok::<Value, ShellError>(match &args.range {
                    Some(range) => merge_record(record, range, name_span),
                    None => Value::record(record, name_span),
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        name_span,
    )
    .into_pipeline_data())
}

pub fn find_columns(input: &str) -> Vec<Spanned<String>> {
    // For space-separated format, use the original baseline method
    let mut chars = input.char_indices().peekable();
    let mut output = vec![];

    while let Some((_, c)) = chars.peek() {
        if c.is_whitespace() {
            // If the next character is non-newline whitespace, skip it.
            let _ = chars.next();
        } else {
            // Otherwise, try to consume an unclassified token.
            let result = baseline(&mut chars);
            output.push(result);
        }
    }

    output
}

/// Return `true` if any of the given string‑like items contains duplicates.
///
/// The generic form accepts anything whose items implement `AsRef<str>`, which
/// includes `&str`, `String`, and `Spanned<String>` (via `.item`).
///
/// We allocate owned `String`s in the hash set; this keeps lifetimes simple and
/// avoids borrowing issues when the input iterator produces temporaries.
fn has_duplicate_names<I, S>(iter: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut set = std::collections::HashSet::new();
    for item in iter {
        let s = item.as_ref();
        if !set.insert(s.to_string()) {
            return true;
        }
    }
    false
}

/// Check for duplicate column names and return an error if found.
fn check_duplicate_headers(
    headers: &[Spanned<String>],
    input_span: Span,
    name_span: Span,
) -> Result<(), ShellError> {
    if has_duplicate_names(headers.iter().map(|h| &h.item)) {
        Err(ShellError::ColumnDetectionFailure {
            bad_value: input_span,
            failure_site: name_span,
        })
    } else {
        Ok(())
    }
}

/// Check for duplicate column names in string headers and return an error if found.
fn check_duplicate_string_headers(
    headers: &[String],
    input_span: Span,
    name_span: Span,
) -> Result<(), ShellError> {
    if has_duplicate_names(headers.iter().map(|s| s.as_str())) {
        Err(ShellError::ColumnDetectionFailure {
            bad_value: input_span,
            failure_site: name_span,
        })
    } else {
        Ok(())
    }
}

/// Filter and clean box drawing characters from lines.
/// Returns filtered lines with box-only lines removed and border characters stripped.
fn filter_box_chars<I>(lines_iter: I) -> Vec<String>
where
    I: Iterator<Item = String>,
{
    lines_iter
        // Filter out lines where all non-whitespace characters are box drawing characters
        .filter(|r| !r.trim().chars().all(is_box_char))
        // Clean border characters from each line
        .map(|line| {
            let trimmed = line.trim();
            // Strip only leading border character (| or │) and one optional space
            let cleaned = trimmed
                .strip_prefix('|')
                .or_else(|| trimmed.strip_prefix('│'))
                .unwrap_or(trimmed);
            let cleaned = cleaned.strip_prefix(' ').unwrap_or(cleaned);
            // Strip only trailing border character and one optional space
            let cleaned = cleaned
                .strip_suffix('|')
                .or_else(|| cleaned.strip_suffix('│'))
                .unwrap_or(cleaned);
            let cleaned = cleaned.strip_suffix(' ').unwrap_or(cleaned);
            cleaned.to_string()
        })
        .collect()
}

/// Find column positions (start indices) from a header line.
/// Returns a vector of (start_position, header_name) pairs.
fn find_header_positions(header_line: &str) -> Vec<(usize, String)> {
    let mut positions = vec![];
    let mut in_word = false;
    let mut word_start = 0;
    let mut current_word = String::new();

    for (idx, c) in header_line.char_indices() {
        if c.is_whitespace() {
            if in_word {
                // End of a word
                positions.push((word_start, current_word.clone()));
                current_word.clear();
                in_word = false;
            }
        } else {
            if !in_word {
                // Start of a new word
                word_start = idx;
                in_word = true;
            }
            current_word.push(c);
        }
    }

    // Don't forget the last word if the line doesn't end with whitespace
    if in_word && !current_word.is_empty() {
        positions.push((word_start, current_word));
    }

    positions
}

/// Adjust an index to the nearest character boundary for the given string.
///
/// - if `backward` is true, walk *backwards* from `idx` until a valid boundary is
///   found (or zero is reached). this is used for column **starts**, since a
///   header-derived offset landing inside a multibyte char should be moved to the
///   beginning of that char.
/// - otherwise walk *forwards* until a valid boundary or the end of the string is
///   reached. this is used for column **ends** so that we don't truncate a character.
#[inline]
fn adjust_char_boundary(s: &str, idx: usize, backward: bool) -> usize {
    if s.is_char_boundary(idx) {
        return idx;
    }

    if backward {
        (0..idx).rev().find(|&i| s.is_char_boundary(i)).unwrap_or(0)
    } else {
        (idx..=s.len())
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(s.len())
    }
}

/// Given the raw header-derived byte `start`/`end` positions, compute a safe
/// (start,end) pair for `line`, clamped to `prev_end` to avoid overlap.  Both
/// returned indices are guaranteed to be valid char boundaries.
fn safe_slice_range(line: &str, start: usize, end: usize, prev_end: usize) -> (usize, usize) {
    let line_len = line.len();
    let actual_end = end.min(line_len);

    let mut safe_start = adjust_char_boundary(line, start, true);
    if safe_start < prev_end {
        safe_start = prev_end;
    }

    let mut safe_end = adjust_char_boundary(line, actual_end, false);
    if safe_end < safe_start {
        safe_end = safe_start;
    }

    (safe_start, safe_end)
}

/// Split a data line into columns based on header positions.
/// Each column's value is the substring from its header position to the next header position.
///
/// Note that the header positions are computed from the first line only and are
/// therefore byte offsets **in that header string**. subsequent rows may contain
/// wider characters (e.g. an ellipsis or accented letter) which makes those offsets
/// invalid for the later lines. we therefore adjust each start/end offset to a
/// valid character boundary *for the line being sliced* to avoid panics.
fn split_line_by_positions(line: &str, positions: &[(usize, String)]) -> Vec<String> {
    if positions.is_empty() {
        return vec![line.to_string()];
    }

    let mut values = vec![];
    let line_len = line.len();

    let mut prev_end = 0;
    for (i, (start, _)) in positions.iter().enumerate() {
        let start = *start;
        let end = if i + 1 < positions.len() {
            positions[i + 1].0
        } else {
            line_len
        };

        if start < line_len {
            let (safe_start, safe_end) = safe_slice_range(line, start, end, prev_end);
            let value = &line[safe_start..safe_end];
            values.push(value.trim().to_string());
            prev_end = safe_end;
        } else {
            values.push(String::new());
        }
    }

    values
}

#[derive(Clone, Copy)]
enum BlockKind {
    Parenthesis,
    Brace,
    Bracket,
}

/// Tokenizes a single "baseline" token from the input stream.
/// A baseline token is a sequence of characters that can span multiple lines,
/// but is bounded by whitespace, pipes, semicolons, or other shell syntax elements.
/// It handles string literals, nested delimiters (parentheses, braces, brackets),
/// and stops at terminating characters.
fn baseline(src: &mut Input) -> Spanned<String> {
    let mut token_contents = String::new();

    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    // This variable tracks the starting character of a string literal, so that
    // we remain inside the string literal lexer mode until we encounter the
    // closing quote.
    let mut quote_start: Option<char> = None;

    // This Vec tracks paired delimiters
    let mut block_level: Vec<BlockKind> = vec![];

    // A baseline token is terminated if it's not nested inside of a paired
    // delimiter and the next character is one of: `|`, `;`, `#` or any
    // whitespace.
    fn is_termination(block_level: &[BlockKind], c: char) -> bool {
        block_level.is_empty() && (c.is_whitespace())
    }

    // The process of slurping up a baseline token repeats:
    //
    // - String literal, which begins with `'`, `"` or `\``, and continues until
    //   the same character is encountered again.
    // - Delimiter pair, which begins with `[`, `(`, or `{`, and continues until
    //   the matching closing delimiter is found, skipping comments and string
    //   literals.
    // - When not nested inside of a delimiter pair, when a terminating
    //   character (whitespace, `|`, `;` or `#`) is encountered, the baseline
    //   token is done.
    // - Otherwise, accumulate the character into the current baseline token.
    while let Some((_, c)) = src.peek() {
        let c = *c;

        if quote_start.is_some() {
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if Some(c) == quote_start {
                quote_start = None;
            }
        } else if c == '\n' {
            if is_termination(&block_level, c) {
                break;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            // We encountered the opening quote of a string literal.
            quote_start = Some(c);
        } else if c == '[' {
            // We encountered an opening `[` delimiter.
            block_level.push(BlockKind::Bracket);
        } else if c == ']' {
            // We encountered a closing `]` delimiter. Pop off the opening `[`
            // delimiter.
            if let Some(BlockKind::Bracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '{' {
            // We encountered an opening `{` delimiter.
            block_level.push(BlockKind::Brace);
        } else if c == '}' {
            // We encountered a closing `}` delimiter. Pop off the opening `{`.
            if let Some(BlockKind::Brace) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '(' {
            // We enceountered an opening `(` delimiter.
            block_level.push(BlockKind::Parenthesis);
        } else if c == ')' {
            // We encountered a closing `)` delimiter. Pop off the opening `(`.
            if let Some(BlockKind::Parenthesis) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if is_termination(&block_level, c) {
            break;
        }

        // Otherwise, accumulate the character into the current token.
        token_contents.push(c);

        // Consume the character.
        let _ = src.next();
    }

    let span = Span::new(start_offset, start_offset + token_contents.len());

    // If there is still unclosed opening delimiters, close them and add
    // synthetic closing characters to the accumulated token.
    if block_level.last().is_some() {
        // let delim: char = (*block).closing();
        // let cause = ParseError::unexpected_eof(delim.to_string(), span);

        // while let Some(bk) = block_level.pop() {
        //     token_contents.push(bk.closing());
        // }

        return Spanned {
            item: token_contents,
            span,
        };
    }

    if quote_start.is_some() {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        // token_contents.push(delimiter);

        // return (
        //     token_contents.spanned(span),
        //     Some(ParseError::unexpected_eof(delimiter.to_string(), span)),
        // );
        return Spanned {
            item: token_contents,
            span,
        };
    }

    Spanned {
        item: token_contents,
        span,
    }
}

fn merge_record(record: Record, range: &Range, input_span: Span) -> Value {
    let (start_index, end_index) = match process_range(range, record.len(), input_span) {
        Ok(Some((l_idx, r_idx))) => (l_idx, r_idx),
        Ok(None) => return Value::record(record, input_span),
        Err(e) => return Value::error(e, input_span),
    };

    match merge_record_impl(record, start_index, end_index, input_span) {
        Ok(rec) => Value::record(rec, input_span),
        Err(err) => Value::error(err, input_span),
    }
}

fn process_range(
    range: &Range,
    length: usize,
    input_span: Span,
) -> Result<Option<(usize, usize)>, ShellError> {
    match nu_cmd_base::util::process_range(range) {
        Ok((l_idx, r_idx)) => {
            let l_idx = if l_idx < 0 {
                length as isize + l_idx
            } else {
                l_idx
            };

            let r_idx = if r_idx < 0 {
                length as isize + r_idx
            } else {
                r_idx
            };

            if !(l_idx <= r_idx && (r_idx >= 0 || l_idx < (length as isize))) {
                return Ok(None);
            }

            Ok(Some((
                l_idx.max(0) as usize,
                (r_idx as usize + 1).min(length),
            )))
        }
        Err(processing_error) => Err(processing_error("could not find range index", input_span)),
    }
}

fn merge_record_impl(
    record: Record,
    start_index: usize,
    end_index: usize,
    input_span: Span,
) -> Result<Record, ShellError> {
    let (mut cols, mut vals): (Vec<_>, Vec<_>) = record.into_iter().unzip();
    // Merge Columns
    ((start_index + 1)..(cols.len() - end_index + start_index + 1)).for_each(|idx| {
        cols.swap(idx, end_index - start_index - 1 + idx);
    });
    cols.truncate(cols.len() - end_index + start_index + 1);

    // Merge Values
    let combined = vals
        .iter()
        .take(end_index)
        .skip(start_index)
        .map(|v| v.coerce_str().unwrap_or_default())
        .join(" ");
    let binding = Value::string(combined, Span::unknown());
    let last_seg = vals.split_off(end_index);
    vals.truncate(start_index);
    vals.push(binding);
    vals.extend(last_seg);

    Record::from_raw_cols_vals(cols, vals, Span::unknown(), input_span)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(DetectColumns)
    }

    /// Ensure that splitting a line using a header offset that falls inside a
    /// multibyte character does not panic and produces a reasonable result. This
    /// mirrors the crash described in the issue where an ellipsis in a data row
    /// caused slicing to panic.
    #[test]
    fn split_line_by_positions_multibyte_boundary() {
        // `…` is three bytes long; choose an index in the middle of it.
        let line = "a…b";
        assert!(!line.is_char_boundary(2));

        // pretend the second column was discovered at byte offset 2
        let positions = vec![(0, "a".to_string()), (2, "b".to_string())];

        let cols = split_line_by_positions(line, &positions);
        // After clamping, the first column captures the ellipsis and the second
        // column begins at the byte boundary after it. result should be
        // ["a…", "b"].
        assert_eq!(cols, vec!["a…".to_string(), "b".to_string()]);
    }

    #[test]
    fn split_line_with_various_unicode() {
        // header positions for three simple space-separated columns
        let positions = find_header_positions("a b c");

        let examples = [
            "x é y",         // combining accent
            "x 😄 y",        // single emoji
            "x 👨‍👩‍👧‍👦 y",        // ZWJ family emoji
            "x 中 y",        // CJK character
            "x a\u{0301} y", // decomposed accent
        ];

        for &line in examples.iter() {
            // should never panic and should produce three columns; we don't assert
            // on the exact values because wide graphemes may be split unpredictably,
            // but the column count should remain stable.
            let cols = split_line_by_positions(line, &positions);
            assert_eq!(cols.len(), 3, "line produced wrong column count: {}", line);
        }
    }
}
