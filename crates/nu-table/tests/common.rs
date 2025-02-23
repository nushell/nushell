#![allow(dead_code)]

use nu_protocol::TrimStrategy;
use nu_table::{string_width, NuTable, TableTheme};
use tabled::grid::records::vec_records::Text;

#[derive(Debug, Clone)]
pub struct TestCase {
    theme: TableTheme,
    with_header: bool,
    with_footer: bool,
    with_index: bool,
    expand: bool,
    strategy: TrimStrategy,
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
        }
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
    let mut table = NuTable::from(data);
    table.set_theme(case.theme);
    table.set_structure(case.with_index, case.with_header, case.with_footer);
    table.set_trim(case.strategy);
    table.set_strategy(case.expand);

    table.draw(case.termwidth)
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
