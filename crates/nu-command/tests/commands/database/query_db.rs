use std::path::{Path, PathBuf};

use indoc::indoc;
use nu_test_support::prelude::*;
use rusqlite::Connection;

#[track_caller]
fn database_init(dir: impl AsRef<Path>) -> PathBuf {
    let path = dir.as_ref().join("test.db");
    Connection::open(&path)
        .expect("failed to open db")
        .execute_batch(indoc! {"
            CREATE TABLE IF NOT EXISTS test_db (
                name TEXT,
                age INTEGER,
                height REAL,
                serious BOOLEAN,
                created_at DATETIME,
                largest_file INTEGER,
                time_slept INTEGER,
                null_field TEXT,
                data BLOB
            )
        "})
        .expect("failed to create table");
    path
}

#[test]
fn data_types() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let db = database_init(dirs.test());
        let mut tester = test().cwd(dirs.test());

        // Add row with our data types
        let []: [(); 0] = tester.run_with_data(
            r#"
                open $in | query db "INSERT INTO test_db VALUES (
                    'nimurod',
                    20,
                    6.0,
                    true,
                    date('2024-03-23T00:15:24-03:00'),
                    72400000,
                    1000000,
                    NULL,
                    x'68656c6c6f'
                )"
            "#,
            db.as_path(),
        )?;

        // Query our table with the row we just added to get its nushell types
        tester
            .run_with_data(
                "
                    open $in
                    | query db 'SELECT * FROM test_db'
                    | first
                    | values
                    | each { describe }
                ",
                db.as_path(),
            )
            .expect_value_eq(
                // Assert data types match.
                // Booleans are mapped to "numeric" due to internal SQLite representations:
                // https://www.sqlite.org/datatype3.html
                // They are simply 1 or 0 in practice,
                // but the column could contain any valid SQLite value
                [
                    "string", "int", "float", "int", "string", "int", "int", "nothing", "binary",
                ],
            )
    })
}

#[test]
fn ordered_params() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let db = database_init(dirs.test());
        let mut tester = test().cwd(dirs.test());

        // Add row with our data types
        let []: [(); 0] = tester.run_with_data(
            r#"
                open $in | query db "INSERT INTO test_db VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)" -p [
                    'nimurod',
                    20,
                    6.0,
                    true,
                    ('2024-03-23T00:15:24-03:00' | into datetime),
                    72.4mb,
                    1ms,
                    null,
                    ("hello" | into binary)
                ]
            "#,
            db.as_path(),
        )?;

        // Query our nu values and types
        tester
            .run_with_data(
                r#"
                    open $in
                    | query db "SELECT * FROM test_db"
                    | first
                    | values
                    | { values: $in, types: ($in | each { describe }) }
                "#,
                db.as_path(),
            )
            .expect_value_eq(test_value! {
                {
                    values: [
                        "nimurod",
                        20,
                        6.0,
                        1,
                        "2024-03-23 00:15:24-03:00",
                        72400000,
                        1000000,
                        (),
                        (Value::test_binary(b"hello")),
                    ],
                    types: ["string", "int", "float", "int", "string", "int", "int", "nothing", "binary"],
                }
            })
    })
}

#[test]
fn named_params() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let db = database_init(dirs.test());
        let mut tester = test().cwd(dirs.test());

        // Add row with our data types. query db should support all possible named parameters
        // @-prefixed, $-prefixed, and :-prefixed
        // But :prefix is the "blessed" way to do it, and as such, the only one that's
        // promoted to from a bare word `key: value` property in the record
        // In practice, users should not use @param or $param
        let []: [(); 0] = tester.run_with_data(
            r#"
                open $in | query db "INSERT INTO test_db VALUES (:name, :age, @height, $serious, :created_at, :largest_file, :time_slept, :null_field, :data)" -p {
                    name: 'nimurod',
                    ':age': 20,
                    '@height': 6.0,
                    '$serious': true,
                    created_at: ('2024-03-23T00:15:24-03:00' | into datetime),
                    largest_file: 72.4mb,
                    time_slept: 1ms,
                    null_field: null,
                    data: ("hello" | into binary)
                }
            "#,
            db.as_path(),
        )?;

        // Query our nu values and types
        tester
            .run_with_data(
                r#"
                    open $in
                    | query db "SELECT * FROM test_db"
                    | first
                    | values
                    | { values: $in, types: ($in | each { describe }) }
                "#,
                db.as_path(),
            )
            .expect_value_eq(test_value! {
                {
                    values: [
                        "nimurod",
                        20,
                        6.0,
                        1,
                        "2024-03-23 00:15:24-03:00",
                        72400000,
                        1000000,
                        (),
                        (Value::test_binary(b"hello")),
                    ],
                    types: ["string", "int", "float", "int", "string", "int", "int", "nothing", "binary"],
                }
            })
    })
}
