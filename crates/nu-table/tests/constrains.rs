mod common;

use nu_protocol::TrimStrategy;
use nu_table::{NuTable, TableTheme as theme};

use common::{create_row, test_table, TestCase};

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

    test_trim(&tests, TrimStrategy::wrap(false));
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
        (13, Some("┏━━━━━┳━━━━━┓\n┃ 123 ┃ ... ┃\n┃  45 ┃     ┃\n┃ 678 ┃     ┃\n┣━━━━━╋━━━━━┫\n┃ 0   ┃ ... ┃\n┃ 0   ┃ ... ┃\n┗━━━━━┻━━━━━┛")),
        (21, Some("┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┃ 4567 ┃ w    ┃     ┃\n┃ 8    ┃ eqwe ┃     ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛")),
        (29, Some("┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃ 123      ┃ qweqw    ┃ ... ┃\n┃ 45678    ┃ eqwe     ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛")),
        (49, Some("┏━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ 123 45678 ┃ qweqw eqwe ┃ xxx xx xx x xx ┃ ... ┃\n┃           ┃            ┃  x xx xx       ┃     ┃\n┣━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┃ 0         ┃ 1          ┃ 2              ┃ ... ┃\n┗━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━┻━━━━━┛")),
    ];

    test_trim(&tests, TrimStrategy::wrap(true));
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

    test_trim(&tests, TrimStrategy::truncate(None));
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

    test_trim(&tests, TrimStrategy::truncate(Some(String::from("..."))));
}

#[test]
fn width_control_test_0() {
    let data = vec![
        vec![common::cell("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"); 16],
        vec![common::cell("yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"); 16],
        vec![common::cell("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"); 16],
    ];

    let tests = [
        (20, "┏━━━━━━━━━━━━┳━━━━━┓\n┃ xxxxxxx... ┃ ... ┃\n┣━━━━━━━━━━━━╋━━━━━┫\n┃ yyyyyyy... ┃ ... ┃\n┃ zzzzzzz... ┃ ... ┃\n┗━━━━━━━━━━━━┻━━━━━┛"),
        (119, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx... ┃ ... ┃\n┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy... ┃ ... ┃\n┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz... ┃ ... ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━┛"),
        (120, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━┓\n┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx... ┃ ... ┃\n┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━┫\n┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy... ┃ ... ┃\n┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz... ┃ ... ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━┛"),
        (121, "┏━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━┓\n┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxx... ┃ xxxxxxxxx... ┃ xxxxxxxxx... ┃ ... ┃\n┣━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━╋━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━╋━━━━━┫\n┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyy... ┃ yyyyyyyyy... ┃ yyyyyyyyy... ┃ ... ┃\n┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzz... ┃ zzzzzzzzz... ┃ zzzzzzzzz... ┃ ... ┃\n┗━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━┻━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━┻━━━━━┛"),
        (150, "┏━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━━━━━━━━┳━━━━━┓\n┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ xxxxxxx... ┃ ... ┃\n┣━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━━━━━━━━╋━━━━━┫\n┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ yyyyyyy... ┃ ... ┃\n┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ zzzzzzz... ┃ ... ┃\n┗━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━━━━━━━━┻━━━━━┛"),
        (usize::MAX, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓\n┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx ┃\n┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫\n┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃ yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy ┃\n┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃ zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ┃\n┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛"),
    ];

    test_width(data, &tests);
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
