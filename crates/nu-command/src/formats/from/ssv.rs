use indexmap::map::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct FromSsv;

const DEFAULT_MINIMUM_SPACES: usize = 2;

impl Command for FromSsv {
    fn name(&self) -> &str {
        "from ssv"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ssv")
            .input_output_types(vec![(Type::String, Type::Table(vec![]))])
            .switch(
                "noheaders",
                "don't treat the first row as column names",
                Some('n'),
            )
            .switch("aligned-columns", "assume columns are aligned", Some('a'))
            .named(
                "minimum-spaces",
                SyntaxShape::Int,
                "the minimum spaces to separate columns",
                Some('m'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as space-separated values and create a table. The default minimum number of spaces counted as a separator is 2."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: r#"'FOO   BAR
1   2' | from ssv"#,
            description: "Converts ssv formatted string to table",
            result: Some(Value::List { vals: vec![Value::Record { cols: vec!["FOO".to_string(), "BAR".to_string()], vals: vec![Value::String { val: "1".to_string(), span: Span::test_data() }, Value::String { val: "2".to_string(), span: Span::test_data() }], span: Span::test_data() }], span: Span::test_data() }),
        }, Example {
            example: r#"'FOO   BAR
1   2' | from ssv -n"#,
            description: "Converts ssv formatted string to table but not treating the first row as column names",
            result: Some(
                Value::List { vals: vec![Value::Record { cols: vec!["column1".to_string(), "column2".to_string()], vals: vec![Value::String { val: "FOO".to_string(), span: Span::test_data() }, Value::String { val: "BAR".to_string(), span: Span::test_data() }], span: Span::test_data() }, Value::Record { cols: vec!["column1".to_string(), "column2".to_string()], vals: vec![Value::String { val: "1".to_string(), span: Span::test_data() }, Value::String { val: "2".to_string(), span: Span::test_data() }], span: Span::test_data() }], span: Span::test_data() }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        from_ssv(engine_state, stack, call, input)
    }
}

enum HeaderOptions<'a> {
    WithHeaders(&'a str),
    WithoutHeaders,
}

fn parse_aligned_columns<'a>(
    lines: impl Iterator<Item = &'a str>,
    headers: HeaderOptions,
    separator: &str,
) -> Vec<Vec<(String, String)>> {
    fn construct<'a>(
        lines: impl Iterator<Item = &'a str>,
        headers: Vec<(String, usize)>,
    ) -> Vec<Vec<(String, String)>> {
        lines
            .map(|l| {
                headers
                    .iter()
                    .enumerate()
                    .map(|(i, (header_name, start_position))| {
                        let val = match headers.get(i + 1) {
                            Some((_, end)) => {
                                if *end < l.len() {
                                    l.get(*start_position..*end)
                                } else {
                                    l.get(*start_position..)
                                }
                            }
                            None => l.get(*start_position..),
                        }
                        .unwrap_or("")
                        .trim()
                        .into();
                        (header_name.clone(), val)
                    })
                    .collect()
            })
            .collect()
    }

    let find_indices = |line: &str| {
        let values = line
            .split(&separator)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        values
            .fold(
                (0, vec![]),
                |(current_pos, mut indices), value| match line[current_pos..].find(value) {
                    None => (current_pos, indices),
                    Some(index) => {
                        let absolute_index = current_pos + index;
                        indices.push(absolute_index);
                        (absolute_index + value.len(), indices)
                    }
                },
            )
            .1
    };

    let parse_with_headers = |lines, headers_raw: &str| {
        let indices = find_indices(headers_raw);
        let headers = headers_raw
            .split(&separator)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .zip(indices);

        let columns = headers.collect::<Vec<(String, usize)>>();

        construct(lines, columns)
    };

    let parse_without_headers = |ls: Vec<&str>| {
        let mut indices = ls
            .iter()
            .flat_map(|s| find_indices(s))
            .collect::<Vec<usize>>();

        indices.sort_unstable();
        indices.dedup();

        let headers: Vec<(String, usize)> = indices
            .iter()
            .enumerate()
            .map(|(i, position)| (format!("column{}", i + 1), *position))
            .collect();

        construct(ls.iter().map(|s| s.to_owned()), headers)
    };

    match headers {
        HeaderOptions::WithHeaders(headers_raw) => parse_with_headers(lines, headers_raw),
        HeaderOptions::WithoutHeaders => parse_without_headers(lines.collect()),
    }
}

fn parse_separated_columns<'a>(
    lines: impl Iterator<Item = &'a str>,
    headers: HeaderOptions,
    separator: &str,
) -> Vec<Vec<(String, String)>> {
    fn collect<'a>(
        headers: Vec<String>,
        rows: impl Iterator<Item = &'a str>,
        separator: &str,
    ) -> Vec<Vec<(String, String)>> {
        rows.map(|r| {
            headers
                .iter()
                .zip(r.split(separator).map(str::trim).filter(|s| !s.is_empty()))
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect()
        })
        .collect()
    }

    let parse_with_headers = |lines, headers_raw: &str| {
        let headers = headers_raw
            .split(&separator)
            .map(str::trim)
            .map(str::to_owned)
            .filter(|s| !s.is_empty())
            .collect();
        collect(headers, lines, separator)
    };

    let parse_without_headers = |ls: Vec<&str>| {
        let num_columns = ls.iter().map(|r| r.len()).max().unwrap_or(0);

        let headers = (1..=num_columns)
            .map(|i| format!("column{}", i))
            .collect::<Vec<String>>();
        collect(headers, ls.into_iter(), separator)
    };

    match headers {
        HeaderOptions::WithHeaders(headers_raw) => parse_with_headers(lines, headers_raw),
        HeaderOptions::WithoutHeaders => parse_without_headers(lines.collect()),
    }
}

fn string_to_table(
    s: &str,
    noheaders: bool,
    aligned_columns: bool,
    split_at: usize,
) -> Vec<Vec<(String, String)>> {
    let mut lines = s.lines().filter(|l| !l.trim().is_empty());
    let separator = " ".repeat(std::cmp::max(split_at, 1));

    let (ls, header_options) = if noheaders {
        (lines, HeaderOptions::WithoutHeaders)
    } else {
        match lines.next() {
            Some(header) => (lines, HeaderOptions::WithHeaders(header)),
            None => return vec![],
        }
    };

    let f = if aligned_columns {
        parse_aligned_columns
    } else {
        parse_separated_columns
    };

    f(ls, header_options, &separator)
}

fn from_ssv_string_to_value(
    s: &str,
    noheaders: bool,
    aligned_columns: bool,
    split_at: usize,
    span: Span,
) -> Value {
    let rows = string_to_table(s, noheaders, aligned_columns, split_at)
        .iter()
        .map(|row| {
            let mut dict = IndexMap::new();
            for (col, entry) in row {
                dict.insert(
                    col.to_string(),
                    Value::String {
                        val: entry.to_string(),
                        span,
                    },
                );
            }
            Value::from(Spanned { item: dict, span })
        })
        .collect();

    Value::List { vals: rows, span }
}

fn from_ssv(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let config = engine_state.get_config();
    let name = call.head;

    let noheaders = call.has_flag("noheaders");
    let aligned_columns = call.has_flag("aligned-columns");
    let minimum_spaces: Option<Spanned<usize>> =
        call.get_flag(engine_state, stack, "minimum-spaces")?;

    let concat_string = input.collect_string("", config)?;
    let split_at = match minimum_spaces {
        Some(number) => number.item,
        None => DEFAULT_MINIMUM_SPACES,
    };

    Ok(
        from_ssv_string_to_value(&concat_string, noheaders, aligned_columns, split_at, name)
            .into_pipeline_data(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(x: &str, y: &str) -> (String, String) {
        (String::from(x), String::from(y))
    }

    #[test]
    fn it_trims_empty_and_whitespace_only_lines() {
        let input = r#"

            a       b

            1       2

            3       4
        "#;
        let result = string_to_table(input, false, true, 1);
        assert_eq!(
            result,
            vec![
                vec![owned("a", "1"), owned("b", "2")],
                vec![owned("a", "3"), owned("b", "4")]
            ]
        );
    }

    #[test]
    fn it_deals_with_single_column_input() {
        let input = r#"
            a
            1
            2
        "#;
        let result = string_to_table(input, false, true, 1);
        assert_eq!(result, vec![vec![owned("a", "1")], vec![owned("a", "2")]]);
    }

    #[test]
    fn it_uses_first_row_as_data_when_noheaders() {
        let input = r#"
            a b
            1 2
            3 4
        "#;
        let result = string_to_table(input, true, true, 1);
        assert_eq!(
            result,
            vec![
                vec![owned("column1", "a"), owned("column2", "b")],
                vec![owned("column1", "1"), owned("column2", "2")],
                vec![owned("column1", "3"), owned("column2", "4")]
            ]
        );
    }

    #[test]
    fn it_allows_a_predefined_number_of_spaces() {
        let input = r#"
            column a   column b
            entry 1    entry number  2
            3          four
        "#;

        let result = string_to_table(input, false, true, 3);
        assert_eq!(
            result,
            vec![
                vec![
                    owned("column a", "entry 1"),
                    owned("column b", "entry number  2")
                ],
                vec![owned("column a", "3"), owned("column b", "four")]
            ]
        );
    }

    #[test]
    fn it_trims_remaining_separator_space() {
        let input = r#"
            colA   colB     colC
            val1   val2     val3
        "#;

        let trimmed = |s: &str| s.trim() == s;

        let result = string_to_table(input, false, true, 2);
        assert!(result
            .iter()
            .all(|row| row.iter().all(|(a, b)| trimmed(a) && trimmed(b))));
    }

    #[test]
    fn it_keeps_empty_columns() {
        let input = r#"
            colA   col B     col C
                   val2      val3
            val4   val 5     val 6
            val7             val8
        "#;

        let result = string_to_table(input, false, true, 2);
        assert_eq!(
            result,
            vec![
                vec![
                    owned("colA", ""),
                    owned("col B", "val2"),
                    owned("col C", "val3")
                ],
                vec![
                    owned("colA", "val4"),
                    owned("col B", "val 5"),
                    owned("col C", "val 6")
                ],
                vec![
                    owned("colA", "val7"),
                    owned("col B", ""),
                    owned("col C", "val8")
                ],
            ]
        );
    }

    #[test]
    fn it_can_produce_an_empty_stream_for_header_only_input() {
        let input = "colA   col B";

        let result = string_to_table(input, false, true, 2);
        let expected: Vec<Vec<(String, String)>> = vec![];
        assert_eq!(expected, result);
    }

    #[test]
    fn it_uses_the_full_final_column() {
        let input = r#"
            colA   col B
            val1   val2   trailing value that should be included
        "#;

        let result = string_to_table(input, false, true, 2);
        assert_eq!(
            result,
            vec![vec![
                owned("colA", "val1"),
                owned("col B", "val2   trailing value that should be included"),
            ]]
        );
    }

    #[test]
    fn it_handles_empty_values_when_noheaders_and_aligned_columns() {
        let input = r#"
            a multi-word value  b           d
            1                        3-3    4
                                                       last
        "#;

        let result = string_to_table(input, true, true, 2);
        assert_eq!(
            result,
            vec![
                vec![
                    owned("column1", "a multi-word value"),
                    owned("column2", "b"),
                    owned("column3", ""),
                    owned("column4", "d"),
                    owned("column5", "")
                ],
                vec![
                    owned("column1", "1"),
                    owned("column2", ""),
                    owned("column3", "3-3"),
                    owned("column4", "4"),
                    owned("column5", "")
                ],
                vec![
                    owned("column1", ""),
                    owned("column2", ""),
                    owned("column3", ""),
                    owned("column4", ""),
                    owned("column5", "last")
                ],
            ]
        );
    }

    #[test]
    fn input_is_parsed_correctly_if_either_option_works() {
        let input = r#"
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#;

        let aligned_columns_noheaders = string_to_table(input, true, true, 2);
        let separator_noheaders = string_to_table(input, true, false, 2);
        let aligned_columns_with_headers = string_to_table(input, false, true, 2);
        let separator_with_headers = string_to_table(input, false, false, 2);
        assert_eq!(aligned_columns_noheaders, separator_noheaders);
        assert_eq!(aligned_columns_with_headers, separator_with_headers);
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromSsv {})
    }
}
