#![allow(dead_code)]

use nu_protocol::{TableIndent, TrimStrategy};
use nu_table::{NuTable, TableTheme, string_width};
use tabled::grid::records::vec_records::Text;

#[derive(Debug, Clone)]
pub struct TestCase {
    theme: TableTheme,
    with_header: bool,
    with_footer: bool,
    with_index: bool,
    expand: bool,
    strategy: TrimStrategy,
    indent: TableIndent,
    header_on_border: bool,
    priorities: Vec<usize>,
    termwidth: usize,
    expected: Option<String>,
}

impl TestCase {
    pub fn new(termwidth: usize) -> Self {
        Self {
            termwidth,
            expected: None,
            theme: TableTheme::basic(),
            with_header: false,
            with_footer: false,
            with_index: false,
            expand: false,
            strategy: TrimStrategy::truncate(None),
            indent: TableIndent::new(1, 1),
            header_on_border: false,
            priorities: vec![],
        }
    }

    pub fn width(mut self, termwidth: usize) -> Self {
        self.termwidth = termwidth;
        self
    }

    pub fn indent(mut self, indent: TableIndent) -> Self {
        self.indent = indent;
        self
    }

    pub fn header_on_border(mut self, on: bool) -> Self {
        self.header_on_border = on;
        self
    }

    pub fn priorities(mut self, columns: Vec<usize>) -> Self {
        self.priorities = columns;
        self
    }

    pub fn expected(mut self, value: Option<String>) -> Self {
        self.expected = value;
        self
    }

    pub fn theme(mut self, theme: TableTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn expand(mut self) -> Self {
        self.expand = true;
        self
    }

    pub fn header(mut self) -> Self {
        self.with_header = true;
        self
    }

    pub fn footer(mut self) -> Self {
        self.with_footer = true;
        self
    }

    pub fn index(mut self) -> Self {
        self.with_index = true;
        self
    }

    pub fn trim(mut self, trim: TrimStrategy) -> Self {
        self.strategy = trim;
        self
    }
}

type Data = Vec<Vec<Text<String>>>;

pub fn test_table<I>(data: Data, tests: I)
where
    I: IntoIterator<Item = TestCase>,
{
    for (i, test) in tests.into_iter().enumerate() {
        let actual = create_table(data.clone(), test.clone());

        assert_eq!(
            actual, test.expected,
            "\nfail i={:?} termwidth={}",
            i, test.termwidth
        );

        if let Some(table) = actual {
            assert!(string_width(&table) <= test.termwidth);
        }
    }
}

pub fn create_table(data: Data, case: TestCase) -> Option<String> {
    let count_rows = data.len();
    let count_cols = data[0].len();

    let mut table = NuTable::new(count_rows, count_cols);
    // The indent sets the base column widths, so it goes before the data.
    table.set_indent(case.indent);
    for (i, row) in data.into_iter().enumerate() {
        table.set_row(i, row);
    }

    table.set_theme(case.theme);
    table.set_structure(case.with_index, case.with_header, case.with_footer);
    table.set_trim(case.strategy);
    table.set_strategy(case.expand);
    table.set_border_header(case.header_on_border);
    table.set_width_priority_columns(&case.priorities);

    table.draw(case.termwidth)
}

/// The cells of one line of a bordered table: the line without its outer
/// border characters, split at `separator`.
pub fn split_cells(line: &str, separator: char) -> Vec<&str> {
    let mut chars = line.chars();
    chars.next();
    chars.next_back();
    chars.as_str().split(separator).collect()
}

/// Width of a table with these column widths (padding included) and
/// one-character borders and separators.
pub fn bordered_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + widths.len() + 1
}

pub fn create_row(count_columns: usize) -> Vec<Text<String>> {
    let mut row = Vec::with_capacity(count_columns);
    for i in 0..count_columns {
        row.push(Text::new(i.to_string()));
    }

    row
}

pub fn cell(text: &str) -> Text<String> {
    Text::new(text.to_string())
}
