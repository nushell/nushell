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

// SQLite reports a statement's errors as its rows are read. A failing INSERT reports on the first
// row, and a failing SELECT reports partway through, after returning some rows.
#[rstest::rstest]
#[case::without_params(
    r#"open $in | query db "INSERT INTO crew (role) VALUES ('captain')""#,
    "NOT NULL constraint failed: crew.name"
)]
#[case::named_params(
    r#"open $in | query db "INSERT INTO crew (role) VALUES (:role)" -p {role: captain}"#,
    "NOT NULL constraint failed: crew.name"
)]
#[case::select_fails_partway(
    r#"open $in | query db "SELECT json(column1) FROM (VALUES ('1'), ('2'), ('x'))""#,
    "malformed JSON"
)]
fn reports_statement_errors(#[case] code: &str, #[case] expected: &str) -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let db = dirs.test().join("test.db");
        Connection::open(&db)
            .expect("failed to open db")
            .execute_batch("CREATE TABLE crew (name TEXT NOT NULL, role TEXT)")
            .expect("failed to create table");
        let mut tester = test().cwd(dirs.test());

        match tester
            .run_with_data::<Value>(code, db.as_path())
            .expect_shell_error()?
        {
            ShellError::Generic(generic) => {
                assert_contains(expected, generic.msg.as_ref());
                Ok(())
            }
            err => Err(err.into()),
        }
    })
}

// `open` reads every table through the same row loop, so its error names the table that failed.
#[test]
fn open_reports_which_table_failed() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |dirs, _| {
        let db = dirs.test().join("test.db");
        Connection::open(&db)
            .expect("failed to open db")
            .execute_batch(
                "CREATE TABLE fine (a TEXT);
                 CREATE TABLE broken (a TEXT);
                 INSERT INTO broken (a) VALUES ('[1]'), ('[2]'), ('not json');
                 ALTER TABLE broken ADD COLUMN b TEXT AS (json(a)) VIRTUAL;",
            )
            .expect("failed to create tables");
        let mut tester = test().cwd(dirs.test());

        match tester
            .run_with_data::<Value>("open $in | values | length", db.as_path())
            .expect_shell_error()?
        {
            ShellError::Generic(generic) => {
                assert_eq!(generic.error, "Failed to read table `broken`");
                assert_contains("malformed JSON", generic.msg.as_ref());
                Ok(())
            }
            err => Err(err.into()),
        }
    })
}
