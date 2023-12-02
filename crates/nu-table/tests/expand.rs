mod common;

use common::{create_row, create_table};
use nu_table::{NuTableConfig, TableTheme as theme};

#[test]
fn test_expand() {
    let table = create_table(
        vec![create_row(4); 3],
        NuTableConfig {
            theme: theme::rounded(),
            with_header: true,
            expand: true,
            ..Default::default()
        },
        50,
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
