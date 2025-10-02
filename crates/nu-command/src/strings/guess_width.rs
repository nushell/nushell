/// Attribution: https://github.com/noborus/guesswidth/blob/main/guesswidth.go
/// The MIT License (MIT) as of 2024-03-22
///
/// GuessWidth handles the format as formatted by printf.
/// Spaces exist as delimiters, but spaces are not always delimiters.
/// The width seems to be a fixed length, but it doesn't always fit.
/// GuessWidth finds the column separation position
/// from the reference line(header) and multiple lines(body).
///
/// Briefly, the algorithm uses a histogram of spaces to find widths.
/// blanks, lines, and pos are variables used in the algorithm. The other
/// items names below are just for reference.
/// blanks =  0000003000113333111100003000
///  lines = "   PID TTY          TIME CMD"
///          "302965 pts/3    00:00:11 zsh"
///          "709737 pts/3    00:00:00 ps"
///
/// measure= "012345678901234567890123456789"
/// spaces = "      ^        ^        ^"
///    pos =  6 15 24 <- the carets show these positions
/// the items in pos map to 3's in the blanks array
///
/// Now that we have pos, we can let split() use this pos array to figure out
/// how to split all lines by comparing each index to see if there's a space.
/// So, it looks at position 6, 15, 24 and sees if it has a space in those
/// positions. If it does, it splits the line there. If it doesn't, it wiggles
/// around the position to find the next space and splits there.
use std::io::{self, BufRead};
use unicode_width::UnicodeWidthStr;

/// the number to scan to analyze.
const SCAN_NUM: u8 = 128;
/// the minimum number of lines to recognize as a separator.
/// 1 if only the header, 2 or more if there is a blank in the body.
const MIN_LINES: usize = 2;
/// whether to trim the space in the value.
const TRIM_SPACE: bool = true;
/// the base line number.  It starts from 0.
const HEADER: usize = 0;

/// GuessWidth reads records from printf-like output.
pub struct GuessWidth {
    pub(crate) reader: io::BufReader<Box<dyn io::Read>>,
    // a list of separator positions.
    pub(crate) pos: Vec<usize>,
    // stores the lines read for scan.
    pub(crate) pre_lines: Vec<String>,
    // the number returned by read.
    pub(crate) pre_count: usize,
    // the maximum number of columns to split.
    pub(crate) limit_split: usize,
}

impl GuessWidth {
    pub fn new_reader(r: Box<dyn io::Read>) -> GuessWidth {
        let reader = io::BufReader::new(r);
        GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        }
    }

    /// read_all reads all rows
    /// and returns a two-dimensional slice of rows and columns.
    pub fn read_all(&mut self) -> Vec<Vec<String>> {
        if self.pre_lines.is_empty() {
            self.scan(SCAN_NUM);
        }

        let mut rows = Vec::new();
        while let Ok(columns) = self.read() {
            if !columns.is_empty() {
                rows.push(columns);
            }
        }
        rows
    }

    /// scan preReads and parses the lines.
    fn scan(&mut self, num: u8) {
        for _ in 0..num {
            let mut buf = String::new();
            if self.reader.read_line(&mut buf).unwrap_or(0) == 0 {
                break;
            }

            let line = buf.trim_end().to_string();
            self.pre_lines.push(line);
        }

        self.pos = positions(&self.pre_lines, HEADER, MIN_LINES);
        if self.limit_split > 0 && self.pos.len() > self.limit_split {
            self.pos.truncate(self.limit_split);
        }
    }

    /// read reads one row and returns a slice of columns.
    /// scan is executed first if it is not preRead.
    fn read(&mut self) -> Result<Vec<String>, io::Error> {
        if self.pre_lines.is_empty() {
            self.scan(SCAN_NUM);
        }

        if self.pre_count < self.pre_lines.len() {
            let line = &self.pre_lines[self.pre_count];
            self.pre_count += 1;
            Ok(split(line, &self.pos, TRIM_SPACE))
        } else {
            let mut buf = String::new();
            if self.reader.read_line(&mut buf)? == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "End of file"));
            }

            let line = buf.trim_end().to_string();
            Ok(split(&line, &self.pos, TRIM_SPACE))
        }
    }
}

// positions returns separator positions
// from multiple lines and header line number.
// Lines before the header line are ignored.
fn positions(lines: &[String], header: usize, min_lines: usize) -> Vec<usize> {
    let mut blanks = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        if n < header {
            continue;
        }

        if n == header {
            blanks = lookup_blanks(line.trim_end_matches(' '));
            continue;
        }

        count_blanks(&mut blanks, line.trim_end_matches(' '));
    }

    positions_helper(&blanks, min_lines)
}

fn separator_position(lr: &[char], p: usize, pos: &[usize], n: usize) -> usize {
    if lr[p].is_whitespace() {
        return p;
    }

    let mut f = p;
    while f < lr.len() && !lr[f].is_whitespace() {
        f += 1;
    }

    let mut b = p;
    while b > 0 && !lr[b].is_whitespace() {
        b -= 1;
    }

    if b == pos[n] {
        return f;
    }

    if n < pos.len() - 1 {
        if f == pos[n + 1] {
            return b;
        }
        if b == pos[n] {
            return f;
        }
        if b > pos[n] && b < pos[n + 1] {
            return b;
        }
    }

    f
}

fn split(line: &str, pos: &[usize], trim_space: bool) -> Vec<String> {
    let mut n = 0;
    let mut start_char = 0;
    let mut columns = Vec::with_capacity(pos.len() + 1);
    let (line_char_boundaries, line_chars): (Vec<usize>, Vec<char>) = line.char_indices().unzip();
    let mut w = 0;

    if line_chars.is_empty() || line_chars.iter().all(|&c| c.is_whitespace()) {
        // current line is completely empty, or only filled with whitespace
        return Vec::new();
    } else if !pos.is_empty()
        && line_chars.iter().all(|&c| !c.is_whitespace())
        && pos[0] < UnicodeWidthStr::width(line)
    {
        // we have more than 1 column in the input, but the current line has no whitespace,
        // and it is longer than the first detected column separation position
        // this indicates some kind of decoration line. let's skip it
        return Vec::new();
    }

    for p in 0..line_char_boundaries.len() {
        if pos.is_empty() || n > pos.len() - 1 {
            start_char = p;
            break;
        }

        if pos[n] <= w {
            let end_char = separator_position(&line_chars, p, pos, n);
            if start_char > end_char || end_char >= line_char_boundaries.len() {
                break;
            }
            let col = &line[line_char_boundaries[start_char]..line_char_boundaries[end_char]];
            let col = if trim_space { col.trim() } else { col };
            columns.push(col.to_string());
            n += 1;
            start_char = end_char;
        }

        w += UnicodeWidthStr::width(line_chars[p].to_string().as_str());
    }

    // add last part.
    let col = &line[line_char_boundaries[start_char]..];
    let col = if trim_space { col.trim() } else { col };
    columns.push(col.to_string());
    columns
}

// Creates a blank(1) and non-blank(0) slice.
// Execute for the base line (header line).
fn lookup_blanks(line: &str) -> Vec<usize> {
    let mut blanks = Vec::new();
    let mut first = true;

    for c in line.chars() {
        if c == ' ' {
            if first {
                blanks.push(0);
                continue;
            }
            blanks.push(1);
            continue;
        }

        first = false;
        blanks.push(0);
        if UnicodeWidthStr::width(c.to_string().as_str()) == 2 {
            blanks.push(0);
        }
    }

    blanks
}

// count up if the line is blank where the reference line was blank.
fn count_blanks(blanks: &mut [usize], line: &str) {
    let mut n = 0;

    for c in line.chars() {
        if n >= blanks.len() {
            break;
        }

        if c == ' ' && blanks[n] > 0 {
            blanks[n] += 1;
        }

        n += 1;
        if UnicodeWidthStr::width(c.to_string().as_str()) == 2 {
            n += 1;
        }
    }
}

// Generates a list of separator positions from a blank slice.
fn positions_helper(blanks: &[usize], min_lines: usize) -> Vec<usize> {
    let mut max = min_lines;
    let mut p = 0;
    let mut pos = Vec::new();

    for (n, v) in blanks.iter().enumerate() {
        if *v >= max {
            max = *v;
            p = n;
        }
        if *v == 0 {
            max = min_lines;
            if p > 0 {
                pos.push(p);
                p = 0;
            }
        }
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    /// to_rows returns rows separated by columns.
    fn to_rows(lines: Vec<String>, pos: Vec<usize>, trim_space: bool) -> Vec<Vec<String>> {
        let mut rows: Vec<Vec<String>> = Vec::with_capacity(lines.len());
        for line in lines {
            let columns = split(&line, &pos, trim_space);
            rows.push(columns);
        }
        rows
    }

    /// to_table parses a slice of lines and returns a table.
    pub fn to_table(lines: Vec<String>, header: usize, trim_space: bool) -> Vec<Vec<String>> {
        let pos = positions(&lines, header, 2);
        to_rows(lines, pos, trim_space)
    }

    /// to_table_n parses a slice of lines and returns a table, but limits the number of splits.
    pub fn to_table_n(
        lines: Vec<String>,
        header: usize,
        num_split: usize,
        trim_space: bool,
    ) -> Vec<Vec<String>> {
        let mut pos = positions(&lines, header, 2);
        if pos.len() > num_split {
            pos.truncate(num_split);
        }
        to_rows(lines, pos, trim_space)
    }

    #[test]
    fn test_guess_width_ps_trim() {
        let input = "   PID TTY          TIME CMD
302965 pts/3    00:00:11 zsh
709737 pts/3    00:00:00 ps";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        #[rustfmt::skip]
        let want = vec![
            vec!["PID", "TTY", "TIME", "CMD"],
            vec!["302965", "pts/3", "00:00:11", "zsh"],
            vec!["709737", "pts/3", "00:00:00", "ps"],
        ];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_ps_overflow_trim() {
        let input = "USER         PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
root           1  0.0  0.0 168576 13788 ?        Ss   Mar11   0:49 /sbin/init splash
noborus   703052  2.1  0.7 1184814400 230920 ?   Sl   10:03   0:45 /opt/google/chrome/chrome
noborus   721971  0.0  0.0  13716  3524 pts/3    R+   10:39   0:00 ps aux";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        #[rustfmt::skip]
        let want = vec![
            vec!["USER", "PID", "%CPU", "%MEM", "VSZ", "RSS", "TTY", "STAT", "START", "TIME", "COMMAND"],
            vec!["root", "1", "0.0", "0.0", "168576", "13788", "?", "Ss", "Mar11", "0:49", "/sbin/init splash"],
            vec!["noborus", "703052", "2.1", "0.7", "1184814400", "230920", "?", "Sl", "10:03", "0:45", "/opt/google/chrome/chrome"],
            vec!["noborus", "721971", "0.0", "0.0", "13716", "3524", "pts/3", "R+", "10:39", "0:00", "ps aux"],
        ];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_ps_limit_trim() {
        let input = "   PID TTY          TIME CMD
302965 pts/3    00:00:11 zsh
709737 pts/3    00:00:00 ps";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 2,
        };

        #[rustfmt::skip]
        let want = vec![
            vec!["PID", "TTY", "TIME CMD"],
            vec!["302965", "pts/3", "00:00:11 zsh"],
            vec!["709737", "pts/3", "00:00:00 ps"],
        ];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_windows_df_trim() {
        let input = "Filesystem     1K-blocks      Used Available Use% Mounted on
C:/Apps/Git    998797308 869007000 129790308  88% /
D:             104792064  17042676  87749388  17% /d";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        #[rustfmt::skip]
        let want = vec![
            vec!["Filesystem","1K-blocks","Used","Available","Use%","Mounted on"],
            vec!["C:/Apps/Git","998797308","869007000","129790308","88%","/"],
            vec!["D:","104792064","17042676","87749388","17%","/d"],
        ];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_multibyte() {
        let input = "A… B\nC… D";
        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![vec!["A…", "B"], vec!["C…", "D"]];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_combining_diacritical_marks() {
        let input = "Name        Surname
Ștefan         Țincu ";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![vec!["Name", "Surname"], vec!["Ștefan", "Țincu"]];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_single_column() {
        let input = "A

B

C";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![vec!["A"], vec!["B"], vec!["C"]];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_row_without_whitespace() {
        let input = "A B C D
-------
E F G H";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![vec!["A", "B", "C", "D"], vec!["E", "F", "G", "H"]];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_row_with_single_column() {
        let input = "A B C D
E
F G H I";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![
            vec!["A", "B", "C", "D"],
            vec!["E"],
            vec!["F", "G", "H", "I"],
        ];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_empty_row() {
        let input = "A B C D

E F G H";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![vec!["A", "B", "C", "D"], vec!["E", "F", "G", "H"]];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_width_row_with_only_whitespace() {
        let input = "A B C D

E F G H";

        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let want = vec![vec!["A", "B", "C", "D"], vec!["E", "F", "G", "H"]];
        let got = guess_width.read_all();
        assert_eq!(got, want);
    }

    #[test]
    fn test_guess_no_panic_for_some_cases() {
        let input = r#"nu_plugin_highlight = '1.2.2+0.97.1'    # A nushell plugin for syntax highlighting
trace_nu_plugin = '0.3.1'               # A wrapper to trace Nu plugins
nu_plugin_bash_env = '0.13.0'           # Nu plugin bash-env
nu_plugin_from_sse = '0.4.0'            # Nushell plugin to convert a HTTP server sent event stream to structured data
... and 90 crates more (use --limit N to see more)"#;
        let r = Box::new(std::io::BufReader::new(input.as_bytes())) as Box<dyn std::io::Read>;
        let reader = std::io::BufReader::new(r);

        let mut guess_width = GuessWidth {
            reader,
            pos: Vec::new(),
            pre_lines: Vec::new(),
            pre_count: 0,
            limit_split: 0,
        };

        let first_column_want = [
            "nu_plugin_highlight = '1.2.2+0.97.1'",
            "trace_nu_plugin = '0.3.1'",
            "nu_plugin_bash_env = '0.13.0'",
            "nu_plugin_from_sse = '0.4.0'",
            "... and 90 crates more (use --limit N",
        ];
        let got = guess_width.read_all();
        for (row_index, row) in got.into_iter().enumerate() {
            assert_eq!(row[0], first_column_want[row_index]);
        }
    }

    #[test]
    fn test_to_table() {
        let lines = vec![
            "   PID TTY          TIME CMD".to_string(),
            "302965 pts/3    00:00:11 zsh".to_string(),
            "709737 pts/3    00:00:00 ps".to_string(),
        ];

        let want = vec![
            vec!["PID", "TTY", "TIME", "CMD"],
            vec!["302965", "pts/3", "00:00:11", "zsh"],
            vec!["709737", "pts/3", "00:00:00", "ps"],
        ];

        let header = 0;
        let trim_space = true;
        let table = to_table(lines, header, trim_space);
        assert_eq!(table, want);
    }

    #[test]
    fn test_to_table_n() {
        let lines = vec![
            "2022-12-21T09:50:16+0000 WARN A warning that should be ignored is usually at this level and should be actionable.".to_string(),
    		"2022-12-21T09:50:17+0000 INFO This is less important than debug log and is often used to provide context in the current task.".to_string(),
        ];

        let want = vec![
            vec![
                "2022-12-21T09:50:16+0000",
                "WARN",
                "A warning that should be ignored is usually at this level and should be actionable.",
            ],
            vec![
                "2022-12-21T09:50:17+0000",
                "INFO",
                "This is less important than debug log and is often used to provide context in the current task.",
            ],
        ];

        let header = 0;
        let trim_space = true;
        let num_split = 2;
        let table = to_table_n(lines, header, num_split, trim_space);
        assert_eq!(table, want);
    }
}
