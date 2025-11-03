use crate::{grapheme_flags, grapheme_flags_const};
use fancy_regex::Regex;
use nu_engine::command_prelude::*;

use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct SplitWords;

impl Command for SplitWords {
    fn name(&self) -> &str {
        "split words"
    }

    fn signature(&self) -> Signature {
        Signature::build("split words")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::List(Box::new(Type::String))))
                ),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Strings)
            // .switch(
            //     "ignore-hyphenated",
            //     "ignore hyphenated words, splitting at the hyphen",
            //     Some('i'),
            // )
            // .switch(
            //     "ignore-apostrophes",
            //     "ignore apostrophes in words by removing them",
            //     Some('a'),
            // )
            // .switch(
            //     "ignore-punctuation",
            //     "ignore punctuation around words by removing them",
            //     Some('p'),
            // )
            .named(
                "min-word-length",
                SyntaxShape::Int,
                "The minimum word length",
                Some('l'),
            )
            .switch(
                "grapheme-clusters",
                "measure word length in grapheme clusters (requires -l)",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "measure word length in UTF-8 bytes (default; requires -l; non-ASCII chars are length 2+)",
                Some('b'),
            )
    }

    fn description(&self) -> &str {
        "Split a string's words into separate rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Split the string's words into separate rows",
                example: "'hello world' | split words",
                result: Some(Value::list(
                    vec![Value::test_string("hello"), Value::test_string("world")],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Split the string's words, of at least 3 characters, into separate rows",
                example: "'hello to the world' | split words --min-word-length 3",
                result: Some(Value::list(
                    vec![
                        Value::test_string("hello"),
                        Value::test_string("the"),
                        Value::test_string("world"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "A real-world example of splitting words",
                example: "http get https://www.gutenberg.org/files/11/11-0.txt | str downcase | split words --min-word-length 2 | uniq --count | sort-by count --reverse | first 10",
                result: None,
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
        let word_length: Option<usize> = call.get_flag(engine_state, stack, "min-word-length")?;
        let has_grapheme = call.has_flag(engine_state, stack, "grapheme-clusters")?;
        let has_utf8 = call.has_flag(engine_state, stack, "utf-8-bytes")?;
        let graphemes = grapheme_flags(engine_state, stack, call)?;

        let args = Arguments {
            word_length,
            has_grapheme,
            has_utf8,
            graphemes,
        };
        split_words(engine_state, call, input, args)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let word_length: Option<usize> = call.get_flag_const(working_set, "min-word-length")?;
        let has_grapheme = call.has_flag_const(working_set, "grapheme-clusters")?;
        let has_utf8 = call.has_flag_const(working_set, "utf-8-bytes")?;
        let graphemes = grapheme_flags_const(working_set, call)?;

        let args = Arguments {
            word_length,
            has_grapheme,
            has_utf8,
            graphemes,
        };
        split_words(working_set.permanent(), call, input, args)
    }
}

struct Arguments {
    word_length: Option<usize>,
    has_grapheme: bool,
    has_utf8: bool,
    graphemes: bool,
}

fn split_words(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    // let ignore_hyphenated = call.has_flag(engine_state, stack, "ignore-hyphenated")?;
    // let ignore_apostrophes = call.has_flag(engine_state, stack, "ignore-apostrophes")?;
    // let ignore_punctuation = call.has_flag(engine_state, stack, "ignore-punctuation")?;

    if args.word_length.is_none() {
        if args.has_grapheme {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "--grapheme-clusters (-g) requires --min-word-length (-l)".to_string(),
                span,
            });
        }
        if args.has_utf8 {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "--utf-8-bytes (-b) requires --min-word-length (-l)".to_string(),
                span,
            });
        }
    }

    input.map(
        move |x| split_words_helper(&x, args.word_length, span, args.graphemes),
        engine_state.signals(),
    )
}

fn split_words_helper(v: &Value, word_length: Option<usize>, span: Span, graphemes: bool) -> Value {
    // There are some options here with this regex.
    // [^A-Za-z\'] = do not match uppercase or lowercase letters or apostrophes
    // [^[:alpha:]\'] = do not match any uppercase or lowercase letters or apostrophes
    // [^\p{L}\'] = do not match any unicode uppercase or lowercase letters or apostrophes
    // Let's go with the unicode one in hopes that it works on more than just ascii characters
    let regex_replace = Regex::new(r"[^\p{L}\p{N}\']").expect("regular expression error");
    let v_span = v.span();

    match v {
        Value::Error { error, .. } => Value::error(*error.clone(), v_span),
        v => {
            let v_span = v.span();
            if let Ok(s) = v.as_str() {
                // let splits = s.unicode_words();
                // let words = trim_to_words(s);
                // let words: Vec<&str> = s.split_whitespace().collect();

                let replaced_string = regex_replace.replace_all(s, " ").to_string();
                let words = replaced_string
                    .split(' ')
                    .filter_map(|s| {
                        if s.trim() != "" {
                            if let Some(len) = word_length {
                                if if graphemes {
                                    s.graphemes(true).count()
                                } else {
                                    s.len()
                                } >= len
                                {
                                    Some(Value::string(s, v_span))
                                } else {
                                    None
                                }
                            } else {
                                Some(Value::string(s, v_span))
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Value>>();
                Value::list(words, v_span)
            } else {
                Value::error(
                    ShellError::OnlySupportsThisInputType {
                        exp_input_type: "string".into(),
                        wrong_type: v.get_type().to_string(),
                        dst_span: span,
                        src_span: v_span,
                    },
                    v_span,
                )
            }
        }
    }
}

// original at least 1 char long
// curl -sL "https://www.gutenberg.org/files/11/11-0.txt" | tr '[:upper:]' '[:lower:]' | grep -oE "[a-z\']{1,}" | ^sort | ^uniq -c | ^sort -nr | head -n 10
// benchmark INCLUDING DOWNLOAD: 1sec 253ms 91µs 511ns
//    1839 the
//     942 and
//     811 to
//     695 a
//     638 of
//     610 it
//     553 she
//     546 i
//     486 you
//     462 said

// original at least 2 chars long
// curl -sL "https://www.gutenberg.org/files/11/11-0.txt" | tr '[:upper:]' '[:lower:]' | grep -oE "[a-z\']{2,}" | ^sort | ^uniq -c | ^sort -nr | head -n 10
//    1839 the
//     942 and
//     811 to
//     638 of
//     610 it
//     553 she
//     486 you
//     462 said
//     435 in
//     403 alice

// regex means, replace everything that is not A-Z or a-z or ' with a space
// ❯ $contents | str replace "[^A-Za-z\']" " " -a | split row ' ' | where ($it | str length) > 1 | uniq -i -c | sort-by count --reverse | first 10
// benchmark: 1sec 775ms 471µs 600ns
// ╭───┬───────┬───────╮
// │ # │ value │ count │
// ├───┼───────┼───────┤
// │ 0 │ the   │  1839 │
// │ 1 │ and   │   942 │
// │ 2 │ to    │   811 │
// │ 3 │ of    │   638 │
// │ 4 │ it    │   610 │
// │ 5 │ she   │   553 │
// │ 6 │ you   │   486 │
// │ 7 │ said  │   462 │
// │ 8 │ in    │   435 │
// │ 9 │ alice │   403 │
// ╰───┴───────┴───────╯

// $alice |str replace "[^A-Za-z\']" " " -a | split row ' ' | uniq -i -c | sort-by count --reverse | first 10
// benchmark: 1sec 518ms 701µs 200ns
// ╭───┬───────┬───────╮
// │ # │ value │ count │
// ├───┼───────┼───────┤
// │ 0 │ the   │  1839 │
// │ 1 │ and   │   942 │
// │ 2 │ to    │   811 │
// │ 3 │ a     │   695 │
// │ 4 │ of    │   638 │
// │ 5 │ it    │   610 │
// │ 6 │ she   │   553 │
// │ 7 │ i     │   546 │
// │ 8 │ you   │   486 │
// │ 9 │ said  │   462 │
// ├───┼───────┼───────┤
// │ # │ value │ count │
// ╰───┴───────┴───────╯

// s.unicode_words()
// $alice | str downcase | split words | sort | uniq -c | sort-by count | reverse | first 10
// benchmark: 4sec 965ms 285µs 800ns
// ╭───┬───────┬───────╮
// │ # │ value │ count │
// ├───┼───────┼───────┤
// │ 0 │ the   │  1839 │
// │ 1 │ and   │   941 │
// │ 2 │ to    │   811 │
// │ 3 │ a     │   695 │
// │ 4 │ of    │   638 │
// │ 5 │ it    │   542 │
// │ 6 │ she   │   538 │
// │ 7 │ said  │   460 │
// │ 8 │ in    │   434 │
// │ 9 │ you   │   426 │
// ├───┼───────┼───────┤
// │ # │ value │ count │
// ╰───┴───────┴───────╯

// trim_to_words
// benchmark: 5sec 992ms 76µs 200ns
// ╭───┬───────┬───────╮
// │ # │ value │ count │
// ├───┼───────┼───────┤
// │ 0 │ the   │  1829 │
// │ 1 │ and   │   918 │
// │ 2 │ to    │   801 │
// │ 3 │ a     │   689 │
// │ 4 │ of    │   632 │
// │ 5 │ she   │   537 │
// │ 6 │ it    │   493 │
// │ 7 │ said  │   457 │
// │ 8 │ in    │   430 │
// │ 9 │ you   │   413 │
// ├───┼───────┼───────┤
// │ # │ value │ count │
// ╰───┴───────┴───────╯

// fn trim_to_words(content: String) -> std::vec::Vec<std::string::String> {
//     let content: Vec<String> = content
//         .to_lowercase()
//         .replace(&['-'][..], " ")
//         //should 's be replaced?
//         .replace("'s", "")
//         .replace(
//             &[
//                 '(', ')', ',', '\"', '.', ';', ':', '=', '[', ']', '{', '}', '-', '_', '/', '\'',
//                 '’', '?', '!', '“', '‘',
//             ][..],
//             "",
//         )
//         .split_whitespace()
//         .map(String::from)
//         .collect::<Vec<String>>();
//     content
// }

// split_whitespace()
// benchmark: 9sec 379ms 790µs 900ns
// ╭───┬───────┬───────╮
// │ # │ value │ count │
// ├───┼───────┼───────┤
// │ 0 │ the   │  1683 │
// │ 1 │ and   │   783 │
// │ 2 │ to    │   778 │
// │ 3 │ a     │   667 │
// │ 4 │ of    │   605 │
// │ 5 │ she   │   485 │
// │ 6 │ said  │   416 │
// │ 7 │ in    │   406 │
// │ 8 │ it    │   357 │
// │ 9 │ was   │   329 │
// ├───┼───────┼───────┤
// │ # │ value │ count │
// ╰───┴───────┴───────╯

// current
// $alice | str downcase | split words | uniq -c | sort-by count --reverse | first 10
// benchmark: 1sec 481ms 604µs 700ns
// ╭───┬───────┬───────╮
// │ # │ value │ count │
// ├───┼───────┼───────┤
// │ 0 │ the   │  1839 │
// │ 1 │ and   │   942 │
// │ 2 │ to    │   811 │
// │ 3 │ a     │   695 │
// │ 4 │ of    │   638 │
// │ 5 │ it    │   610 │
// │ 6 │ she   │   553 │
// │ 7 │ i     │   546 │
// │ 8 │ you   │   486 │
// │ 9 │ said  │   462 │
// ├───┼───────┼───────┤
// │ # │ value │ count │
// ╰───┴───────┴───────╯

#[cfg(test)]
mod test {
    use super::*;
    use nu_test_support::nu;

    #[test]
    fn test_incompat_flags() {
        let out = nu!("'a' | split words -bg -l 2");
        assert!(out.err.contains("incompatible_parameters"));
    }

    #[test]
    fn test_incompat_flags_2() {
        let out = nu!("'a' | split words -g");
        assert!(out.err.contains("incompatible_parameters"));
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SplitWords {})
    }
    #[test]
    fn mixed_letter_number() {
        let actual = nu!(r#"echo "a1 b2 c3" | split words | str join ','"#);
        assert_eq!(actual.out, "a1,b2,c3");
    }
}
