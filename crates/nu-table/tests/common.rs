#![allow(dead_code)]

use nu_table::{string_width, NuTable, NuTableConfig};
use tabled::grid::records::vec_records::Text;

pub struct TestCase {
    cfg: NuTableConfig,
    termwidth: usize,
    expected: Option<String>,
}

impl TestCase {
    pub fn new(cfg: NuTableConfig, termwidth: usize, expected: Option<String>) -> Self {
        Self {
            cfg,
            termwidth,
            expected,
        }
    }
}

type Data = Vec<Vec<Text<String>>>;

pub fn test_table<I: IntoIterator<Item = TestCase>>(data: Data, tests: I) {
    for (i, test) in tests.into_iter().enumerate() {
        let actual = create_table(data.clone(), test.cfg.clone(), test.termwidth);

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

pub fn create_table(data: Data, config: NuTableConfig, termwidth: usize) -> Option<String> {
    let table = NuTable::from(data);
    table.draw(config, termwidth)
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
