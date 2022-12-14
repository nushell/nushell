use std::{collections::HashMap, usize};

use nu_protocol::{Config, TrimStrategy};
use nu_table::{string_width, Alignments, Table, TableTheme as theme, TextStyle};
use tabled::papergrid::records::{cell_info::CellInfo, tcell::TCell};

#[test]
fn data_and_header_has_different_size() {
    let table = Table::new(
        vec![row(3), row(5), row(5)],
        (3, 5),
        usize::MAX,
        true,
        false,
        &theme::heavy(),
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
        &theme::heavy(),
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
        let table = Table::new(
            vec![row(3), row(3), row(5)],
            (3, 5),
            i,
            true,
            false,
            &theme::heavy(),
        );
        assert!(draw_table(table, i, &cfg).is_none());
    }

    let table = Table::new(
        vec![row(3), row(3), row(5)],
        (3, 5),
        11,
        true,
        false,
        &theme::heavy(),
    );
    assert!(draw_table(table, 11, &cfg).is_some());

    let cfg = Config {
        trim_strategy: TrimStrategy::Truncate { suffix: None },
        ..Default::default()
    };

    for i in 0..10 {
        let table = Table::new(
            vec![row(3), row(3), row(5)],
            (3, 5),
            i,
            true,
            false,
            &theme::heavy(),
        );
        assert!(draw_table(table, i, &cfg).is_none());
    }

    let table = Table::new(
        vec![row(3), row(3), row(5)],
        (3, 5),
        11,
        true,
        false,
        &theme::heavy(),
    );
    assert!(draw_table(table, 11, &cfg).is_some());
}

#[test]
fn wrap_test() {
    let tests = [
        (0, None),
        (1, None),
        (2, None),
        (3, None),
        (4, None),
        (5, None),
        (6, None),
        (7, None),
        (8, None),
        (9, None),
        (10, None),
        (11, None),
        (12, Some("┏━━━━┳━━━━━┓\n┃ 12 ┃ ... ┃\n┃ 3  ┃     ┃\n┃ 45 ┃     ┃\n┃ 67 ┃     ┃\n┃ 8  ┃     ┃\n┣━━━━╋━━━━━┫\n┃ 0  ┃ ... ┃\n┃ 0  ┃ ... ┃\n┗━━━━┻━━━━━┛")),
        (13, Some("┏━━━━━┳━━━━━┓\n┃ 123 ┃ ... ┃\n┃  45 ┃     ┃\n┃ 678 ┃     ┃\n┣━━━━━╋━━━━━┫\n┃ 0   ┃ ... ┃\n┃ 0   ┃ ... ┃\n┗━━━━━┻━━━━━┛")),
        (21, Some("┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┃ 4567 ┃ w eq ┃     ┃\n┃  8   ┃  we  ┃     ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛")),
        (29, Some("┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123 4567 ┃ qweqw eq ┃ ... ┃\n┃    8     ┃    we    ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛")),
        (49, Some("┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n┃           ┃            ┃     x xx xx    ┃     ┃\n┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛")),
    ];

    let trim = TrimStrategy::wrap(false);
    test_config(&tests, trim);
}

#[test]
fn wrap_keep_words_test() {
    let tests = [
        (0, None),
        (1, None),
        (2, None),
        (3, None),
        (4, None),
        (5, None),
        (6, None),
        (7, None),
        (8, None),
        (9, None),
        (10, None),
        (11, None),
        (12, Some("┏━━━━┳━━━━━┓\n┃ 12 ┃ ... ┃\n┃ 3  ┃     ┃\n┃ 45 ┃     ┃\n┃ 67 ┃     ┃\n┃ 8  ┃     ┃\n┣━━━━╋━━━━━┫\n┃ 0  ┃ ... ┃\n┃ 0  ┃ ... ┃\n┗━━━━┻━━━━━┛")),
        (13, Some("┏━━━━━┳━━━━━┓\n┃ 123 ┃ ... ┃\n┃     ┃     ┃\n┃ 456 ┃     ┃\n┃ 78  ┃     ┃\n┣━━━━━╋━━━━━┫\n┃ 0   ┃ ... ┃\n┃ 0   ┃ ... ┃\n┗━━━━━┻━━━━━┛")),
        (21, Some("┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┃ 4567 ┃  w   ┃     ┃\n┃ 8    ┃ eqwe ┃     ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛")),
        (29, Some("┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃   123    ┃  qweqw   ┃ ... ┃\n┃ 45678    ┃ eqwe     ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛")),
        (49, Some("┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n┃           ┃            ┃  x xx xx       ┃     ┃\n┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛")),
    ];

    let trim = TrimStrategy::wrap(true);
    test_config(&tests, trim);
}

#[test]
fn truncate_test() {
    let tests = [
        (0, None),
        (1, None),
        (2, None),
        (3, None),
        (4, None),
        (5, None),
        (6, None),
        (7, None),
        (8, None),
        (9, None),
        (10, None),
        (11, None),
        (12, Some("┏━━━━┳━━━━━┓\n┃ 12 ┃ ... ┃\n┣━━━━╋━━━━━┫\n┃ 0  ┃ ... ┃\n┃ 0  ┃ ... ┃\n┗━━━━┻━━━━━┛")),
        (13, Some("┏━━━━━┳━━━━━┓\n┃ 123 ┃ ... ┃\n┣━━━━━╋━━━━━┫\n┃ 0   ┃ ... ┃\n┃ 0   ┃ ... ┃\n┗━━━━━┻━━━━━┛")),
        (21, Some("┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛")),
        (29, Some("┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123 4567 ┃ qweqw eq ┃ ... ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛")),
        (49, Some("┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛")),
    ];

    let trim = TrimStrategy::truncate(None);
    test_config(&tests, trim);
}

#[test]
fn truncate_with_suffix_test() {
    let tests = [
        (0, None),
        (1, None),
        (2, None),
        (3, None),
        (4, None),
        (5, None),
        (6, None),
        (7, None),
        (8, None),
        (9, None),
        (10, None),
        (11, None),
        (12, Some("┏━━━━┳━━━━━┓\n┃ .. ┃ ... ┃\n┣━━━━╋━━━━━┫\n┃ 0  ┃ ... ┃\n┃ 0  ┃ ... ┃\n┗━━━━┻━━━━━┛")),
        (13, Some("┏━━━━━┳━━━━━┓\n┃ ... ┃ ... ┃\n┣━━━━━╋━━━━━┫\n┃ 0   ┃ ... ┃\n┃ 0   ┃ ... ┃\n┗━━━━━┻━━━━━┛")),
        (21, Some("┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 1... ┃ q... ┃ ... ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛")),
        (29, Some("┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123 4... ┃ qweqw... ┃ ... ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛")),
        (49, Some("┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x... ┃ ... ┃\n┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛")),
    ];

    let trim = TrimStrategy::truncate(Some(String::from("...")));
    test_config(&tests, trim);
}

fn test_config(tests: &[(usize, Option<&str>)], trim: TrimStrategy) {
    let config = Config {
        trim_strategy: trim,
        ..Default::default()
    };

    for (i, &(termwidth, expected)) in tests.iter().enumerate() {
        let table = table_with_data(termwidth);
        let actual = draw_table(table, termwidth, &config);

        assert_eq!(
            actual.as_deref(),
            expected,
            "\nfail i={:?} width={}",
            i,
            termwidth
        );

        if let Some(table) = actual {
            assert!(string_width(&table) <= termwidth);
        }
    }
}

fn draw_table(table: Table, limit: usize, cfg: &Config) -> Option<String> {
    let styles = HashMap::default();
    let alignments = Alignments::default();
    table.draw_table(cfg, &styles, alignments, &theme::heavy(), limit, false)
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

    Table::new(data, (3, 5), termwidth, true, false, &theme::heavy())
}
