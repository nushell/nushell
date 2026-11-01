use nu_engine::command_prelude::*;
use nu_protocol::{Signals, shell_error::generic::GenericError};

// Stand-in for `bracoxide::{ExpansionError, expand_with}` until a bracoxide release
// has them: see the BEGIN comment above `mod brace_expansion` for the swap.
use brace_expansion::{ExpansionError, expand_with};

#[derive(Clone)]
pub struct StrExpand;

impl Command for StrExpand {
    fn name(&self) -> &str {
        "str expand"
    }

    fn description(&self) -> &str {
        "Generates all possible combinations defined in brace expansion syntax."
    }

    fn extra_description(&self) -> &str {
        "This syntax may seem familiar with `glob {A,B}.C`. The difference is glob relies on filesystem, but str expand is not. Inside braces, we put variants. Then basically we're creating all possible outcomes."
    }

    fn signature(&self) -> Signature {
        Signature::build("str expand")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::List(Box::new(Type::String)))),
                ),
            ])
            .switch(
                "path",
                "Replaces all backslashes with double backslashes, useful for Path.",
                None,
            )
            .allow_variants_without_examples(true)
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![
            Example {
                description: "Define a range inside braces to produce a list of string.",
                example: "\"{3..5}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("3"),
                        Value::test_string("4"),
                        Value::test_string("5"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Ignore the next character after the backslash ('\\').",
                example: "'A{B\\,,C}' | str expand",
                result: Some(Value::list(
                    vec![Value::test_string("AB,"), Value::test_string("AC")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Commas that are not inside any braces need to be skipped.",
                example: "'Welcome\\, {home,mon ami}!' | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("Welcome, home!"),
                        Value::test_string("Welcome, mon ami!"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Use double backslashes to add a backslash.",
                example: "'A{B\\\\,C}' | str expand",
                result: Some(Value::list(
                    vec![Value::test_string("AB\\"), Value::test_string("AC")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Export comma separated values inside braces (`{}`) to a string list.",
                example: "\"{apple,banana,cherry}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("apple"),
                        Value::test_string("banana"),
                        Value::test_string("cherry"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "If the piped data is path, you may want to use --path flag, or else manually replace the backslashes with double backslashes.",
                example: "'C:\\{Users,Windows}' | str expand --path",
                result: Some(Value::list(
                    vec![
                        Value::test_string("C:\\Users"),
                        Value::test_string("C:\\Windows"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Brace expressions can be used one after another.",
                example: "\"A{b,c}D{e,f}G\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AbDeG"),
                        Value::test_string("AbDfG"),
                        Value::test_string("AcDeG"),
                        Value::test_string("AcDfG"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Collection may include an empty item. It can be put at the start of the list.",
                example: "\"A{,B,C}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("A"),
                        Value::test_string("AB"),
                        Value::test_string("AC"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Empty item can be at the end of the collection.",
                example: "\"A{B,C,}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AB"),
                        Value::test_string("AC"),
                        Value::test_string("A"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Empty item can be in the middle of the collection.",
                example: "\"A{B,,C}\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AB"),
                        Value::test_string("A"),
                        Value::test_string("AC"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Also, it is possible to use one inside another. Here is a real-world example, that creates files.",
                example: "\"A{B{1,3},C{2,5}}D\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("AB1D"),
                        Value::test_string("AB3D"),
                        Value::test_string("AC2D"),
                        Value::test_string("AC5D"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Supports zero padding in numeric ranges.",
                example: "\"A{08..10}B{11..013}C\" | str expand",
                result: Some(Value::list(
                    vec![
                        Value::test_string("A08B011C"),
                        Value::test_string("A08B012C"),
                        Value::test_string("A08B013C"),
                        Value::test_string("A09B011C"),
                        Value::test_string("A09B012C"),
                        Value::test_string("A09B013C"),
                        Value::test_string("A10B011C"),
                        Value::test_string("A10B012C"),
                        Value::test_string("A10B013C"),
                    ],
                    Span::test_data(),
                )),
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
        let is_path = call.has_flag(engine_state, stack, "path")?;
        run(call, input, is_path, engine_state)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let is_path = call.has_flag_const(working_set, stack, "path")?;
        run(call, input, is_path, working_set.permanent())
    }
}

fn run(
    call: &Call,
    input: PipelineData,
    is_path: bool,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    if let PipelineData::Empty = input {
        return Err(ShellError::PipelineEmpty { dst_span: span });
    }
    let signals = engine_state.signals().clone();
    input.map(
        move |v| {
            let value_span = v.span();
            let type_ = v.get_type();
            match v.coerce_into_string() {
                Ok(s) => {
                    let contents = if is_path { s.replace('\\', "\\\\") } else { s };
                    str_expand(&contents, span, value_span, &signals)
                }
                Err(_) => Value::error(
                    ShellError::OnlySupportsThisInputType {
                        exp_input_type: "string".into(),
                        wrong_type: type_.to_string(),
                        dst_span: span,
                        src_span: value_span,
                    },
                    span,
                ),
            }
        },
        engine_state.signals(),
    )
}

/// Expands the brace pattern `contents` into a list of strings. The expansion
/// stops with `ShellError::Interrupted` once `signals` is interrupted, since a
/// short pattern such as `{1..4000000000}` can make billions of strings.
fn str_expand(contents: &str, span: Span, value_span: Span, signals: &Signals) -> Value {
    use bracoxide::{
        parser::{ParsingError, parse},
        tokenizer::{TokenizationError, tokenize},
    };
    match tokenize(contents) {
        Ok(tokens) => {
            match parse(&tokens) {
                Ok(node) => {
                    match expand_with(&node, &mut || signals.interrupted()) {
                        Ok(possibilities) => {
                            Value::list(possibilities.iter().map(|e| Value::string(e,span)).collect::<Vec<Value>>(), span)
                        },
                        Err(e) => match e {
                            ExpansionError::Interrupted => {
                                Value::error(ShellError::Interrupted { span }, span)
                            }
                            ExpansionError::NumConversionFailed(s) => Value::error(
                                ShellError::Generic(
                                    GenericError::new(
                                        "Number Conversion Failed",
                                        format!("Number conversion failed at {s}."),
                                        value_span,
                                    )
                                    .with_help("Expected number, found text. Range format is `{M..N}`, where M and N are numeric values representing the starting and ending limits."),
                                ),
                                span,
                            ),
                        },
                    }
                },
                Err(e) => Value::error(
                    match e {
                        ParsingError::NoTokens => ShellError::PipelineEmpty { dst_span: value_span },
                        ParsingError::OBraExpected(s) => ShellError::Generic(
                            GenericError::new(
                                "Opening Brace Expected",
                                format!("Opening brace is expected at {s}."),
                                value_span,
                            )
                            .with_help("In brace syntax, we use equal amount of opening (`{`) and closing (`}`). Please, take a look at the examples."),
                        ),
                        ParsingError::CBraExpected(s) => ShellError::Generic(
                            GenericError::new(
                                "Closing Brace Expected",
                                format!("Closing brace is expected at {s}."),
                                value_span,
                            )
                            .with_help("In brace syntax, we use equal amount of opening (`{`) and closing (`}`). Please, see the examples."),
                        ),
                        ParsingError::RangeStartLimitExpected(s) => ShellError::Generic(
                            GenericError::new(
                                "Range Start Expected",
                                format!("Range start limit is missing, expected at {s}."),
                                value_span,
                            )
                            .with_help("In brace syntax, Range is defined like `{X..Y}`, where X and Y are a number. X is the start, Y is the end. Please, inspect the examples for more information."),
                        ),
                        ParsingError::RangeEndLimitExpected(s) => ShellError::Generic(
                            GenericError::new(
                                "Range Start Expected",
                                format!("Range start limit is missing, expected at {s}."),
                                value_span,
                            )
                            .with_help("In brace syntax, Range is defined like `{X..Y}`, where X and Y are a number. X is the start, Y is the end. Please see the examples, for more information."),
                        ),
                        ParsingError::ExpectedText(s) => ShellError::Generic(
                            GenericError::new(
                                "Expected Text",
                                format!("Expected text at {s}."),
                                value_span,
                            )
                            .with_help("Texts are only allowed before opening brace (`{`), after closing brace (`}`), or inside `{}`. Please take a look at the examples."),
                        ),
                        ParsingError::InvalidCommaUsage(s) => ShellError::Generic(
                            GenericError::new(
                                "Invalid Comma Usage",
                                format!("Found comma at {s}. Commas are only valid inside collection (`{{X,Y}}`)."),
                                value_span,
                            )
                            .with_help("To escape comma use backslash `\\,`."),
                        ),
                        ParsingError::RangeCantHaveText(s) => ShellError::Generic(
                            GenericError::new(
                                "Range Can not Have Text",
                                format!(
                                    "Expecting, brace, number, or range operator, but found text at {s}."
                                ),
                                value_span,
                            )
                            .with_help("Please use the format {M..N} for ranges in brace expansion, where M and N are numeric values representing the starting and ending limits of the sequence, respectively."),
                        ),
                        ParsingError::ExtraRangeOperator(s) => ShellError::Generic(
                            GenericError::new(
                                "Extra Range Operator",
                                format!("Found additional, range operator at {s}."),
                                value_span,
                            )
                            .with_help("Please, use the format `{M..N}` where M and N are numeric values representing the starting and ending limits of the range."),
                        ),
                        ParsingError::ExtraCBra(s) => ShellError::Generic(
                            GenericError::new(
                                "Extra Closing Brace",
                                format!("Used extra closing brace at {s}."),
                                value_span,
                            )
                            .with_help("To escape closing brace use backslash, e.g. `\\}`"),
                        ),
                        ParsingError::ExtraOBra(s) => ShellError::Generic(
                            GenericError::new(
                                "Extra Opening Brace",
                                format!("Used extra opening brace at {s}."),
                                value_span,
                            )
                            .with_help("To escape opening brace use backslash, e.g. `\\{`"),
                        ),
                        ParsingError::NothingInBraces(s) => ShellError::Generic(
                            GenericError::new(
                                "Nothing In Braces",
                                format!("Nothing found inside braces at {s}."),
                                value_span,
                            )
                            .with_help("Please provide valid content within the braces. Additionally, you can safely remove it, not needed."),
                        ),
                    }
                ,
                span,
                )
            }
        },
        Err(e) => match e {
            TokenizationError::EmptyContent => Value::error(
                ShellError::PipelineEmpty { dst_span: value_span },
                value_span,
            ),
            TokenizationError::FormatNotSupported => Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "Format Not Supported",
                        "Usage of only `{` or `}`. Brace Expansion syntax, needs to have equal amount of opening (`{`) and closing (`}`)",
                        value_span,
                    )
                    .with_help("In brace expansion syntax, it is important to have an equal number of opening (`{`) and closing (`}`) braces. Please ensure that you provide a balanced pair of braces in your brace expansion pattern."),
                ),
                value_span,
            ),
            TokenizationError::NoBraces => Value::error(
                ShellError::Generic(
                    GenericError::new(
                        "No Braces",
                        "At least one `{}` brace expansion expected.",
                        value_span,
                    )
                    .with_help("Please, examine the examples."),
                ),
                value_span,
            )
        },
    }
}

// ==== BEGIN stand-in for `bracoxide::expand_with` ===============================
//
// bracoxide 0.1.8's `expand` never checks for an interrupt, so Ctrl+C can't stop
// `'{1..4000000000}' | str expand`. atahabaki/bracoxide#15 adds `expand_with` and
// `ExpansionError::Interrupted` upstream. This module copies that change on top of
// bracoxide 0.1.8's public `parser::Node`, so nushell has it before a release.
//
// To swap in the upstream version once nushell depends on a bracoxide release that
// has `expand_with`:
// 1. Delete everything from this BEGIN line to the END line below.
// 2. At the top of this file, change `use brace_expansion::{ExpansionError, expand_with}`
//    to `use bracoxide::{ExpansionError, expand_with}`. The tests pick it up through
//    `use super::*`, so `expand_with_stops_while_expanding` then tests bracoxide's.
// 3. Upstream marks `ExpansionError` `#[non_exhaustive]`, so add a `_` arm to the
//    match on it in `str_expand`, such as a `GenericError` with `e.to_string()`.
// 4. Delete the `stand_in_matches_bracoxide_expand` test, which compares this copy
//    with bracoxide's own `expand`.
mod brace_expansion {
    use bracoxide::parser::Node;

    /// Why an expansion failed: bracoxide's `ExpansionError` plus `Interrupted`.
    #[derive(Debug, PartialEq)]
    pub(super) enum ExpansionError {
        /// A range limit isn't a number that fits in a `usize`.
        NumConversionFailed(String),
        /// `should_stop` returned `true`.
        Interrupted,
    }

    /// Expands `node` like `bracoxide::expand`, but calls `should_stop` before
    /// making each string, the parts it joins included, and returns
    /// `ExpansionError::Interrupted` as soon as it returns `true`.
    pub(super) fn expand_with(
        node: &Node,
        should_stop: &mut dyn FnMut() -> bool,
    ) -> Result<Vec<String>, ExpansionError> {
        match node {
            Node::Text { message, .. } => {
                stop_if(should_stop)?;
                Ok(vec![message.as_ref().to_owned()])
            }
            Node::BraceExpansion {
                prefix,
                inside,
                postfix,
                ..
            } => {
                let mut expand_part = |part: &Option<Box<Node>>| match part {
                    Some(part) => expand_with(part, should_stop),
                    None => Ok(vec![String::new()]),
                };
                let prefixes = expand_part(prefix)?;
                let insides = expand_part(inside)?;
                let postfixes = expand_part(postfix)?;
                let mut inner = vec![];
                for prefix in &prefixes {
                    for inside in &insides {
                        for postfix in &postfixes {
                            stop_if(should_stop)?;
                            inner.push(format!("{prefix}{inside}{postfix}"));
                        }
                    }
                }
                Ok(inner)
            }
            Node::Collection { items, .. } => {
                let mut inner = vec![];
                for item in items {
                    inner.extend(expand_with(item, should_stop)?);
                }
                Ok(inner)
            }
            Node::Range { from, to, .. } => {
                // A limit with a leading zero pads every number to the longer limit.
                let zero_pad = if (from.starts_with('0') && from.len() > 1) || to.starts_with('0') {
                    from.len().max(to.len())
                } else {
                    0
                };
                let parse_limit = |limit: &str| {
                    limit
                        .parse::<usize>()
                        .map_err(|_| ExpansionError::NumConversionFailed(limit.to_owned()))
                };
                let (from, to) = (parse_limit(from)?, parse_limit(to)?);
                let mut inner = vec![];
                for i in from..=to {
                    stop_if(should_stop)?;
                    inner.push(format!("{i:0>zero_pad$}"));
                }
                Ok(inner)
            }
        }
    }

    /// `ExpansionError::Interrupted` when `should_stop` returns `true`.
    fn stop_if(should_stop: &mut dyn FnMut() -> bool) -> Result<(), ExpansionError> {
        if should_stop() {
            Err(ExpansionError::Interrupted)
        } else {
            Ok(())
        }
    }
}
// ==== END stand-in for `bracoxide::expand_with` =================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};

    #[test]
    fn test_zero_padding_actual_zero() {
        assert_eq!(
            str_expand(
                "{0..10}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("0"), Span::test_data(),),
                    Value::string(String::from("1"), Span::test_data(),),
                    Value::string(String::from("2"), Span::test_data(),),
                    Value::string(String::from("3"), Span::test_data(),),
                    Value::string(String::from("4"), Span::test_data(),),
                    Value::string(String::from("5"), Span::test_data(),),
                    Value::string(String::from("6"), Span::test_data(),),
                    Value::string(String::from("7"), Span::test_data(),),
                    Value::string(String::from("8"), Span::test_data(),),
                    Value::string(String::from("9"), Span::test_data(),),
                    Value::string(String::from("10"), Span::test_data(),),
                ],
                Span::test_data(),
            )
        );
        assert_eq!(
            str_expand(
                "{00..10}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("00"), Span::test_data(),),
                    Value::string(String::from("01"), Span::test_data(),),
                    Value::string(String::from("02"), Span::test_data(),),
                    Value::string(String::from("03"), Span::test_data(),),
                    Value::string(String::from("04"), Span::test_data(),),
                    Value::string(String::from("05"), Span::test_data(),),
                    Value::string(String::from("06"), Span::test_data(),),
                    Value::string(String::from("07"), Span::test_data(),),
                    Value::string(String::from("08"), Span::test_data(),),
                    Value::string(String::from("09"), Span::test_data(),),
                    Value::string(String::from("10"), Span::test_data(),),
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_double_dots_outside_curly() {
        assert_eq!(
            str_expand(
                "..{a,b}..",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("..a.."), Span::test_data(),),
                    Value::string(String::from("..b.."), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_outer_single_item() {
        assert_eq!(
            str_expand(
                "{W{x,y}}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("Wx"), Span::test_data(),),
                    Value::string(String::from("Wy"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn dots() {
        assert_eq!(
            str_expand(
                "{a.b.c,d}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("a.b.c"), Span::test_data(),),
                    Value::string(String::from("d"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
        assert_eq!(
            str_expand(
                "{1.2.3,a}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("1.2.3"), Span::test_data(),),
                    Value::string(String::from("a"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
        assert_eq!(
            str_expand(
                "{a-1.2,b}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("a-1.2"), Span::test_data(),),
                    Value::string(String::from("b"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    #[test]
    fn test_numbers_proceeding_escape_char_not_ignored() {
        assert_eq!(
            str_expand(
                "1\\\\{a,b}",
                Span::test_data(),
                Span::test_data(),
                &Signals::empty()
            ),
            Value::list(
                vec![
                    Value::string(String::from("1\\a"), Span::test_data(),),
                    Value::string(String::from("1\\b"), Span::test_data(),)
                ],
                Span::test_data(),
            )
        );
    }

    /// An interrupt stops the expansion, even of a pattern that would finish at once.
    #[test]
    fn interrupted_expansion_is_an_error() {
        let signals = Signals::new(Arc::new(AtomicBool::new(true)));
        let value = str_expand("{1..3}", Span::test_data(), Span::test_data(), &signals);
        assert!(
            matches!(&value, Value::Error { error, .. } if matches!(**error, ShellError::Interrupted { .. })),
            "expected an interrupted error, got {value:?}"
        );
    }

    /// `expand_with` checks `should_stop` while it counts through a range and
    /// while it joins the parts of a brace expansion, so neither runs to the end.
    #[test]
    fn expand_with_stops_while_expanding() {
        use bracoxide::{parser::parse, tokenizer::tokenize};

        let range = bracoxide::parser::Node::Range {
            from: Arc::new("1".into()),
            to: Arc::new("100000".into()),
            start: 0,
            end: 0,
        };
        // The two ranges and the inner join take 3,000 checks; the outer join
        // would then make a million strings.
        let joined = parse(&tokenize("{1..1000}{1..1000}").expect("pattern tokenizes"))
            .expect("pattern parses");
        for node in [range, joined] {
            let mut checks = 0;
            let expanded = expand_with(&node, &mut || {
                checks += 1;
                checks > 5000
            });
            assert_eq!(expanded, Err(ExpansionError::Interrupted), "{node:?}");
        }
    }

    /// The stand-in expands like bracoxide's own `expand`. Delete this test with
    /// the `brace_expansion` module.
    #[test]
    fn stand_in_matches_bracoxide_expand() {
        use bracoxide::{parser::parse, tokenizer::tokenize};

        for pattern in [
            "{3..5}",
            "A{B\\,,C}",
            "Welcome\\, {home,mon ami}!",
            "A{B\\\\,C}",
            "{apple,banana,cherry}",
            "A{b,c}D{e,f}G",
            "A{,B,C}",
            "A{B,C,}",
            "A{B,,C}",
            "A{B{1,3},C{2,5}}D",
            "A{08..10}B{11..013}C",
            "{0..10}",
            "{00..10}",
            "{0..0}",
            "{10..1}",
            "..{a,b}..",
            "{W{x,y}}",
            "{a.b.c,d}",
            "1\\\\{a,b}",
            "{1..99999999999999999999999}",
        ] {
            let node =
                parse(&tokenize(pattern).expect("pattern tokenizes")).expect("pattern parses");
            let upstream = bracoxide::expand(&node).map_err(|error| match error {
                bracoxide::ExpansionError::NumConversionFailed(s) => {
                    ExpansionError::NumConversionFailed(s)
                }
            });
            assert_eq!(expand_with(&node, &mut || false), upstream, "{pattern}");
        }
    }

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(StrExpand)
    }
}
