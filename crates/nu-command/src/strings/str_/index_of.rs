use std::ops::Bound;

use crate::{grapheme_flags, grapheme_flags_const};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{IntRange, engine::StateWorkingSet};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

struct Arguments {
    end: bool,
    all: bool,
    substring: String,
    range: Option<Spanned<IntRange>>,
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct StrIndexOf;

impl Command for StrIndexOf {
    fn name(&self) -> &str {
        "str index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("str index-of")
            .input_output_types(vec![
                (Type::String, Type::one_of([Type::Int, Type::list(Type::Int)])),
                (
                    Type::List(Box::new(Type::String)),
                    Type::list(Type::one_of([Type::Int, Type::list(Type::Int)])),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("string", SyntaxShape::String, "The string to find in the input.")
            .switch(
                "grapheme-clusters",
                "Count indexes using grapheme clusters (all visible chars have length 1).",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "Count indexes using UTF-8 bytes (default; non-ASCII chars have length 2+).",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, search strings at the given cell paths, and replace with result.",
            )
            .named(
                "range",
                SyntaxShape::Range,
                "Optional start and/or end index.",
                Some('r'),
            )
            .switch("end", "Search from the end of the input.", Some('e'))
            .switch("all", "Returns all matched indices.", Some('a'))
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Returns start index of first occurrence of string in input, or -1 if no match."
    }

    fn extra_description(&self) -> &str {
        "Use --all to return a list of every matched index instead of just the first (or last, with --end). \
         Matches don't overlap, so --all --end can return a different set of indices than --all, not just the same set in reverse order."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["match", "find", "search"]
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
        let substring: Spanned<String> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            range: call.get_flag(engine_state, stack, "range")?,
            end: call.has_flag(engine_state, stack, "end")?,
            all: call.has_flag(engine_state, stack, "all")?,
            cell_paths,
            graphemes: grapheme_flags(engine_state, stack, call)?,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let substring: Spanned<String> = call.req_const(working_set, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            range: call.get_flag_const(working_set, stack, "range")?,
            end: call.has_flag_const(working_set, stack, "end")?,
            all: call.has_flag_const(working_set, stack, "all")?,
            cell_paths,
            graphemes: grapheme_flags_const(working_set, stack, call)?,
        };
        operate(
            action,
            args,
            input,
            call.head,
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Returns index of string in input.",
                example: " 'my_library.rb' | str index-of '.rb'",
                result: Some(Value::test_int(10)),
            },
            Example {
                description: "Count length using grapheme clusters.",
                example: "'🇯🇵ほげ ふが ぴよ' | str index-of --grapheme-clusters 'ふが'",
                result: Some(Value::test_int(4)),
            },
            Example {
                description: "A match that falls inside a grapheme cluster is reported as not found.",
                example: "'🇯🇵' | str index-of --grapheme-clusters '🇵'",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of string in input within a`rhs open range`.",
                example: " '.rb.rb' | str index-of '.rb' --range 1..",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Returns index of string in input within a lhs open range.",
                example: " '123456' | str index-of '6' --range ..4",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of string in input within a range.",
                example: " '123456' | str index-of '3' --range 1..4",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Returns index of string in input.",
                example: " '/this/is/some/path/file.txt' | str index-of '/' -e",
                result: Some(Value::test_int(18)),
            },
            Example {
                description: "Returns all matched indices.",
                example: " 'a.b.c.d' | str index-of '.' --all",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(3),
                    Value::test_int(5),
                ])),
            },
            Example {
                description: "Returns all matched indices, searching from the end.",
                example: " 'a.b.c.d' | str index-of '.' --all --end",
                result: Some(Value::test_list(vec![
                    Value::test_int(5),
                    Value::test_int(3),
                    Value::test_int(1),
                ])),
            },
        ]
    }
}

fn action(
    input: &Value,
    Arguments {
        substring,
        range,
        end,
        graphemes,
        all,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val: s, .. } => {
            let (search_str, start_index) = if let Some(spanned_range) = range {
                let range_span = spanned_range.span;
                let range = &spanned_range.item;

                let (start, end) = range.absolute_bounds(s.len());
                let s = match end {
                    Bound::Excluded(end) => s.get(start..end),
                    Bound::Included(end) => s.get(start..=end),
                    Bound::Unbounded => s.get(start..),
                };

                let s = match s {
                    Some(s) => s,
                    None => {
                        return Value::error(
                            ShellError::OutOfBounds {
                                left_flank: start.to_string(),
                                right_flank: match range.end() {
                                    Bound::Unbounded => "".to_string(),
                                    Bound::Included(end) => format!("={end}"),
                                    Bound::Excluded(end) => format!("<{end}"),
                                },
                                span: range_span,
                            },
                            head,
                        );
                    }
                };
                (s, start)
            } else {
                (s.as_str(), 0)
            };

            if *graphemes {
                if substring.is_empty() {
                    // An empty pattern matches at every grapheme boundary, including the
                    // position right after the last grapheme -- the same set
                    // `match_indices("")` produces at the byte level.
                    let total = s.graphemes(true).count() as i64;
                    let mut indices: Vec<i64> = (0..=total).collect();
                    if *end {
                        indices.reverse();
                    }
                    if *all {
                        Value::list(
                            indices.into_iter().map(|i| Value::int(i, head)).collect(),
                            head,
                        )
                    } else {
                        Value::int(indices.into_iter().next().unwrap_or(-1), head)
                    }
                } else {
                    // A match that doesn't start exactly on a grapheme boundary falls inside a
                    // cluster, so it isn't a real grapheme-level match. `grapheme_match_offsets`
                    // drops it and keeps looking one `char` further in, rather than skipping the
                    // whole pattern length and missing a real match that overlaps those bytes.
                    let hits =
                        grapheme_match_offsets(s, search_str, start_index, substring, *end, *all);
                    if *all {
                        let mut ascending = hits;
                        if *end {
                            ascending.reverse();
                        }
                        let mut indices = grapheme_indices_for(s, &ascending);
                        if *end {
                            indices.reverse();
                        }
                        Value::list(
                            indices.into_iter().map(|i| Value::int(i, head)).collect(),
                            head,
                        )
                    } else {
                        match hits.first() {
                            Some(&offset) => {
                                Value::int(grapheme_indices_for(s, &[offset])[0], head)
                            }
                            None => Value::int(-1, head),
                        }
                    }
                }
            } else if *all {
                let indices = match_offsets(search_str, substring, *end)
                    .into_iter()
                    .map(|offset| Value::int((offset + start_index) as i64, head));
                Value::list(indices.collect(), head)
            } else if let Some(result) = if *end {
                // When the -e flag is present, search using rfind instead of find.
                search_str.rfind(&**substring)
            } else {
                search_str.find(&**substring)
            } {
                Value::int((result + start_index) as i64, head)
            } else {
                Value::int(-1, head)
            }
        }
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}

/// Every non-overlapping start byte offset at which `pattern` matches within `haystack`. When
/// `from_end` is set, offsets are found working backward from the end, so they come out in
/// descending order (mirrors `bytes index-of --all --end`); matches don't overlap, so the set of
/// offsets found from the end can differ from the forward set, not just its order.
///
/// An empty `pattern` matches at every char boundary in `haystack`, including the position right
/// after the last char, same as `str::match_indices`/`rmatch_indices` with an empty pattern.
fn match_offsets(haystack: &str, pattern: &str, from_end: bool) -> Vec<usize> {
    if from_end {
        haystack.rmatch_indices(pattern).map(|(i, _)| i).collect()
    } else {
        haystack.match_indices(pattern).map(|(i, _)| i).collect()
    }
}

/// Every grapheme-boundary-aligned start byte offset (as an absolute index into `s`, not
/// `search_str`) at which the non-empty `substring` matches within `search_str`, a byte range of
/// `s` starting at `start_index`. Offsets come out in the same order `match_offsets` would report
/// them: ascending for a forward scan, descending when `from_end` is set.
///
/// A hit that starts inside a grapheme cluster doesn't count as a real match: the scan advances
/// by a single `char` and keeps looking from there, instead of skipping over the whole pattern's
/// length, so it doesn't walk past a genuine match that overlaps those same bytes.
///
/// Stops after the first valid hit when `want_all` is false, so a single lookup doesn't have to
/// scan the rest of `search_str` once a match has already been found.
fn grapheme_match_offsets(
    s: &str,
    search_str: &str,
    start_index: usize,
    substring: &str,
    from_end: bool,
    want_all: bool,
) -> Vec<usize> {
    let mut hits = Vec::new();
    if from_end {
        let last_char_len = substring.chars().next_back().map_or(1, char::len_utf8);
        let mut limit = search_str.len();
        while let Some(o) = search_str[..limit].rfind(substring) {
            if is_grapheme_boundary(s, o + start_index) {
                hits.push(o + start_index);
                if !want_all {
                    break;
                }
                limit = o;
            } else {
                // This hit fell inside a cluster, so give back the bytes it covered except for
                // the pattern's own last char -- a real match further left that overlaps those
                // bytes still needs a chance to be found.
                limit = o + substring.len() - last_char_len;
            }
        }
    } else {
        let mut from = 0;
        while let Some(i) = search_str[from..].find(substring) {
            let o = from + i;
            if is_grapheme_boundary(s, o + start_index) {
                hits.push(o + start_index);
                if !want_all {
                    break;
                }
                from = o + substring.len();
            } else {
                from = o + search_str[o..].chars().next().map_or(1, char::len_utf8);
            }
        }
    }
    hits
}

fn is_grapheme_boundary(s: &str, byte_offset: usize) -> bool {
    GraphemeCursor::new(byte_offset, s.len(), true)
        .is_boundary(s, 0)
        .unwrap_or(false)
}

/// Converts absolute byte offsets into `s` -- each guaranteed by its caller to be exactly some
/// grapheme's own start, as `grapheme_match_offsets` only ever returns boundary-validated offsets
/// -- to their grapheme indices. `ascending_offsets` must be sorted ascending; the whole call is a
/// single pass over `s`'s graphemes regardless of how many offsets are given, so scoring every
/// match found by `--all` still costs one walk of `s`, not one walk per match.
fn grapheme_indices_for(s: &str, ascending_offsets: &[usize]) -> Vec<i64> {
    let mut result = Vec::with_capacity(ascending_offsets.len());
    let mut iter = s.grapheme_indices(true).enumerate();
    let mut current = iter.next();
    for &offset in ascending_offsets {
        while let Some((_, (byte_offset, _))) = current {
            if byte_offset < offset {
                current = iter.next();
            } else {
                break;
            }
        }
        let (grapheme_index, _) = current.expect("offset is a valid grapheme boundary in s");
        result.push(grapheme_index as i64);
    }
    result
}

#[cfg(test)]
mod tests {
    use nu_protocol::ast::RangeInclusion;

    use super::*;
    use super::{Arguments, StrIndexOf, action};

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(StrIndexOf)
    }

    #[test]
    fn returns_index_of_substring() {
        let word = Value::test_string("Cargo.tomL");

        let options = Arguments {
            substring: String::from(".tomL"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());

        assert_eq!(actual, Value::test_int(5));
    }
    #[test]
    fn index_of_does_not_exist_in_string() {
        let word = Value::test_string("Cargo.tomL");

        let options = Arguments {
            substring: String::from("Lm"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());

        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn returns_index_of_next_substring() {
        let word = Value::test_string("Cargo.Cargo");
        let range = IntRange::new(
            Value::int(1, Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::nothing(Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("Cargo"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(6));
    }

    #[test]
    fn index_does_not_exist_due_to_end_index() {
        let word = Value::test_string("Cargo.Banana");
        let range = IntRange::new(
            Value::nothing(Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::int(5, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("Banana"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn returns_index_of_nums_in_middle_due_to_index_limit_from_both_ends() {
        let word = Value::test_string("123123123");
        let range = IntRange::new(
            Value::int(2, Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::int(6, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("123"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(3));
    }

    #[test]
    fn index_does_not_exists_due_to_strict_bounds() {
        let word = Value::test_string("123456");
        let range = IntRange::new(
            Value::int(2, Span::test_data()),
            Value::nothing(Span::test_data()),
            Value::int(5, Span::test_data()),
            RangeInclusion::RightExclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("1"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn use_utf8_bytes() {
        let word = Value::string(String::from("🇯🇵ほげ ふが ぴよ"), Span::test_data());

        let options = Arguments {
            substring: String::from("ふが"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(15));
    }

    #[test]
    fn returns_all_matching_indices() {
        let word = Value::test_string("a.b.c.d");

        let options = Arguments {
            substring: String::from("."),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(3),
                Value::test_int(5),
            ])
        );
    }

    #[test]
    fn returns_all_matching_indices_from_end() {
        let word = Value::test_string("a.b.c.d");

        let options = Arguments {
            substring: String::from("."),
            range: None,
            cell_paths: None,
            end: true,
            all: true,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list(vec![
                Value::test_int(5),
                Value::test_int(3),
                Value::test_int(1),
            ])
        );
    }

    #[test]
    fn returns_all_matching_indices_with_grapheme_clusters() {
        // Each "🇯🇵" is a single grapheme cluster but 8 UTF-8 bytes, so the byte index of each
        // 'a' (8, then 17) must differ from its grapheme index (1, then 3) -- same conversion
        // the single-match case already does, just applied to every match instead of one.
        let word = Value::test_string("🇯🇵a🇯🇵a🇯🇵");

        let options = Arguments {
            substring: String::from("a"),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: true,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list(vec![Value::test_int(1), Value::test_int(3)])
        );
    }

    #[test]
    fn all_grapheme_clusters_drops_matches_that_fall_inside_a_cluster() {
        // "🇯🇵" is a single grapheme cluster made of two 4-byte regional-indicator codepoints, so
        // a match of just the second one (the 'P') starts mid-cluster and isn't a real match at
        // the grapheme level. It should be dropped, not rounded to the next (or an invalid)
        // grapheme index.
        let cases = [("🇯🇵🇵", vec![1]), ("🇯🇵🇯🇵", vec![]), ("🇯🇵🇯🇵x", vec![])];

        for (input, expected) in cases {
            let word = Value::test_string(input);
            let options = Arguments {
                substring: String::from("🇵"),
                range: None,
                cell_paths: None,
                end: false,
                all: true,
                graphemes: true,
            };

            let actual = action(&word, &options, Span::test_data());
            assert_eq!(
                actual,
                Value::test_list(expected.into_iter().map(Value::test_int).collect()),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn single_grapheme_clusters_skips_a_match_that_falls_inside_a_cluster() {
        let word = Value::test_string("🇯🇵🇵");

        let options = Arguments {
            substring: String::from("🇵"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: true,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(1));
    }

    #[test]
    fn empty_pattern_matches_at_every_boundary() {
        let word = Value::test_string("abc");

        let single = Arguments {
            substring: String::new(),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };
        assert_eq!(
            action(&word, &single, Span::test_data()),
            Value::test_int(0)
        );

        let single_from_end = Arguments {
            end: true,
            ..single
        };
        assert_eq!(
            action(&word, &single_from_end, Span::test_data()),
            Value::test_int(3)
        );

        let all = Arguments {
            all: true,
            end: false,
            ..single_from_end
        };
        assert_eq!(
            action(&word, &all, Span::test_data()),
            Value::test_list([0, 1, 2, 3].into_iter().map(Value::test_int).collect())
        );
    }

    #[test]
    fn empty_pattern_with_grapheme_clusters_matches_at_every_grapheme_boundary() {
        // "🇯🇵" is one grapheme cluster made of two 4-byte codepoints, so the mid-cluster byte
        // boundary at offset 4 isn't a grapheme boundary and must be dropped, the same way a
        // mid-cluster substring match is.
        let word = Value::test_string("🇯🇵a");

        let options = Arguments {
            substring: String::new(),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: true,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(
            actual,
            Value::test_list([0, 1, 2].into_iter().map(Value::test_int).collect())
        );
    }

    #[test]
    fn index_is_not_a_char_boundary() {
        let word = Value::string(String::from("💛"), Span::test_data());

        let range = IntRange::new(
            Value::int(0, Span::test_data()),
            Value::int(1, Span::test_data()),
            Value::int(2, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::new(),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert!(actual.is_error());
    }

    #[test]
    fn index_is_out_of_bounds() {
        let word = Value::string(String::from("hello"), Span::test_data());

        let range = IntRange::new(
            Value::int(-1, Span::test_data()),
            Value::int(1, Span::test_data()),
            Value::int(3, Span::test_data()),
            RangeInclusion::Inclusive,
            Span::test_data(),
        )
        .expect("valid range");

        let spanned_range = Spanned {
            item: range,
            span: Span::test_data(),
        };

        let options = Arguments {
            substring: String::from("h"),

            range: Some(spanned_range),
            cell_paths: None,
            end: false,
            all: false,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn grapheme_clusters_finds_a_match_that_overlaps_a_mid_cluster_hit() {
        // "👨\u{200D}👩👩👩👩" is 4 clusters: 👨‍👩 (a ZWJ sequence), 👩, 👩, 👩. A plain byte
        // search for "👩👩" hits first at byte 7, inside the 👨‍👩 cluster (its trailing 👩).
        // Skipping ahead by the whole pattern length from that hit jumps over the real match at
        // cluster 1 (byte 11) and lands on cluster 2 instead -- the match must be found by
        // retreating to just past the mid-cluster hit's single char, not its whole pattern width.
        let word = Value::test_string("👨\u{200D}👩👩👩👩");
        let options = Arguments {
            substring: String::from("👩👩"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: true,
        };
        assert_eq!(
            action(&word, &options, Span::test_data()),
            Value::test_int(1)
        );

        let all = Arguments {
            all: true,
            ..options
        };
        assert_eq!(
            action(&word, &all, Span::test_data()),
            Value::test_list(vec![Value::test_int(1)])
        );
    }

    #[test]
    fn grapheme_clusters_finds_the_only_real_match_with_one_fewer_trailing_cluster() {
        let word = Value::test_string("👨\u{200D}👩👩👩");
        let options = Arguments {
            substring: String::from("👩👩"),
            range: None,
            cell_paths: None,
            end: false,
            all: false,
            graphemes: true,
        };
        assert_eq!(
            action(&word, &options, Span::test_data()),
            Value::test_int(1)
        );
    }

    #[test]
    fn grapheme_clusters_all_finds_a_match_that_overlaps_a_mid_cluster_hit_in_regional_indicators()
    {
        // "🇯🇵🇵🇵" pairs its 4 regional indicators into 2 clusters: 🇯🇵 (bytes 0-7), 🇵🇵
        // (bytes 8-15). A byte search for "🇵🇵" hits first at byte 4 (the second half of
        // cluster 0 plus the first half of cluster 1) before the real, boundary-aligned match at
        // byte 8.
        let word = Value::test_string("🇯🇵🇵🇵");
        let options = Arguments {
            substring: String::from("🇵🇵"),
            range: None,
            cell_paths: None,
            end: false,
            all: true,
            graphemes: true,
        };
        assert_eq!(
            action(&word, &options, Span::test_data()),
            Value::test_list(vec![Value::test_int(1)])
        );
    }

    #[test]
    fn grapheme_clusters_from_end_finds_a_match_that_overlaps_a_mid_cluster_hit() {
        // "🇵🇵🇵" is 2 clusters: 🇵🇵 (bytes 0-7), 🇵 (bytes 8-11, an unpaired trailing
        // indicator). Searching backward for "🇵🇵" hits first at byte 4 (straddling both
        // clusters) before the real match at byte 0.
        let word = Value::test_string("🇵🇵🇵");
        let options = Arguments {
            substring: String::from("🇵🇵"),
            range: None,
            cell_paths: None,
            end: true,
            all: false,
            graphemes: true,
        };
        assert_eq!(
            action(&word, &options, Span::test_data()),
            Value::test_int(0)
        );
    }

    /// Brute-force reference: every non-overlapping match of `substring` in `s` that starts on a
    /// grapheme boundary, found by trying every boundary left-to-right and taking the first
    /// substring-length match at or after each search position -- the same contract
    /// `grapheme_match_offsets` implements, just without its early-exit or backward-scan
    /// optimizations.
    fn brute_force_grapheme_hits(s: &str, substring: &str) -> Vec<i64> {
        let boundaries: Vec<usize> = s.grapheme_indices(true).map(|(i, _)| i).collect();
        let mut hits = Vec::new();
        let mut i = 0;
        while i < boundaries.len() {
            let start = boundaries[i];
            if s[start..].starts_with(substring) {
                hits.push(i as i64);
                // Advance to the first boundary at or after the end of this match.
                let end = start + substring.len();
                i = boundaries.partition_point(|&b| b < end).max(i + 1);
            } else {
                i += 1;
            }
        }
        hits
    }

    #[test]
    fn grapheme_clusters_all_matches_a_brute_force_reference_on_random_regional_indicator_strings()
    {
        use rand::RngExt;

        let alphabet = ['🇯', '🇵', 'a', '👩'];
        let mut rng = rand::rng();
        for _ in 0..2000 {
            let len = rng.random_range(1..12);
            let s: String = (0..len)
                .map(|_| alphabet[rng.random_range(0..alphabet.len())])
                .collect();
            for substring in ["🇵🇵", "👩👩", "🇵", "a"] {
                let word = Value::test_string(s.clone());
                let options = Arguments {
                    substring: String::from(substring),
                    range: None,
                    cell_paths: None,
                    end: false,
                    all: true,
                    graphemes: true,
                };
                let actual = action(&word, &options, Span::test_data());
                let expected = brute_force_grapheme_hits(&s, substring);
                assert_eq!(
                    actual,
                    Value::test_list(expected.into_iter().map(Value::test_int).collect()),
                    "s: {s:?}, substring: {substring:?}"
                );
            }
        }
    }
}
