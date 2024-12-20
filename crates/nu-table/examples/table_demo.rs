use nu_ansi_term::{Color, Style};
use nu_color_config::TextStyle;
use nu_table::{NuTable, TableTheme};
use tabled::grid::records::vec_records::Text;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut width = 0;

    if args.len() > 1 {
        width = args[1].parse::<usize>().expect("Need a width in columns");
    }

    if width < 4 {
        println!("Width must be greater than or equal to 4, setting width to 80");
        width = 80;
    }

    let (table_headers, row_data) = make_table_data();

    let headers = to_cell_info_vec(&table_headers);
    let rows = to_cell_info_vec(&row_data);

    let mut rows = vec![rows; 3];
    rows.insert(0, headers);

    let mut table = NuTable::from(rows);

    table.set_data_style(TextStyle::basic_left());
    table.set_header_style(TextStyle::basic_center().style(Style::new().on(Color::Blue)));
    table.set_theme(TableTheme::rounded());
    table.set_structure(false, true, false);

    let output_table = table
        .draw(width)
        .unwrap_or_else(|| format!("Couldn't fit table into {width} columns!"));

    println!("{output_table}")
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

fn to_cell_info_vec(data: &[&str]) -> Vec<Text<String>> {
    let mut v = vec![];
    for x in data {
        v.push(Text::new(String::from(*x)));
    }

    v
}
