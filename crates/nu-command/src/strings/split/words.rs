use fancy_regex::Regex;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split words"
    }

    fn signature(&self) -> Signature {
        Signature::build("split words")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .vectorizes_over_list(true)
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
    }

    fn usage(&self) -> &str {
        "Split a string's words into separate rows"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split the string's words into separate rows",
                example: "'hello world' | split words",
                result: Some(Value::List {
                    vals: vec![Value::test_string("hello"), Value::test_string("world")],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Split the string's words, of at least 3 characters, into separate rows",
                example: "'hello to the world' | split words -l 3",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("hello"),
                        Value::test_string("the"),
                        Value::test_string("world"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "A real-world example of splitting words",
                example: "fetch https://www.gutenberg.org/files/11/11-0.txt | str downcase | split words -l 2 | uniq -c | sort-by count --reverse | first 10",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        split_words(engine_state, stack, call, input)
    }
}

fn split_words(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let span = call.head;
    // let ignore_hyphenated = call.has_flag("ignore-hyphenated");
    // let ignore_apostrophes = call.has_flag("ignore-apostrophes");
    // let ignore_punctuation = call.has_flag("ignore-punctuation");
    let word_length: Option<usize> = call.get_flag(engine_state, stack, "min-word-length")?;

    input.flat_map(
        move |x| split_words_helper(&x, word_length, span),
        engine_state.ctrlc.clone(),
    )
}

fn split_words_helper(v: &Value, word_length: Option<usize>, span: Span) -> Vec<Value> {
    // There are some options here with this regex.
    // [^A-Za-z\'] = do not match uppercase or lowercase letters or apostrophes
    // [^[:alpha:]\'] = do not match any uppercase or lowercase letters or apostrophes
    // [^\p{L}\'] = do not match any unicode uppercase or lowercase letters or apostrophes
    // Let's go with the unicode one in hopes that it works on more than just ascii characters
    let regex_replace = Regex::new(r"[^\p{L}\']").expect("regular expression error");

    match v.span() {
        Ok(v_span) => {
            if let Ok(s) = v.as_string() {
                // let splits = s.unicode_words();
                // let words = trim_to_words(s);
                // let words: Vec<&str> = s.split_whitespace().collect();

                let replaced_string = regex_replace.replace_all(&s, " ").to_string();
                replaced_string
                    .split(' ')
                    .filter_map(|s| {
                        if s.trim() != "" {
                            if let Some(len) = word_length {
                                if s.chars().count() >= len {
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
                    .collect()
            } else {
                vec![Value::Error {
                    error: ShellError::PipelineMismatch("string".into(), span, v_span),
                }]
            }
        }
        Err(error) => vec![Value::Error { error }],
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

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
