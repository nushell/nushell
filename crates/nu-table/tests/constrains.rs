use std::{collections::HashMap, usize};

use nu_protocol::{Config, TrimStrategy};
use nu_table::{Alignments, Table, TableTheme as theme, TextStyle};
use tabled::papergrid::records::{cell_info::CellInfo, tcell::TCell};

#[test]
fn data_and_header_has_different_size() {
    let table = Table::new(
        vec![row(3), row(5), row(5)],
        (3, 5),
        usize::MAX,
        true,
        false,
    );

    let table = draw_table(table, usize::MAX, &Config::default());

    let expected = "┏━━━┳━━━┳━━━┳━━━┳━━━┓\n\
                         ┃ 0 ┃ 1 ┃ 2 ┃   ┃   ┃\n\
                         ┣━━━╋━━━╋━━━╋━━━╋━━━┫\n\
                         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
                         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
                         ┗━━━┻━━━┻━━━┻━━━┻━━━┛";

    assert_eq!(table.as_deref(), Some(expected));

    let table = Table::new(
        vec![row(5), row(3), row(3)],
        (3, 5),
        usize::MAX,
        true,
        false,
    );
    let table = draw_table(table, usize::MAX, &Config::default());

    let expected = "┏━━━┳━━━┳━━━┳━━━┳━━━┓\n\
                         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
                         ┣━━━╋━━━╋━━━╋━━━╋━━━┫\n\
                         ┃ 0 ┃ 1 ┃ 2 ┃   ┃   ┃\n\
                         ┃ 0 ┃ 1 ┃ 2 ┃   ┃   ┃\n\
                         ┗━━━┻━━━┻━━━┻━━━┻━━━┛";

    assert_eq!(table.as_deref(), Some(expected));
}

#[test]
fn termwidth_too_small() {
    let cfg = Config::default();
    for i in 0..10 {
        let table = Table::new(vec![row(3), row(3), row(5)], (3, 5), i, true, false);
        assert!(draw_table(table, i, &cfg).is_none());
    }

    let table = Table::new(vec![row(3), row(3), row(5)], (3, 5), 11, true, false);
    assert!(draw_table(table, 11, &cfg).is_some());

    let cfg = Config {
        trim_strategy: TrimStrategy::Truncate { suffix: None },
        ..Default::default()
    };

    for i in 0..10 {
        let table = Table::new(vec![row(3), row(3), row(5)], (3, 5), i, true, false);
        assert!(draw_table(table, i, &cfg).is_none());
    }

    let table = Table::new(vec![row(3), row(3), row(5)], (3, 5), 11, true, false);
    assert!(draw_table(table, 11, &cfg).is_some());
}

#[test]
fn wrap_test() {
    let cfg = Config {
        trim_strategy: TrimStrategy::Wrap {
            try_to_keep_words: false,
        },
        ..Default::default()
    };

    for i in 0..10 {
        assert!(draw_table(table_with_data(i), i, &cfg).is_none());
    }

    assert_eq!(draw_table(table_with_data(10), 10, &cfg).unwrap(), "┏━━━━┳━━━┓\n┃ 12 ┃ . ┃\n┃ 3  ┃ . ┃\n┃ 45 ┃ . ┃\n┃ 67 ┃   ┃\n┃ 8  ┃   ┃\n┣━━━━╋━━━┫\n┃ 0  ┃ . ┃\n┃    ┃ . ┃\n┃    ┃ . ┃\n┃ 0  ┃ . ┃\n┃    ┃ . ┃\n┃    ┃ . ┃\n┗━━━━┻━━━┛");
    assert_eq!(
        draw_table(table_with_data(21), 21, &cfg).unwrap(),
        "┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┃ 4567 ┃ w eq ┃     ┃\n┃  8   ┃  we  ┃     ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(29), 29, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123 4567 ┃ qweqw eq ┃ ... ┃\n┃    8     ┃    we    ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(49), 49, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n┃ 123 4567 ┃ qweqw eq ┃ xxx xx  ┃ qqq qqq ┃ ... ┃\n┃    8     ┃    we    ┃ xx x xx ┃  qqqq q ┃     ┃\n┃          ┃          ┃  x xx x ┃  qq qq  ┃     ┃\n┃          ┃          ┃    x    ┃         ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━━━━━┻━━━━━━━━━┻━━━━━┛"
    );
}

#[test]
fn wrap_keep_words_test() {
    let cfg = Config {
        trim_strategy: TrimStrategy::Wrap {
            try_to_keep_words: true,
        },
        ..Default::default()
    };

    for i in 0..10 {
        assert!(draw_table(table_with_data(i), i, &cfg).is_none());
    }

    assert_eq!(draw_table(table_with_data(10), 10, &cfg).unwrap(), "┏━━━━┳━━━┓\n┃ 12 ┃ . ┃\n┃ 3  ┃ . ┃\n┃ 45 ┃ . ┃\n┃ 67 ┃   ┃\n┃ 8  ┃   ┃\n┣━━━━╋━━━┫\n┃ 0  ┃ . ┃\n┃    ┃ . ┃\n┃    ┃ . ┃\n┃ 0  ┃ . ┃\n┃    ┃ . ┃\n┃    ┃ . ┃\n┗━━━━┻━━━┛");
    assert_eq!(
        draw_table(table_with_data(21), 21, &cfg).unwrap(),
        "┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┃ 4567 ┃  w   ┃     ┃\n┃ 8    ┃ eqwe ┃     ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(29), 29, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃   123    ┃  qweqw   ┃ ... ┃\n┃ 45678    ┃ eqwe     ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(49), 49, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n┃   123    ┃  qweqw   ┃ xxx xx  ┃ qqq qqq ┃ ... ┃\n┃ 45678    ┃ eqwe     ┃ xx x xx ┃  qqqq   ┃     ┃\n┃          ┃          ┃  x xx   ┃ qqq qq  ┃     ┃\n┃          ┃          ┃ xx      ┃         ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━━━━━┻━━━━━━━━━┻━━━━━┛"
    );
}

#[test]
fn truncate_test() {
    let cfg = Config {
        trim_strategy: TrimStrategy::Truncate { suffix: None },
        ..Default::default()
    };

    for i in 0..10 {
        assert!(draw_table(table_with_data(i), i, &cfg).is_none());
    }

    assert_eq!(
        draw_table(table_with_data(10), 10, &cfg).unwrap(),
        "┏━━━━┳━━━┓\n┃ 12 ┃ . ┃\n┣━━━━╋━━━┫\n┃ 0  ┃ . ┃\n┃ 0  ┃ . ┃\n┗━━━━┻━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(21), 21, &cfg).unwrap(),
        "┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(29), 29, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123 4567 ┃ qweqw eq ┃ ... ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(49), 49, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n┃ 123 4567 ┃ qweqw eq ┃ xxx xx  ┃ qqq qqq ┃ ... ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━━━━━┻━━━━━━━━━┻━━━━━┛"
    );
}

#[test]
fn truncate_with_suffix_test() {
    let cfg = Config {
        trim_strategy: TrimStrategy::Truncate {
            suffix: Some(String::from("...")),
        },
        ..Default::default()
    };

    for i in 0..10 {
        assert!(draw_table(table_with_data(i), i, &cfg).is_none());
    }

    assert_eq!(
        draw_table(table_with_data(10), 10, &cfg).unwrap(),
        "┏━━━━┳━━━┓\n┃ .. ┃ . ┃\n┣━━━━╋━━━┫\n┃ 0  ┃ . ┃\n┃ 0  ┃ . ┃\n┗━━━━┻━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(21), 21, &cfg).unwrap(),
        "┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 1... ┃ q... ┃ ... ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(29), 29, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123 4... ┃ qweqw... ┃ ... ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛"
    );
    assert_eq!(
        draw_table(table_with_data(49), 49, &cfg).unwrap(),
        "┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n┃ 123 4... ┃ qweqw... ┃ xxx ... ┃ qqq ... ┃ ... ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┃ 0        ┃ 1        ┃ 2       ┃ 3       ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━━━━━┻━━━━━━━━━┻━━━━━┛"
    );
}

fn draw_table(table: Table, limit: usize, cfg: &Config) -> Option<String> {
    let styles = HashMap::default();
    let alignments = Alignments::default();
    table.draw_table(cfg, &styles, alignments, &theme::heavy(), limit)
}

fn row(count_columns: usize) -> Vec<TCell<CellInfo<'static>, TextStyle>> {
    let mut row = Vec::with_capacity(count_columns);

    for i in 0..count_columns {
        row.push(Table::create_cell(i.to_string(), TextStyle::default()));
    }

    row
}

fn styled_str(s: &str) -> TCell<CellInfo<'static>, TextStyle> {
    Table::create_cell(s.to_string(), TextStyle::default())
}

fn table_with_data(termwidth: usize) -> Table {
    let header = vec![
        styled_str("123 45678"),
        styled_str("qweqw eqwe"),
        styled_str("xxx xx xx x xx x xx xx"),
        styled_str("qqq qqq qqqq qqq qq"),
        styled_str("qw"),
    ];
    let data = vec![header, row(5), row(5)];

    Table::new(data, (3, 5), termwidth, true, false)
}
