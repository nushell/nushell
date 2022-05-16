use crate::textstyle::TextStyle;
use ansi_str::AnsiStr;
use nu_ansi_term::Style;
use std::borrow::Cow;
use std::collections::HashMap;
use std::{fmt::Display, iter::Iterator};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

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

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for subline in &self.sublines {
            if !first {
                write!(f, " ")?;
            } else {
                first = false;
            }
            write!(f, "{}", subline.subline)?;
        }
        Ok(())
    }
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

pub fn wrap(
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
