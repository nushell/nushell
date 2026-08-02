mod common;

use nu_protocol::TrimStrategy;
use nu_table::{NuTable, TableTheme as theme};

use common::{TestCase, cell, create_row, test_table};

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
fn wrap_test() {
    for test in 0..15 {
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
                "┏━━━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw e ┃ ... ┃\n\
                 ┃           ┃ qwe     ┃     ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━┻━━━━━┛",
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
                "┏━━━━━━━━━━━━┓\n\
                 ┃ 0 2 4 6 8  ┃\n\
                 ┃ 0 2 4 6 8  ┃\n\
                 ┃ 0          ┃\n\
                 ┗━━━━━━━━━━━━┛",
            ),
        ),
        (
            21,
            Some(
                "┏━━━━━━━━━━━━━━━━━━┓\n\
                 ┃ 0 2 4 6 8 0 2 4  ┃\n\
                 ┃ 6 8 0            ┃\n\
                 ┗━━━━━━━━━━━━━━━━━━┛",
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
    for test in 0..15 {
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
                "┏━━━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw   ┃ ... ┃\n\
                 ┃           ┃ eqwe    ┃     ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━┻━━━━━┛",
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
    for test in 0..15 {
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
                "┏━━━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweqw e ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━┻━━━━━┛",
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
    for test in 0..15 {
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
                "┏━━━━━━━━━━━┳━━━━━━━━━┳━━━━━┓\n\
                 ┃ 123 45678 ┃ qweq... ┃ ... ┃\n\
                 ┣━━━━━━━━━━━╋━━━━━━━━━╋━━━━━┫\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┃ 0         ┃ 1       ┃ ... ┃\n\
                 ┗━━━━━━━━━━━┻━━━━━━━━━┻━━━━━┛",
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
            "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            120,
            "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            121,
            "┏━━━━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━┛",
        ),
        (
            150,
            "┏━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━┓\n\
             ┃ xxxxxxxxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ ... ┃\n\
             ┣━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━┫\n\
             ┃ yyyyyyyyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ ... ┃\n\
             ┃ zzzzzzzzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ ... ┃\n\
             ┗━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━┛",
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
fn forced_mode_keeps_primary_priority_dominant() {
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
    table.set_width_priority_columns(&[6, 2]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(121).expect("table renders");
    let widths = parse_heavy_header_column_widths(&rendered);

    assert!(rendered.contains("..."));
    assert_eq!(widths.len(), 8);
    assert!(
        widths[6] > widths[2],
        "expected primary priority column to stay wider than secondary; rendered table:\n{rendered}"
    );
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
    assert_eq!(primary_widths.len(), secondary_widths.len());
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
    let rendered_width = header_columns_to_total_width(&widths);

    assert_eq!(rendered_width, 121);
}

#[test]
fn right_priority_compacts_columns_to_its_right_when_partial() {
    let headers: Vec<Text<String>> = (0..14).map(|i| cell(&format!("c{i}"))).collect();

    let row: Vec<Text<String>> = (0..14)
        .map(|i| cell(&format!("value_{i}_abcdefghijklmnopqrstuvwxyz0123456789")))
        .collect();

    let mut table = NuTable::new(3, 14);
    table.set_theme(theme::heavy());
    table.set_structure(false, true, false);
    // c7 is on the right side of the initial visible range at this width.
    table.set_width_priority_columns(&[7, 3]);
    table.set_row(0, headers);
    table.set_row(1, row.clone());
    table.set_row(2, row);

    let rendered = table.draw(121).expect("table renders");
    let header_cells = parse_heavy_header_cells(&rendered);

    assert!(
        header_cells.iter().any(|cell| cell == "c7"),
        "expected c7 to remain visible; header cells: {header_cells:?}\n{rendered}"
    );
    assert!(
        !header_cells.iter().any(|cell| cell == "c8"),
        "expected c8 to be removed; header cells: {header_cells:?}\n{rendered}"
    );
}

#[test]
fn single_left_priority_drops_trailing_columns_for_wider_values() {
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

    assert!(rendered.contains(priority_value));
    assert!(
        !header_cells.iter().any(|cell| cell == "c11"),
        "expected trailing columns to be dropped first; header cells: {header_cells:?}\n{rendered}"
    );
}

fn parse_heavy_header_column_widths(rendered: &str) -> Vec<usize> {
    let header_line = rendered
        .lines()
        .nth(1)
        .expect("table header line is present");

    let cells: Vec<&str> = header_line.split('┃').collect();
    cells[1..cells.len() - 1]
        .iter()
        .map(|cell| cell.chars().count())
        .collect()
}

fn parse_heavy_header_cells(rendered: &str) -> Vec<String> {
    let header_line = rendered
        .lines()
        .nth(1)
        .expect("table header line is present");

    let cells: Vec<&str> = header_line.split('┃').collect();
    cells[1..cells.len() - 1]
        .iter()
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn header_columns_to_total_width(widths: &[usize]) -> usize {
    // For heavy theme with outer borders and vertical separators, total width is:
    // sum(column widths) + number of separators (columns + 1).
    widths.iter().sum::<usize>() + widths.len() + 1
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
