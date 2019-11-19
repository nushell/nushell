use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct FromSSV;

#[derive(Deserialize)]
pub struct FromSSVArgs {
    headerless: bool,
    #[serde(rename(deserialize = "aligned-columns"))]
    aligned_columns: bool,
    #[serde(rename(deserialize = "minimum-spaces"))]
    minimum_spaces: Option<Tagged<usize>>,
}

const STRING_REPRESENTATION: &str = "from-ssv";
const DEFAULT_MINIMUM_SPACES: usize = 2;

impl WholeStreamCommand for FromSSV {
    fn name(&self) -> &str {
        STRING_REPRESENTATION
    }

    fn signature(&self) -> Signature {
        Signature::build(STRING_REPRESENTATION)
            .switch("headerless", "don't treat the first row as column names")
            .switch("aligned-columns", "assume columns are aligned")
            .named(
                "minimum-spaces",
                SyntaxShape::Int,
                "the mininum spaces to separate columns",
            )
    }

    fn usage(&self) -> &str {
        "Parse text as space-separated values and create a table. The default minimum number of spaces counted as a separator is 2."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_ssv)?.run()
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
            .flat_map(|s| find_indices(*s))
            .collect::<Vec<usize>>();

        indices.sort();
        indices.dedup();

        let headers: Vec<(String, usize)> = indices
            .iter()
            .enumerate()
            .map(|(i, position)| (format!("Column{}", i + 1), *position))
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
            .map(|s| s.to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        collect(headers, lines, separator)
    };

    let parse_without_headers = |ls: Vec<&str>| {
        let num_columns = ls.iter().map(|r| r.len()).max().unwrap_or(0);

        let headers = (1..=num_columns)
            .map(|i| format!("Column{}", i))
            .collect::<Vec<String>>();
        collect(headers, ls.iter().map(|s| s.as_ref()), separator)
    };

    match headers {
        HeaderOptions::WithHeaders(headers_raw) => parse_with_headers(lines, headers_raw),
        HeaderOptions::WithoutHeaders => parse_without_headers(lines.collect()),
    }
}

fn string_to_table(
    s: &str,
    headerless: bool,
    aligned_columns: bool,
    split_at: usize,
) -> Option<Vec<Vec<(String, String)>>> {
    let mut lines = s.lines().filter(|l| !l.trim().is_empty());
    let separator = " ".repeat(std::cmp::max(split_at, 1));

    let (ls, header_options) = if headerless {
        (lines, HeaderOptions::WithoutHeaders)
    } else {
        let headers = lines.next()?;
        (lines, HeaderOptions::WithHeaders(headers))
    };

    let f = if aligned_columns {
        parse_aligned_columns
    } else {
        parse_separated_columns
    };

    let parsed = f(ls, header_options, &separator);
    match parsed.len() {
        0 => None,
        _ => Some(parsed),
    }
}

fn from_ssv_string_to_value(
    s: &str,
    headerless: bool,
    aligned_columns: bool,
    split_at: usize,
    tag: impl Into<Tag>,
) -> Option<Tagged<Value>> {
    let tag = tag.into();
    let rows = string_to_table(s, headerless, aligned_columns, split_at)?
        .iter()
        .map(|row| {
            let mut tagged_dict = TaggedDictBuilder::new(&tag);
            for (col, entry) in row {
                tagged_dict.insert_tagged(
                    col,
                    Value::Primitive(Primitive::String(String::from(entry))).tagged(&tag),
                )
            }
            tagged_dict.into_tagged_value()
        })
        .collect();

    Some(Value::Table(rows).tagged(&tag))
}

fn from_ssv(
    FromSSVArgs {
        headerless,
        aligned_columns,
        minimum_spaces,
    }: FromSSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;
        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;
        let split_at = match minimum_spaces {
            Some(number) => number.item,
            None => DEFAULT_MINIMUM_SPACES
        };

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag.clone());
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                }
                _ => yield Err(ShellError::labeled_error_with_secondary (
                    "Expected a string from pipeline",
                    "requires string input",
                    &name,
                    "value originates from here",
                    &value_tag
                )),
            }
        }

        match from_ssv_string_to_value(&concat_string, headerless, aligned_columns, split_at, name.clone()) {
            Some(x) => match x {
                Tagged { item: Value::Table(list), ..} => {
                    for l in list { yield ReturnSuccess::value(l) }
                }
                x => yield ReturnSuccess::value(x)
            },
            None => if let Some(tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as SSV",
                    "input cannot be parsed ssv",
                    &name,
                    "value originates from here",
                    &tag,
                ))
            },
        }
    };

    Ok(stream.to_output_stream())
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
            Some(vec![
                vec![owned("a", "1"), owned("b", "2")],
                vec![owned("a", "3"), owned("b", "4")]
            ])
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
        assert_eq!(
            result,
            Some(vec![vec![owned("a", "1")], vec![owned("a", "2")]])
        );
    }

    #[test]
    fn it_uses_first_row_as_data_when_headerless() {
        let input = r#"
            a b
            1 2
            3 4
        "#;
        let result = string_to_table(input, true, true, 1);
        assert_eq!(
            result,
            Some(vec![
                vec![owned("Column1", "a"), owned("Column2", "b")],
                vec![owned("Column1", "1"), owned("Column2", "2")],
                vec![owned("Column1", "3"), owned("Column2", "4")]
            ])
        );
    }

    #[test]
    fn it_returns_none_given_an_empty_string() {
        let input = "";
        let result = string_to_table(input, true, true, 1);
        assert!(result.is_none());
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
            Some(vec![
                vec![
                    owned("column a", "entry 1"),
                    owned("column b", "entry number  2")
                ],
                vec![owned("column a", "3"), owned("column b", "four")]
            ])
        );
    }

    #[test]
    fn it_trims_remaining_separator_space() {
        let input = r#"
            colA   colB     colC
            val1   val2     val3
        "#;

        let trimmed = |s: &str| s.trim() == s;

        let result = string_to_table(input, false, true, 2).unwrap();
        assert!(result
            .iter()
            .all(|row| row.iter().all(|(a, b)| trimmed(a) && trimmed(b))))
    }

    #[test]
    fn it_keeps_empty_columns() {
        let input = r#"
            colA   col B     col C
                   val2      val3
            val4   val 5     val 6
            val7             val8
        "#;

        let result = string_to_table(input, false, true, 2).unwrap();
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
        )
    }

    #[test]
    fn it_uses_the_full_final_column() {
        let input = r#"
            colA   col B
            val1   val2   trailing value that should be included
        "#;

        let result = string_to_table(input, false, true, 2).unwrap();
        assert_eq!(
            result,
            vec![vec![
                owned("colA", "val1"),
                owned("col B", "val2   trailing value that should be included"),
            ],]
        )
    }

    #[test]
    fn it_handles_empty_values_when_headerless_and_aligned_columns() {
        let input = r#"
            a multi-word value  b           d
            1                        3-3    4
                                                       last
        "#;

        let result = string_to_table(input, true, true, 2).unwrap();
        assert_eq!(
            result,
            vec![
                vec![
                    owned("Column1", "a multi-word value"),
                    owned("Column2", "b"),
                    owned("Column3", ""),
                    owned("Column4", "d"),
                    owned("Column5", "")
                ],
                vec![
                    owned("Column1", "1"),
                    owned("Column2", ""),
                    owned("Column3", "3-3"),
                    owned("Column4", "4"),
                    owned("Column5", "")
                ],
                vec![
                    owned("Column1", ""),
                    owned("Column2", ""),
                    owned("Column3", ""),
                    owned("Column4", ""),
                    owned("Column5", "last")
                ],
            ]
        )
    }

    #[test]
    fn input_is_parsed_correctly_if_either_option_works() {
        let input = r#"
                docker-registry   docker-registry=default                   docker-registry=default   172.30.78.158   5000/TCP
                kubernetes        component=apiserver,provider=kubernetes   <none>                    172.30.0.2      443/TCP
                kubernetes-ro     component=apiserver,provider=kubernetes   <none>                    172.30.0.1      80/TCP
            "#;

        let aligned_columns_headerless = string_to_table(input, true, true, 2).unwrap();
        let separator_headerless = string_to_table(input, true, false, 2).unwrap();
        let aligned_columns_with_headers = string_to_table(input, false, true, 2).unwrap();
        let separator_with_headers = string_to_table(input, false, false, 2).unwrap();
        assert_eq!(aligned_columns_headerless, separator_headerless);
        assert_eq!(aligned_columns_with_headers, separator_with_headers);
    }
}
