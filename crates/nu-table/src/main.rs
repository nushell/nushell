use nu_table::{draw_table, StyledString, Table, TextStyle, Theme};

fn main() {
    let args: Vec<_> = std::env::args().collect();

    let width = args[1].parse::<usize>().unwrap();
    let msg = args[2..]
        .iter()
        .map(|x| StyledString::new(x.to_owned(), TextStyle::basic()))
        .collect();

    let t = Table::new(
        vec![
            StyledString::new("Test me".to_owned(), TextStyle::default_header()),
            StyledString::new("Long column name".to_owned(), TextStyle::default_header()),
            StyledString::new("Another".to_owned(), TextStyle::default_header()),
        ],
        vec![msg; 2],
        Theme::compact(),
    );

    draw_table(&t, width);
}
