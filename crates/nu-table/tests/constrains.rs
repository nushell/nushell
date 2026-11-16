mod common;

use nu_protocol::TrimStrategy;
use nu_table::{NuTable, TableTheme as theme, string_width};

use common::{TestCase, bordered_width, cell, create_row, create_table, split_cells, test_table};

use tabled::grid::records::vec_records::Text;

#[test]
fn data_and_header_has_different_size_doesnt_work() {
    let mut table = NuTable::from(vec![create_row(5), create_row(5), create_row(5)]);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);

    let table = table.draw(usize::MAX);

    assert_eq!(
        table.as_deref(),
        Some(
            "┏━━━┳━━━┳━━━┳━━━┳━━━┓\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┣━━━╋━━━╋━━━╋━━━╋━━━┫\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┗━━━┻━━━┻━━━┻━━━┻━━━┛"
        )
    );

    let mut table = NuTable::from(vec![create_row(5), create_row(5), create_row(5)]);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);

    let table = table.draw(usize::MAX);

    assert_eq!(
        table.as_deref(),
        Some(
            "┏━━━┳━━━┳━━━┳━━━┳━━━┓\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┣━━━╋━━━╋━━━╋━━━╋━━━┫\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┗━━━┻━━━┻━━━┻━━━┻━━━┛"
        )
    );
}

#[test]
fn termwidth_too_small() {
    let tests = [
        TrimStrategy::truncate(None),
        TrimStrategy::truncate(Some(String::from("**"))),
        TrimStrategy::truncate(Some(String::from(""))),
        TrimStrategy::wrap(false),
        TrimStrategy::wrap(true),
    ];

    let data = vec![create_row(5), create_row(5), create_row(5)];

    for case in tests {
        for i in 0..10 {
            let mut table = NuTable::from(data.clone());
            table.set_theme(theme::heavy());
            table.set_structure(false, true, false);
            table.set_trim(case.clone());

            let table = table.draw(i);

            assert!(table.is_none());
        }
    }
}

#[test]
fn termwidth_too_small_single_column() {
    let tests = [
        TrimStrategy::truncate(None),
        TrimStrategy::truncate(Some(String::from("**"))),
        TrimStrategy::wrap(false),
        TrimStrategy::wrap(true),
    ];

    let data = vec![
        vec![cell("abcdefghijklmnop")],
        vec![cell("qrstuvwxyzabcdef")],
        vec![cell("1234567890123456")],
    ];

    for case in tests {
        // the table cannot fit anything but the borders below width 8,
        // so it must report that it cannot be drawn rather than panic
        for i in 0..8 {
            let mut table = NuTable::from(data.clone());
            table.set_theme(theme::heavy());
            table.set_structure(false, true, false);
            table.set_trim(case.clone());

            assert!(table.draw(i).is_none(), "termwidth={i}");
        }

        let mut table = NuTable::from(data.clone());
        table.set_theme(theme::heavy());
        table.set_structure(false, true, false);
        table.set_trim(case.clone());

        assert!(table.draw(8).is_some());
    }
}

#[test]
fn wrap_test() {
    for test in 0..14 {
        test_trim(&[(test, None)], TrimStrategy::wrap(false));
    }

    let tests = [
        (
            15,
            Some(
                "┏━━━━━━━┳━━━━━┓\n\
                 ┃ 123 4 ┃ ... ┃\n\
                 ┃ 5678  ┃     ┃\n\
                 ┣━━━━━━━╋━━━━━┫\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┗━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            21,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            29,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            49,
            Some(
                "┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n\
                 ┃           ┃            ┃  x xx xx       ┃     ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
    ];

    test_trim(&tests, TrimStrategy::wrap(false));
    let single_tests = [
        (
            15,
            Some(
                "┏━━━━━━━━━━━━━┓\n\
                 ┃ 0 2 4 6 8 0 ┃\n\
                 ┃  2 4 6 8 0  ┃\n\
                 ┗━━━━━━━━━━━━━┛",
            ),
        ),
        (
            21,
            Some(
                "┏━━━━━━━━━━━━━━━━━━━┓\n\
                 ┃ 0 2 4 6 8 0 2 4 6 ┃\n\
                 ┃  8 0              ┃\n\
                 ┗━━━━━━━━━━━━━━━━━━━┛",
            ),
        ),
        (
            40,
            Some(
                "┏━━━━━━━━━━━━━━━━━━━━━━━┓\n\
                 ┃ 0 2 4 6 8 0 2 4 6 8 0 ┃\n\
                 ┗━━━━━━━━━━━━━━━━━━━━━━━┛",
            ),
        ),
    ];
    test_trim_single(&single_tests, TrimStrategy::wrap(false));
}

#[test]
fn wrap_keep_words_test() {
    for test in 0..14 {
        test_trim(&[(test, None)], TrimStrategy::wrap(true));
    }

    let tests = [
        (
            15,
            Some(
                "┏━━━━━━━┳━━━━━┓\n\
                 ┃ 123   ┃ ... ┃\n\
                 ┃ 45678 ┃     ┃\n\
                 ┣━━━━━━━╋━━━━━┫\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┗━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            15,
            Some(
                "┏━━━━━━━┳━━━━━┓\n\
                 ┃ 123   ┃ ... ┃\n\
                 ┃ 45678 ┃     ┃\n\
                 ┣━━━━━━━╋━━━━━┫\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┗━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            21,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            29,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            49,
            Some(
                "┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n\
                 ┃           ┃            ┃  x xx xx       ┃     ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
    ];

    test_trim(&tests, TrimStrategy::wrap(true));
}

#[test]
fn truncate_test() {
    for test in 0..14 {
        test_trim(&[(test, None)], TrimStrategy::wrap(true));
    }

    let tests = [
        (
            15,
            Some(
                "┏━━━━━━━┳━━━━━┓\n\
                 ┃ 123 4 ┃ ... ┃\n\
                 ┣━━━━━━━╋━━━━━┫\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┗━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            21,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            29,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            49,
            Some(
                "┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
    ];

    test_trim(&tests, TrimStrategy::truncate(None));
}

#[test]
fn truncate_with_suffix_test() {
    for test in 0..14 {
        test_trim(&[(test, None)], TrimStrategy::wrap(true));
    }

    let tests = [
        (
            15,
            Some(
                "┏━━━━━━━┳━━━━━┓\n\
                 ┃ 12... ┃ ... ┃\n\
                 ┣━━━━━━━╋━━━━━┫\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┃ 0     ┃ ... ┃\n\
                 ┗━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            21,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            29,
            Some(
                "┏━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┃ 0         ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
        (
            49,
            Some(
                "┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x... ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛",
            ),
        ),
    ];

    test_trim(&tests, TrimStrategy::truncate(Some(String::from("..."))));
}

#[test]
fn width_control_test_0() {
    let data = vec![
        vec![
            common::cell(
                "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
            );
            16
        ],
        vec![
            common::cell(
                "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"
            );
            16
        ],
        vec![
            common::cell(
                "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
            );
            16
        ],
    ];

    let tests = [
        (
            20,
            "┏━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            119,
            "┏━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            120,
            "┏━━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            121,
            "┏━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxxx... ┃ xxxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyyy... ┃ yyyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzzz... ┃ zzzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            150,
            "┏━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            usize::MAX,
            "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓\n\
             ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃\n\
             ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫\n\
             ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃\n\
             ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃\n\
             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
        ),
    ];

    test_width(data, &tests);
}

#[test]
fn width_priority_columns_are_widened_first() {
    let headers = vec![
        cell("c0"),
        cell("c1"),
        cell("c2"),
        cell("c3"),
        cell("c4"),
        cell("c5"),
        cell("c6"),
        cell("c7"),
    ];

    let row = vec![
        cell("value_0000000000000000"),
        cell("value_1111111111111111"),
        cell("value_2222222222222222"),
        cell("value_3333333333333333"),
        cell("value_4444444444444444"),
        cell("value_5555555555555555"),
        cell("priority_value_12345"),
        cell("value_7777777777777777"),
    ];

    let mut without_priority = NuTable::new(3, 8);
    without_priority.set_theme(theme::heavy());
    without_priority.set_structure(false, true, false);
    without_priority.set_row(0, headers.clone());
    without_priority.set_row(1, row.clone());
    without_priority.set_row(2, row.clone());

    let without_priority = without_priority.draw(121).expect("table renders");

    let mut with_priority = NuTable::new(3, 8);
    with_priority.set_theme(theme::heavy());
    with_priority.set_structure(false, true, false);
    with_priority.set_width_priority_columns(&[6]);
    with_priority.set_row(0, headers);
    with_priority.set_row(1, row.clone());
    with_priority.set_row(2, row);

    let with_priority = with_priority.draw(121).expect("table renders");

    assert!(!without_priority.contains("priority_value_12345"));
    assert!(with_priority.contains("priority_value_12345"));
}

#[test]
fn left_side_priority_keeps_context_columns() {
    let headers = vec![
        cell("c0"),
        cell("c1"),
        cell("c2"),
        cell("c3"),
        cell("c4"),
        cell("c5"),
        cell("c6"),
        cell("c7"),
    ];

    let row = vec![
        cell("value_0000000000000000"),
        cell("value_1111111111111111"),
        cell("value_2222222222222222"),
        cell("value_3333333333333333"),
        cell("value_4444444444444444"),
        cell("value_5555555555555555"),
        cell("value_6666666666666666"),
        cell("value_7777777777777777"),
    ];

    let mut table = NuTable::new(3, 8);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);
    table.set_width_priority_columns(&[1, 6]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(121).expect("table renders");

    assert!(rendered.contains("c2"));
}

#[test]
fn priority_column_keeps_a_third_of_the_width() {
    let headers = vec![
        cell("c0"),
        cell("c1"),
        cell("c2"),
        cell("c3"),
        cell("c4"),
        cell("c5"),
        cell("c6"),
        cell("c7"),
    ];

    let row = vec![
        cell("v0_0000000000000000"),
        cell("v1_1111111111111111"),
        cell("secondary_priority_value_abcdefghijklmnopqrstuvwxyz"),
        cell("v3_3333333333333333"),
        cell("v4_4444444444444444"),
        cell("v5_5555555555555555"),
        cell("primary_priority_value_abcdefghijklmnopqrstuvwxyz0123456789"),
        cell("v7_7777777777777777"),
    ];

    let mut table = NuTable::new(3, 8);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);
    table.set_width_priority_columns(&[2]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(121).expect("table renders");
    let widths = parse_heavy_header_column_widths(&rendered);

    let header_cells = parse_heavy_header_cells(&rendered);

    // All 8 columns fit at their floors, but then c2 would get less than a
    // third of the width: the columns to its right hide until it does, and
    // every other visible column sits at its floor (10 characters plus padding).
    assert!(widths[2] >= 121 / 3, "{rendered}");
    assert_eq!(header_cells.last().map(String::as_str), Some("..."));
    assert!(!header_cells.iter().any(|cell| cell == "c6"), "{rendered}");
    for (col, &width) in widths[..widths.len() - 1].iter().enumerate() {
        if col != 2 {
            assert_eq!(width, 12, "column {col} is not at its floor:\n{rendered}");
        }
    }
}

#[test]
fn priority_marking_never_hides_the_column() {
    let headers: Vec<Text<String>> = (0..8).map(|i| cell(&format!("c{i}"))).collect();
    let row: Vec<Text<String>> = (0..8)
        .map(|i| cell(&format!("value_{i}_abcdefghijklmnopqrstuvwxyz0123456789")))
        .collect();
    let table = |priorities: &[usize]| {
        let mut table = NuTable::new(3, 8);
        table.set_theme(theme::heavy());
        table.set_structure(false, true, false);
        table.set_width_priority_columns(priorities);
        table.set_row(0, headers.clone());
        table.set_row(1, row.clone());
        table.set_row(2, row.clone());
        table.draw(121).expect("table renders")
    };

    // Unmarked, c6 shows. Marked with c2, both want a third of the width, which
    // they cannot both have; hiding c6 would make room, but marking never hides.
    let unmarked = parse_heavy_header_cells(&table(&[]));
    assert!(unmarked.iter().any(|cell| cell == "c6"), "{unmarked:?}");

    let rendered = table(&[6, 2]);
    let header_cells = parse_heavy_header_cells(&rendered);
    let widths = parse_heavy_header_column_widths(&rendered);
    assert!(header_cells.iter().any(|cell| cell == "c6"), "{rendered}");
    assert!(header_cells.iter().any(|cell| cell == "c2"), "{rendered}");
    // The priority columns share what the others leave at their floors.
    for (col, &width) in widths[..widths.len() - 1].iter().enumerate() {
        if col == 2 || col == 6 {
            assert!(
                width > 12,
                "priority column {col} is at its floor:\n{rendered}"
            );
        } else {
            assert_eq!(width, 12, "column {col} is not at its floor:\n{rendered}");
        }
    }
}

#[test]
fn secondary_priority_changes_width_distribution() {
    let headers: Vec<Text<String>> = (0..12).map(|i| cell(&format!("c{i}"))).collect();

    let row: Vec<Text<String>> = (0..12)
        .map(|i| cell(&format!("value_{i}_abcdefghijklmnopqrstuvwxyz0123456789")))
        .collect();

    let mut only_primary = NuTable::new(3, 12);
    only_primary.set_theme(theme::heavy());
    only_primary.set_structure(false, true, false);
    only_primary.set_width_priority_columns(&[7]);
    only_primary.set_row(0, headers.clone());
    only_primary.set_row(1, row.clone());
    only_primary.set_row(2, row.clone());

    let primary_rendered = only_primary.draw(121).expect("table renders");
    let primary_widths = parse_heavy_header_column_widths(&primary_rendered);

    let mut with_secondary = NuTable::new(3, 12);
    with_secondary.set_theme(theme::heavy());
    with_secondary.set_structure(false, true, false);
    with_secondary.set_width_priority_columns(&[7, 2]);
    with_secondary.set_row(0, headers);
    with_secondary.set_row(1, row.clone());
    with_secondary.set_row(2, row);

    let secondary_rendered = with_secondary.draw(121).expect("table renders");
    let secondary_widths = parse_heavy_header_column_widths(&secondary_rendered);

    assert!(secondary_rendered.contains("..."));
    assert!(
        secondary_widths[2] > primary_widths[2],
        "expected secondary priority column width to grow; primary-only: {primary_widths:?}, with-secondary: {secondary_widths:?}"
    );
}

#[test]
fn left_priority_uses_full_width_when_constrained() {
    let headers: Vec<Text<String>> = (0..12).map(|i| cell(&format!("c{i}"))).collect();

    let row: Vec<Text<String>> = (0..12)
        .map(|i| cell(&format!("value_{i}_abcdefghijklmnopqrstuvwxyz0123456789")))
        .collect();

    let mut table = NuTable::new(3, 12);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);
    table.set_width_priority_columns(&[2]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(121).expect("table renders");
    let widths = parse_heavy_header_column_widths(&rendered);
    let rendered_width = bordered_width(&widths);

    assert_eq!(rendered_width, 121);
}

#[test]
fn priority_column_past_the_cut_is_hidden() {
    let headers: Vec<Text<String>> = (0..14).map(|i| cell(&format!("c{i}"))).collect();

    let row: Vec<Text<String>> = (0..14)
        .map(|i| cell(&format!("value_{i}_abcdefghijklmnopqrstuvwxyz0123456789")))
        .collect();

    let mut table = NuTable::new(3, 14);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);
    // c12 is past the columns that fit at their floors at this width.
    table.set_width_priority_columns(&[12, 3]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(121).expect("table renders");
    let header_cells = parse_heavy_header_cells(&rendered);

    let widths = parse_heavy_header_column_widths(&rendered);

    // c3 keeps a third of the width. Columns are hidden from the right, and
    // c12 is past the cut, so it is hidden even though it is a priority.
    assert!(widths[3] >= 121 / 3, "{rendered}");
    assert!(
        !header_cells.iter().any(|cell| cell == "c12"),
        "expected c12 to be hidden; header cells: {header_cells:?}\n{rendered}"
    );
}

#[test]
fn priority_column_takes_the_room_other_columns_give_up() {
    let headers: Vec<Text<String>> = (0..13).map(|i| cell(&format!("c{i}"))).collect();

    let priority_value =
        "very_very_very_long_filename_that_should_get_priority_and_avoid_wrapping.txt";

    let row = vec![
        cell("value_0"),
        cell(priority_value),
        cell("file"),
        cell(""),
        cell("false"),
        cell("rw-r--r--"),
        cell("1"),
        cell("12345"),
        cell("me"),
        cell("staff"),
        cell("1234"),
        cell("2 years ago"),
        cell("2 years ago"),
    ];

    let mut table = NuTable::new(3, 13);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);
    table.set_width_priority_columns(&[1]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(140).expect("table renders");
    let header_cells = parse_heavy_header_cells(&rendered);
    let widths = parse_heavy_header_column_widths(&rendered);

    // The priority column keeps at least a third of the width: the last
    // columns are hidden for it, and the others give up all they can (at most
    // 10 characters plus padding stay).
    assert!(widths[1] >= 140 / 3, "{rendered}");
    assert_eq!(header_cells.last().map(String::as_str), Some("..."));
    assert!(
        widths
            .iter()
            .enumerate()
            .all(|(col, &width)| col == 1 || width <= 12)
    );
}

#[test]
fn unbounded_width_keeps_natural_widths() {
    // `explore` draws its tables with `usize::MAX` as the width.
    let data = vec![create_row(3); 2];
    let unbounded = create_table(data.clone(), TestCase::new(usize::MAX).header());
    let fitting = create_table(data, TestCase::new(100).header());
    assert_eq!(unbounded, fitting);
}

#[test]
fn trailing_column_fits_when_wider_than_the_column_it_replaces() {
    // The `...` column (3 + padding) takes the place of the last fitted
    // column. One-letter columns are narrower than that, which used to make
    // the table one or two columns wider than the terminal.
    let header = ["#", "name", "a", "description", "b", "notes", "c", "extra"];
    let row = [
        "0",
        "a long name",
        "x",
        "a longer description",
        "y",
        "notes",
        "z",
        "more",
    ];
    let data = vec![header.map(cell).to_vec(), row.map(cell).to_vec()];

    for hos in [false, true] {
        for width in 10..=80 {
            let mut table = NuTable::from(data.clone());
            table.set_theme(theme::rounded());
            table.set_structure(true, true, false);
            table.set_trim(TrimStrategy::wrap(true));
            table.set_border_header(hos);
            let Some(rendered) = table.draw(width) else {
                continue;
            };

            assert!(
                rendered.lines().all(|line| string_width(line) <= width),
                "wider than {width} (header_on_separator: {hos}):\n{rendered}"
            );
        }
    }
}

fn parse_heavy_header_column_widths(rendered: &str) -> Vec<usize> {
    let header_line = rendered
        .lines()
        .nth(1)
        .expect("table header line is present");

    split_cells(header_line, '┃')
        .into_iter()
        .map(string_width)
        .collect()
}

#[test]
fn ls_like_table_at_146_does_not_make_one_char_size_column() {
    let long_name = "grok-continue-115-caching-regression.txt";
    let header = vec![
        cell("#"),
        cell("name"),
        cell("type"),
        cell("target"),
        cell("readonly"),
        cell("mode"),
        cell("num_links"),
        cell("inode"),
        cell("user"),
        cell("group"),
        cell("size"),
        cell("created"),
        cell("accessed"),
        cell("modified"),
    ];
    let row = vec![
        cell("45"),
        cell(long_name),
        cell("dir"),
        cell(""),
        cell("false"),
        cell("rwxr-xr-x"),
        cell("4"),
        cell("216175737"),
        cell("fdncred"),
        cell("staff"),
        cell("128 B"),
        cell("2 years ago"),
        cell("2 years ago"),
        cell("2 years ago"),
    ];
    let data = vec![header, row.clone(), row];

    for (label, trim) in [
        ("wrap", TrimStrategy::wrap(true)),
        ("trunc", TrimStrategy::truncate(Some(String::from("...")))),
    ] {
        for hos in [false, true] {
            for width in 130..=160 {
                let mut table = NuTable::from(data.clone());
                table.set_theme(theme::rounded());
                table.set_structure(true, true, true);
                table.set_trim(trim.clone());
                table.set_border_header(hos);
                // `ls` compact listings mark `name` as a width-priority column.
                table.set_width_priority_columns(&[1]);
                let Some(rendered) = table.draw(width) else {
                    continue;
                };

                let has_one_char_size_ladder = rendered.lines().any(|line| {
                    let parts: Vec<&str> = line.split('│').collect();
                    parts.iter().any(|part| {
                        let t = part.trim();
                        t == "s" || t == "i" || t == "z" || t == "e"
                    })
                });
                if has_one_char_size_ladder {
                    panic!(
                        "{label} hos={hos} w={width} rendered a 1-character size ladder:\n{rendered}"
                    );
                }
            }
        }
    }
}

fn parse_heavy_header_cells(rendered: &str) -> Vec<String> {
    let header_line = rendered
        .lines()
        .nth(1)
        .expect("table header line is present");

    split_cells(header_line, '┃')
        .into_iter()
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn test_width(data: Vec<Vec<Text<String>>>, tests: &[(usize, &str)]) {
    let tests = tests.iter().map(|&(termwidth, expected)| {
        TestCase::new(termwidth)
            .theme(theme::heavy())
            .trim(TrimStrategy::truncate(Some(String::from("..."))))
            .header()
            .expected(Some(expected.to_owned()))
    });

    test_table(data, tests);
}

fn test_trim(tests: &[(usize, Option<&str>)], trim: TrimStrategy) {
    let tests = tests.iter().map(|&(termwidth, expected)| {
        TestCase::new(termwidth)
            .theme(theme::heavy())
            .trim(trim.clone())
            .header()
            .expected(expected.map(|s| s.to_string()))
    });

    let data = vec![
        vec![
            common::cell("123 45678"),
            common::cell("qweqw eqwe"),
            common::cell("xxx xx xx x xx x xx xx"),
            common::cell("qqq qqq qqqq qqq qq"),
            common::cell("qw"),
        ],
        create_row(5),
        create_row(5),
    ];

    test_table(data, tests);
}

fn test_trim_single(tests: &[(usize, Option<&str>)], trim: TrimStrategy) {
    let tests = tests.iter().map(|&(termwidth, expected)| {
        TestCase::new(termwidth)
            .theme(theme::heavy())
            .trim(trim.clone())
            .header()
            .expected(expected.map(|s| s.to_string()))
    });

    let data = vec![vec![common::cell("0 2 4 6 8 0 2 4 6 8 0")]];

    test_table(data, tests);
}
