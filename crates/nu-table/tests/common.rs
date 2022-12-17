use nu_table::{string_width, Table, TableConfig, TextStyle};
use tabled::papergrid::records::{cell_info::CellInfo, tcell::TCell};

pub type VecCells = Vec<Vec<TCell<CellInfo<'static>, TextStyle>>>;

#[allow(dead_code)]
pub struct TestCase {
    cfg: TableConfig,
    termwidth: usize,
    expected: Option<String>,
}

impl TestCase {
    #[allow(dead_code)]
    pub fn new(cfg: TableConfig, termwidth: usize, expected: Option<String>) -> Self {
        Self {
            cfg,
            termwidth,
            expected,
        }
    }
}

#[allow(dead_code)]
pub fn test_table<I>(data: VecCells, tests: I)
where
    I: IntoIterator<Item = TestCase>,
{
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

pub fn create_table(data: VecCells, config: TableConfig, termwidth: usize) -> Option<String> {
    let mut size = (0, 0);
    for row in &data {
        size.0 += 1;
        size.1 = std::cmp::max(size.1, row.len());
    }

    let table = Table::new(data, size);
    table.draw(config, termwidth)
}

pub fn create_row(count_columns: usize) -> Vec<TCell<CellInfo<'static>, TextStyle>> {
    let mut row = Vec::with_capacity(count_columns);

    for i in 0..count_columns {
        row.push(Table::create_cell(i.to_string(), TextStyle::default()));
    }

    row
}

#[allow(dead_code)]
pub fn _str(s: &str) -> TCell<CellInfo<'static>, TextStyle> {
    Table::create_cell(s.to_string(), TextStyle::default())
}
