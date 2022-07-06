use crate::textstyle::TextStyle;
use crate::{StyledString, TableTheme};
use ansi_str::AnsiStr;
use nu_ansi_term::Style;
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::Iterator;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug)]
pub struct Subline {
    pub subline: String,
    pub width: usize,
}

#[derive(Debug)]
pub struct Line {
    pub sublines: Vec<Subline>,
    pub width: usize,
}

#[derive(Debug, Clone)]
pub struct WrappedLine {
    pub line: String,
    pub width: usize,
}

#[derive(Debug, Clone)]
pub struct WrappedCell {
    pub lines: Vec<WrappedLine>,
    pub max_width: usize,

    pub style: TextStyle,
}

/// Removes ANSI escape codes and some ASCII control characters
///
/// Keeps `\n` removes `\r`, `\t` etc.
///
/// If parsing fails silently returns the input string
fn strip_ansi(string: &str) -> Cow<str> {
    // Check if any ascii control character except LF(0x0A = 10) is present,
    // which will be stripped. Includes the primary start of ANSI sequences ESC
    // (0x1B = decimal 27)
    if string.bytes().any(|x| matches!(x, 0..=9 | 11..=31)) {
        if let Ok(stripped) = strip_ansi_escapes::strip(string) {
            if let Ok(new_string) = String::from_utf8(stripped) {
                return Cow::Owned(new_string);
            }
        }
    }
    // Else case includes failures to parse!
    Cow::Borrowed(string)
}

// fn special_width(astring: &str) -> usize {
//     // remove the zwj's '\u{200d}'
//     // remove the fe0f's
//     let stripped_string: String = {
//         if let Ok(bytes) = strip_ansi_escapes::strip(astring) {
//             String::from_utf8_lossy(&bytes).to_string()
//         } else {
//             astring.to_string()
//         }
//     };

//     let no_zwj = stripped_string.replace('\u{200d}', "");
//     let no_fe0f = no_zwj.replace('\u{fe0f}', "");
//     UnicodeWidthStr::width(&no_fe0f[..])
// }

pub fn split_sublines(input: &str) -> Vec<Vec<Subline>> {
    input
        .ansi_split("\n")
        .map(|line| {
            line.ansi_split(" ")
                .map(|x| Subline {
                    subline: x.to_string(),
                    width: {
                        // We've tried UnicodeWidthStr::width(x), UnicodeSegmentation::graphemes(x, true).count()
                        // and x.chars().count() with all types of combinations. Currently, it appears that
                        // getting the max of char count and Unicode width seems to produce the best layout.
                        // However, it's not perfect.
                        // let c = x.chars().count();
                        // let u = UnicodeWidthStr::width(x);
                        // std::cmp::min(c, u)

                        // let c = strip_ansi(x).chars().count();
                        // let u = special_width(x);
                        // std::cmp::max(c, u)
                        let stripped = strip_ansi(&x);

                        let c = stripped.chars().count();
                        let u = stripped.width();
                        std::cmp::max(c, u)
                    },
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}

pub fn column_width(input: &[Vec<Subline>]) -> usize {
    let mut max = 0;

    for line in input {
        let mut total = 0;

        let mut first = true;
        for inp in line {
            if !first {
                // Account for the space
                total += 1;
            } else {
                first = false;
            }

            total += inp.width;
        }

        if total > max {
            max = total;
        }
    }

    max
}

fn split_word(cell_width: usize, word: &str) -> Vec<Subline> {
    let mut output = vec![];
    let mut current_width = 0;
    let mut start_index = 0;
    let mut end_index;

    let word_no_ansi = strip_ansi(word);
    for c in word_no_ansi.char_indices() {
        if let Some(width) = c.1.width() {
            end_index = c.0;
            if current_width + width > cell_width {
                output.push(Subline {
                    subline: word.ansi_cut(start_index..end_index),
                    width: current_width,
                });

                start_index = c.0;
                current_width = width;
            } else {
                current_width += width;
            }
        }
    }

    if start_index != word_no_ansi.len() {
        output.push(Subline {
            subline: word.ansi_cut(start_index..),
            width: current_width,
        });
    }

    output
}

pub fn wrap_content(
    cell_width: usize,
    mut input: impl Iterator<Item = Subline>,
    color_hm: &HashMap<String, Style>,
    re_leading: &regex::Regex,
    re_trailing: &regex::Regex,
) -> (Vec<WrappedLine>, usize) {
    let mut lines = vec![];
    let mut current_line: Vec<Subline> = vec![];
    let mut current_width = 0;
    let mut first = true;
    let mut max_width = 0;
    let lead_trail_space_bg_color = color_hm
        .get("leading_trailing_space_bg")
        .unwrap_or(&Style::default())
        .to_owned();

    loop {
        match input.next() {
            Some(item) => {
                if !first {
                    current_width += 1;
                } else {
                    first = false;
                }

                if item.width + current_width > cell_width {
                    // If this is a really long single word, we need to split the word
                    if current_line.len() == 1 && current_width > cell_width {
                        max_width = cell_width;
                        let sublines = split_word(cell_width, &current_line[0].subline);
                        for subline in sublines {
                            let width = subline.width;
                            lines.push(Line {
                                sublines: vec![subline],
                                width,
                            });
                        }

                        first = true;

                        current_width = item.width;
                        current_line = vec![item];
                    } else {
                        if !current_line.is_empty() {
                            lines.push(Line {
                                sublines: current_line,
                                width: current_width,
                            });
                        }

                        first = true;

                        current_width = item.width;
                        current_line = vec![item];
                        max_width = std::cmp::max(max_width, current_width);
                    }
                } else {
                    current_width += item.width;
                    current_line.push(item);
                }
            }
            None => {
                if current_width > cell_width {
                    // We need to break up the last word
                    let sublines = split_word(cell_width, &current_line[0].subline);
                    for subline in sublines {
                        let width = subline.width;
                        lines.push(Line {
                            sublines: vec![subline],
                            width,
                        });
                    }
                } else if current_width > 0 {
                    lines.push(Line {
                        sublines: current_line,
                        width: current_width,
                    });
                }
                break;
            }
        }
    }

    let mut current_max = 0;
    let mut output = vec![];

    for line in lines {
        let mut current_line_width = 0;
        let mut first = true;
        let mut current_line = String::new();

        for subline in line.sublines {
            if !first {
                current_line_width += 1 + subline.width;
                current_line.push(' ');
            } else {
                first = false;
                current_line_width = subline.width;
            }
            current_line.push_str(&subline.subline);
        }

        if current_line_width > current_max {
            current_max = current_line_width;
        }

        // highlight leading and trailing spaces so they stand out.
        let mut bg_color_string = Style::default().prefix().to_string();
        // right now config settings can only set foreground colors so, in this
        // instance we take the foreground color and make it a background color
        if let Some(bg) = lead_trail_space_bg_color.foreground {
            bg_color_string = Style::default().on(bg).prefix().to_string()
        };

        if let Some(leading_match) = re_leading.find(&current_line.clone()) {
            String::insert_str(
                &mut current_line,
                leading_match.end(),
                nu_ansi_term::ansi::RESET,
            );
            String::insert_str(&mut current_line, leading_match.start(), &bg_color_string);
        }

        if let Some(trailing_match) = re_trailing.find(&current_line.clone()) {
            String::insert_str(&mut current_line, trailing_match.start(), &bg_color_string);
            current_line += nu_ansi_term::ansi::RESET;
        }

        output.push(WrappedLine {
            line: current_line,
            width: current_line_width,
        });
    }

    (output, current_max)
}

pub fn wrap(
    headers: &[StyledString],
    data: &[Vec<StyledString>],
    termwidth: usize,
    theme: &TableTheme,
) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    // Remove the edges, if used
    let edges_width = if theme.is_left_set && theme.is_right_set {
        3
    } else if theme.is_left_set || theme.is_right_set {
        1
    } else {
        0
    };

    if termwidth < edges_width {
        return None;
    }

    let termwidth = termwidth - edges_width;

    let (mut headers_splited, mut data_splited) = split_lines(headers, data);

    let max_per_column = get_max_column_widths(&headers_splited, &data_splited);

    maybe_truncate_columns(termwidth, &mut headers_splited, &mut data_splited);

    let mut headers_len = headers_splited.len();
    if headers_len == 0 {
        if !data.is_empty() && !data[0].is_empty() {
            headers_len = data_splited[0].len();
        } else {
            return Some((Vec::new(), Vec::new()));
        }
    }

    // Measure how big our columns need to be (accounting for separators also)
    let max_naive_column_width = (termwidth - 3 * (headers_len - 1)) / headers_len;

    let column_space = ColumnSpace::measure(&max_per_column, max_naive_column_width, headers_len);

    // This gives us the max column width
    let max_column_width = column_space.max_width(termwidth)?;

    // This width isn't quite right, as we're rounding off some of our space
    let column_space = column_space.fix_almost_column_width(
        &max_per_column,
        max_naive_column_width,
        max_column_width,
        headers_len,
    );

    // This should give us the final max column width
    let max_column_width = column_space.max_width(termwidth)?;

    let re_leading =
        regex::Regex::new(r"(?P<beginsp>^\s+)").expect("error with leading space regex");
    let re_trailing =
        regex::Regex::new(r"(?P<endsp>\s+$)").expect("error with trailing space regex");

    let result = wrap_cells(
        headers_splited,
        data_splited,
        max_column_width,
        &re_leading,
        &re_trailing,
    );

    Some(result)
}

struct ContentLines {
    pub lines: Vec<Vec<Subline>>,
    pub style: TextStyle,
}

fn split_lines(
    headers: &[StyledString],
    data: &[Vec<StyledString>],
) -> (Vec<ContentLines>, Vec<Vec<ContentLines>>) {
    let mut splited_headers = Vec::with_capacity(headers.len());
    for column in headers {
        let content = clean(&column.contents);
        let lines = split_sublines(&content);
        splited_headers.push(ContentLines {
            lines,
            style: column.style,
        });
    }

    let mut splited_data = Vec::with_capacity(data.len());
    for row in data {
        let mut splited_row = Vec::with_capacity(row.len());
        for column in row {
            let content = clean(&column.contents);
            let lines = split_sublines(&content);
            splited_row.push(ContentLines {
                lines,
                style: column.style,
            });
        }

        splited_data.push(splited_row);
    }

    (splited_headers, splited_data)
}

fn get_max_column_widths(headers: &[ContentLines], data: &[Vec<ContentLines>]) -> Vec<usize> {
    use std::cmp::max;

    let mut max_num_columns = 0;

    max_num_columns = max(max_num_columns, headers.len());

    for row in data {
        max_num_columns = max(max_num_columns, row.len());
    }

    let mut output = vec![0; max_num_columns];

    for (col, content) in headers.iter().enumerate() {
        output[col] = max(output[col], column_width(&content.lines));
    }

    for row in data {
        for (col, content) in row.iter().enumerate() {
            output[col] = max(output[col], column_width(&content.lines));
        }
    }

    output
}

fn wrap_cells(
    headers_splited: Vec<ContentLines>,
    data_splited: Vec<Vec<ContentLines>>,
    max_column_width: usize,
    re_leading: &regex::Regex,
    re_trailing: &regex::Regex,
) -> (Vec<String>, Vec<Vec<String>>) {
    let mut header = vec![String::new(); headers_splited.len()];
    for (col, splited) in headers_splited.into_iter().enumerate() {
        let mut wrapped = vec![];
        for contents in splited.lines {
            let (mut lines, _) = wrap_content(
                max_column_width,
                contents.into_iter(),
                &HashMap::new(),
                re_leading,
                re_trailing,
            );
            wrapped.append(&mut lines);
        }

        let content = wrapped
            .into_iter()
            .map(|l| l.line)
            .collect::<Vec<_>>()
            .join("\n");
        let content = splited
            .style
            .color_style
            .map(|color| color.paint(&content).to_string())
            .unwrap_or(content);

        header[col] = content;
    }

    let mut data = vec![Vec::new(); data_splited.len()];
    for (row, splited) in data_splited.into_iter().enumerate() {
        for splited in splited.into_iter() {
            let mut wrapped = vec![];
            for contents in splited.lines {
                let (mut lines, _) = wrap_content(
                    max_column_width,
                    contents.into_iter(),
                    &HashMap::new(),
                    re_leading,
                    re_trailing,
                );
                wrapped.append(&mut lines);
            }

            let content = wrapped
                .into_iter()
                .map(|l| l.line)
                .collect::<Vec<_>>()
                .join("\n");
            let content = splited
                .style
                .color_style
                .map(|color| color.paint(&content).to_string())
                .unwrap_or(content);

            data[row].push(content);
        }
    }

    (header, data)
}

fn maybe_truncate_columns(
    termwidth: usize,
    headers: &mut Vec<ContentLines>,
    data: &mut [Vec<ContentLines>],
) {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;

    // If we have too many columns, truncate the table
    if max_num_of_columns < headers.len() {
        headers.truncate(max_num_of_columns);
        headers.push(ContentLines {
            lines: vec![vec![Subline {
                subline: String::from("..."),
                width: 3,
            }]],
            style: TextStyle::basic_center(),
        });
    }

    if max_num_of_columns < headers.len() {
        for entry in data.iter_mut() {
            entry.truncate(max_num_of_columns);
            entry.push(ContentLines {
                lines: vec![vec![Subline {
                    subline: String::from("..."),
                    width: 3,
                }]],
                style: TextStyle::basic_center(),
            });
        }
    }
}

struct ColumnSpace {
    num_overages: usize,
    underage_sum: usize,
    overage_separator_sum: usize,
}

impl ColumnSpace {
    /// Measure how much space we have once we subtract off the columns who are small enough
    fn measure(
        max_per_column: &[usize],
        max_naive_column_width: usize,
        headers_len: usize,
    ) -> ColumnSpace {
        let mut num_overages = 0;
        let mut underage_sum = 0;
        let mut overage_separator_sum = 0;
        let iter = max_per_column.iter().enumerate().take(headers_len);

        for (i, &column_max) in iter {
            if column_max > max_naive_column_width {
                num_overages += 1;
                if i != (headers_len - 1) {
                    overage_separator_sum += 3;
                }
                if i == 0 {
                    overage_separator_sum += 1;
                }
            } else {
                underage_sum += column_max;
                // if column isn't last, add 3 for its separator
                if i != (headers_len - 1) {
                    underage_sum += 3;
                }
                if i == 0 {
                    underage_sum += 1;
                }
            }
        }

        ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        }
    }

    fn fix_almost_column_width(
        self,
        max_per_column: &[usize],
        max_naive_column_width: usize,
        max_column_width: usize,
        headers_len: usize,
    ) -> ColumnSpace {
        let mut num_overages = 0;
        let mut overage_separator_sum = 0;
        let mut underage_sum = self.underage_sum;
        let iter = max_per_column.iter().enumerate().take(headers_len);

        for (i, &column_max) in iter {
            if column_max > max_naive_column_width {
                if column_max <= max_column_width {
                    underage_sum += column_max;
                    // if column isn't last, add 3 for its separator
                    if i != (headers_len - 1) {
                        underage_sum += 3;
                    }
                    if i == 0 {
                        underage_sum += 1;
                    }
                } else {
                    // Column is still too large, so let's count it
                    num_overages += 1;
                    if i != (headers_len - 1) {
                        overage_separator_sum += 3;
                    }
                    if i == 0 {
                        overage_separator_sum += 1;
                    }
                }
            }
        }

        ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        }
    }

    fn max_width(&self, termwidth: usize) -> Option<usize> {
        let ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        } = self;

        if *num_overages > 0 {
            termwidth
                .checked_sub(1)?
                .checked_sub(*underage_sum)?
                .checked_sub(*overage_separator_sum)?
                .checked_div(*num_overages)
        } else {
            Some(99999)
        }
    }
}

fn clean(input: &str) -> String {
    let input = input.replace('\r', "");

    input.replace('\t', "    ")
}
