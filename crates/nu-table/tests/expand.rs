mod common;

use common::{create_row, create_table, TestCase};

use nu_table::TableTheme as theme;

#[test]
fn test_expand() {
    let table = create_table(
        vec![create_row(4); 3],
        TestCase::new(50).theme(theme::rounded()).header().expand(),
    );

    assert_eq!(
        table.unwrap(),
        "╭────────────┬───────────┬───────────┬───────────╮\n\
         │     0      │     1     │     2     │     3     │\n\
         ├────────────┼───────────┼───────────┼───────────┤\n\
         │ 0          │ 1         │ 2         │ 3         │\n\
         │ 0          │ 1         │ 2         │ 3         │\n\
         ╰────────────┴───────────┴───────────┴───────────╯"
    );
}
