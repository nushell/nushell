mod common;

use common::{create_table};

use nu_table::{TableConfig, TableTheme as theme, Table};

#[test]
fn replace_tab() {
    let table = create_table(
        vec![vec![Table::create_cell("123\t345", Default::default())]; 3],
        TableConfig::new(theme::rounded(), true, false, false).expand(),
        50,
    );

    assert_eq!(
        table.unwrap(),
        "╭────────────────────────────────────────────────╮\n\
         │                   123    345                   │\n\
         ├────────────────────────────────────────────────┤\n\
         │ 123    345                                     │\n\
         │ 123    345                                     │\n\
         ╰────────────────────────────────────────────────╯"
    );
}

#[test]
fn replace_u202e_tab() {
    let table = create_table(
        vec![vec![Table::create_cell("123\u{202E}345", Default::default())]; 3],
        TableConfig::new(theme::rounded(), true, false, false).expand(),
        50,
    );

    assert_eq!(
        table.unwrap(),
        "╭────────────────────────────────────────────────╮\n\
         │                     123345                     │\n\
         ├────────────────────────────────────────────────┤\n\
         │ 123345                                         │\n\
         │ 123345                                         │\n\
         ╰────────────────────────────────────────────────╯"
    );
}
