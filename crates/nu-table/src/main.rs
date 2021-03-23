use nu_table::{draw_table, StyledString, Table, TextStyle, Theme};
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
    let table = Table::new(headers, vec![rows; 3], Theme::rounded());

    // FIXME: Config isn't available from here so just put these here to compile
    let color_hm: HashMap<String, nu_ansi_term::Style> = HashMap::new();
    // Draw the table
    draw_table(&table, width, &color_hm);
}

fn make_table_data() -> (Vec<&'static str>, Vec<&'static str>) {
    let mut table_headers = vec![];
    table_headers.push("category");
    table_headers.push("description");
    table_headers.push("emoji");
    table_headers.push("ios_version");
    table_headers.push("unicode_version");
    table_headers.push("aliases");
    table_headers.push("tags");
    table_headers.push("category2");
    table_headers.push("description2");
    table_headers.push("emoji2");
    table_headers.push("ios_version2");
    table_headers.push("unicode_version2");
    table_headers.push("aliases2");
    table_headers.push("tags2");

    let mut row_data = vec![];
    row_data.push("Smileys & Emotion");
    row_data.push("grinning face");
    row_data.push("😀");
    row_data.push("6");
    row_data.push("6.1");
    row_data.push("grinning");
    row_data.push("smile");
    row_data.push("Smileys & Emotion");
    row_data.push("grinning face");
    row_data.push("😀");
    row_data.push("6");
    row_data.push("6.1");
    row_data.push("grinning");
    row_data.push("smile");

    (table_headers, row_data)
}

fn vec_of_str_to_vec_of_styledstr(data: &[&str], is_header: bool) -> Vec<StyledString> {
    let mut v = vec![];

    for x in data.iter() {
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
