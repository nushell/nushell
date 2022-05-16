use nu_protocol::Config;
use nu_table::{draw_table, StyledString, Table, TableTheme, TextStyle};
use std::collections::HashMap;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut width = 0;

    if args.len() > 1 {
        // Width in terminal characters
        width = args[1].parse::<usize>().expect("Need a width in columns");
    }

    if width < 4 {
        println!("Width must be greater than or equal to 4, setting width to 80");
        width = 80;
    }

    // The mocked up table data
    let (table_headers, row_data) = make_table_data();
    // The table headers
    let headers = vec_of_str_to_vec_of_styledstr(&table_headers, true);
    // The table rows
    let rows = vec_of_str_to_vec_of_styledstr(&row_data, false);
    // The table itself
    let table = Table::new(headers, vec![rows; 3], TableTheme::rounded());
    // FIXME: Config isn't available from here so just put these here to compile
    let color_hm: HashMap<String, nu_ansi_term::Style> = HashMap::new();
    // get the default config
    let config = Config::default();
    // Capture the table as a string
    let output_table = draw_table(&table, width, &color_hm, &config);
    // Draw the table
    println!("{}", output_table)
}

fn make_table_data() -> (Vec<&'static str>, Vec<&'static str>) {
    let table_headers = vec![
        "category",
        "description",
        "emoji",
        "ios_version",
        "unicode_version",
        "aliases",
        "tags",
        "category2",
        "description2",
        "emoji2",
        "ios_version2",
        "unicode_version2",
        "aliases2",
        "tags2",
    ];

    let row_data = vec![
        "Smileys & Emotion",
        "grinning face",
        "😀",
        "6",
        "6.1",
        "grinning",
        "smile",
        "Smileys & Emotion",
        "grinning face",
        "😀",
        "6",
        "6.1",
        "grinning",
        "smile",
    ];

    (table_headers, row_data)
}

fn vec_of_str_to_vec_of_styledstr(data: &[&str], is_header: bool) -> Vec<StyledString> {
    let mut v = vec![];

    for x in data {
        if is_header {
            v.push(StyledString::new(
                String::from(*x),
                TextStyle::default_header(),
            ))
        } else {
            v.push(StyledString::new(String::from(*x), TextStyle::basic_left()))
        }
    }
    v
}
