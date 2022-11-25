use std::collections::HashMap;

use nu_protocol::Config;
use nu_table::{Alignments, Table, TableTheme as theme, TextStyle};
use tabled::papergrid::records::{cell_info::CellInfo, tcell::TCell};

#[test]
fn test_expand() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::rounded(), 50),
        "╭────────────┬───────────┬───────────┬───────────╮\n\
         │     0      │     1     │     2     │     3     │\n\
         ├────────────┼───────────┼───────────┼───────────┤\n\
         │ 0          │ 1         │ 2         │ 3         │\n\
         │ 0          │ 1         │ 2         │ 3         │\n\
         ╰────────────┴───────────┴───────────┴───────────╯"
    );
}

fn draw_table(
    data: Vec<Vec<TCell<CellInfo<'static>, TextStyle>>>,
    count_columns: usize,
    with_header: bool,
    theme: theme,
    width: usize,
) -> String {
    let size = (data.len(), count_columns);
    let table = Table::new(data, size, width, with_header, false);

    let cfg = Config::default();
    let styles = HashMap::default();
    let alignments = Alignments::default();
    table
        .draw_table(&cfg, &styles, alignments, &theme, width, true)
        .expect("Unexpectdly got no table")
}

fn row(count_columns: usize) -> Vec<TCell<CellInfo<'static>, TextStyle>> {
    let mut row = Vec::with_capacity(count_columns);

    for i in 0..count_columns {
        row.push(Table::create_cell(i.to_string(), TextStyle::default()));
    }

    row
}
