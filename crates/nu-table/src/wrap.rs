use crate::table::TextStyle;
use nu_ansi_term::Style;
use std::collections::HashMap;
use std::{fmt::Display, iter::Iterator};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug)]
pub struct Subline<'a> {
    pub subline: &'a str,
    pub width: usize,
}

#[derive(Debug)]
pub struct Line<'a> {
    pub sublines: Vec<Subline<'a>>,
    pub width: usize,
}

#[derive(Debug)]
pub struct WrappedLine {
    pub line: String,
    pub width: usize,
}

#[derive(Debug)]
pub struct WrappedCell {
    pub lines: Vec<WrappedLine>,
    pub max_width: usize,

    pub style: TextStyle,
}

impl<'a> Display for Line<'a> {
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

pub fn split_sublines(input: &str) -> Vec<Vec<Subline>> {
    input
        .split_terminator('\n')
        .map(|line| {
            line.split_terminator(' ')
                .map(|x| Subline {
                    subline: x,
                    width: {
                        // We've tried UnicodeWidthStr::width(x), UnicodeSegmentation::graphemes(x, true).count()
                        // and x.chars().count() with all types of combinations. Currently, it appears that
                        // getting the max of char count and unicode width seems to produce the best layout.
                        // However, it's not perfect.
                        let c = x.chars().count();
                        let u = UnicodeWidthStr::width(x);
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
    use unicode_width::UnicodeWidthChar;

    let mut output = vec![];
    let mut current_width = 0;
    let mut start_index = 0;
    let mut end_index;

    for c in word.char_indices() {
        if let Some(width) = c.1.width() {
            end_index = c.0;
            if current_width + width > cell_width {
                output.push(Subline {
                    subline: &word[start_index..end_index],
                    width: current_width,
                });

                start_index = c.0;
                current_width = width;
            } else {
                current_width += width;
            }
        }
    }

    if start_index != word.len() {
        output.push(Subline {
            subline: &word[start_index..],
            width: current_width,
        });
    }

    output
}

pub fn wrap<'a>(
    cell_width: usize,
    mut input: impl Iterator<Item = Subline<'a>>,
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
                current_line.push_str(subline.subline);
            } else {
                first = false;
                current_line_width = subline.width;
                current_line.push_str(subline.subline);
            }
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
            String::insert_str(&mut current_line, leading_match.end(), "\x1b[0m");
            String::insert_str(&mut current_line, leading_match.start(), &bg_color_string);
        }

        if let Some(trailing_match) = re_trailing.find(&current_line.clone()) {
            String::insert_str(&mut current_line, trailing_match.start(), &bg_color_string);
            current_line += "\x1b[0m";
        }

        output.push(WrappedLine {
            line: current_line,
            width: current_line_width,
        });
    }

    (output, current_max)
}
