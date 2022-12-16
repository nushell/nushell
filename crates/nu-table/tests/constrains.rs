mod common;

use nu_protocol::TrimStrategy;
use nu_table::{Table, TableConfig, TableTheme as theme};

use common::{_str, create_row, test_table, TestCase, VecCells};

#[test]
fn data_and_header_has_different_size() {
    let table = Table::new(vec![create_row(3), create_row(5), create_row(5)], (3, 5));

    let table = table.draw(
        TableConfig::new(theme::heavy(), true, false, false),
        usize::MAX,
    );

    assert_eq!(
        table.as_deref(),
        Some(
            "┏━━━┳━━━┳━━━┳━━━┳━━━┓\n\
             ┃ 0 ┃ 1 ┃ 2 ┃   ┃   ┃\n\
             ┣━━━╋━━━╋━━━╋━━━╋━━━┫\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┗━━━┻━━━┻━━━┻━━━┻━━━┛"
        )
    );

    let table = Table::new(vec![create_row(5), create_row(3), create_row(3)], (3, 5));

    let table = table.draw(
        TableConfig::new(theme::heavy(), true, false, false),
        usize::MAX,
    );

    assert_eq!(
        table.as_deref(),
        Some(
            "┏━━━┳━━━┳━━━┳━━━┳━━━┓\n\
             ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃ 4 ┃\n\
             ┣━━━╋━━━╋━━━╋━━━╋━━━┫\n\
             ┃ 0 ┃ 1 ┃ 2 ┃   ┃   ┃\n\
             ┃ 0 ┃ 1 ┃ 2 ┃   ┃   ┃\n\
             ┗━━━┻━━━┻━━━┻━━━┻━━━┛"
        )
    );
}

#[test]
fn termwidth_too_small() {
    let test_loop = |config: TableConfig| {
        for i in 0..10 {
            let table = Table::new(vec![create_row(3), create_row(3), create_row(5)], (3, 5));
            let table = table.draw(config.clone(), i);

            assert!(table.is_none());
        }
    };

    let base_config = TableConfig::new(theme::heavy(), true, false, false);

    let config = base_config.clone();
    test_loop(config);

    let config = base_config.clone().trim(TrimStrategy::truncate(None));
    test_loop(config);

    let config = base_config
        .clone()
        .trim(TrimStrategy::truncate(Some(String::from("**"))));
    test_loop(config);

    let config = base_config
        .clone()
        .trim(TrimStrategy::truncate(Some(String::from(""))));
    test_loop(config);

    let config = base_config.clone().trim(TrimStrategy::wrap(false));
    test_loop(config);

    let config = base_config.trim(TrimStrategy::wrap(true));
    test_loop(config);
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
        (13, Some("┏━━━━━┳━━━━━┓\n┃ 123 ┃ ... ┃\n┃     ┃     ┃\n┃ 456 ┃     ┃\n┃ 78  ┃     ┃\n┣━━━━━╋━━━━━┫\n┃ 0   ┃ ... ┃\n┃ 0   ┃ ... ┃\n┗━━━━━┻━━━━━┛")),
        (21, Some("┏━━━━━━┳━━━━━━┳━━━━━┓\n┃ 123  ┃ qweq ┃ ... ┃\n┃ 4567 ┃  w   ┃     ┃\n┃ 8    ┃ eqwe ┃     ┃\n┣━━━━━━╋━━━━━━╋━━━━━┫\n┃ 0    ┃ 1    ┃ ... ┃\n┃ 0    ┃ 1    ┃ ... ┃\n┗━━━━━━┻━━━━━━┻━━━━━┛")),
        (29, Some("┏━━━━━━━━━━┳━━━━━━━━━━┳━━━━━┓\n┃   123    ┃  qweqw   ┃ ... ┃\n┃ 45678    ┃ eqwe     ┃     ┃\n┣━━━━━━━━━━╋━━━━━━━━━━╋━━━━━┫\n┃ 0        ┃ 1        ┃ ... ┃\n┃ 0        ┃ 1        ┃ ... ┃\n┗━━━━━━━━━━┻━━━━━━━━━━┻━━━━━┛")),
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
fn width_controll_test_0() {
    let data = vec![
        vec![_str("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"); 16],
        vec![_str("yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"); 16],
        vec![_str("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"); 16],
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

fn test_width(data: VecCells, tests: &[(usize, &str)]) {
    let trim = TrimStrategy::truncate(Some(String::from("...")));
    let config = TableConfig::new(nu_table::TableTheme::heavy(), true, false, false).trim(trim);

    let tests = tests.iter().map(|&(termwidth, expected)| {
        TestCase::new(config.clone(), termwidth, Some(expected.to_owned()))
    });

    test_table(data, tests);
}

fn test_trim(tests: &[(usize, Option<&str>)], trim: TrimStrategy) {
    let config = TableConfig::new(nu_table::TableTheme::heavy(), true, false, false).trim(trim);
    let tests = tests.iter().map(|&(termwidth, expected)| {
        TestCase::new(config.clone(), termwidth, expected.map(|s| s.to_string()))
    });

    let data = create_test_table0();

    test_table(data, tests);
}

fn create_test_table0() -> VecCells {
    let header = vec![
        _str("123 45678"),
        _str("qweqw eqwe"),
        _str("xxx xx xx x xx x xx xx"),
        _str("qqq qqq qqqq qqq qq"),
        _str("qw"),
    ];

    vec![header, create_row(5), create_row(5)]
}
