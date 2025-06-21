use nu_test_support::{nu, nu_repl_code, playground::Playground};

// Multiple nu! calls don't persist state, so we can't store it in a function
const DATABASE_INIT: &str = r#"stor open | query db "CREATE TABLE IF NOT EXISTS test_db (
    name TEXT,
    age INTEGER,
    height REAL,
    serious BOOLEAN,
    created_at DATETIME,
    largest_file INTEGER,
    time_slept INTEGER,
    null_field TEXT,
    data BLOB
)""#;

#[test]
fn data_types() {
    Playground::setup("empty", |_, _| {
        let results = nu!(nu_repl_code(&[
            DATABASE_INIT,
            // Add row with our data types
            r#"stor open
                | query db "INSERT INTO test_db VALUES (
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
            // Query our table with the row we just added to get its nushell types
            r#"
                stor open | query db "SELECT * FROM test_db" | first | values | each { describe } | str join "-"
            "#
        ]));

        // Assert data types match. Booleans are mapped to "numeric" due to internal SQLite representations:
        // https://www.sqlite.org/datatype3.html
        // They are simply 1 or 0 in practice, but the column could contain any valid SQLite value
        assert_eq!(
            results.out,
            "string-int-float-int-string-int-int-nothing-binary"
        );
    });
}

#[test]
fn ordered_params() {
    Playground::setup("empty", |_, _| {
        let results = nu!(nu_repl_code(&[
            DATABASE_INIT,
            // Add row with our data types
            r#"(stor open
                | query db "INSERT INTO test_db VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                -p [ 'nimurod', 20, 6.0, true, ('2024-03-23T00:15:24-03:00' | into datetime), 72.4mb, 1ms, null, ("hello" | into binary) ]
            )"#,
            // Query our nu values and types
            r#"
                let values = (stor open | query db "SELECT * FROM test_db" | first | values);

                ($values | str join '-') + "_" + ($values | each { describe } | str join '-')
            "#
        ]));

        assert_eq!(
            results.out,
            "nimurod-20-6.0-1-2024-03-23 00:15:24-03:00-72400000-1000000--[104, 101, 108, 108, 111]_\
            string-int-float-int-string-int-int-nothing-binary"
        );
    });
}

#[test]
fn named_params() {
    Playground::setup("empty", |_, _| {
        let results = nu!(nu_repl_code(&[
            DATABASE_INIT,
            // Add row with our data types. query db should support all possible named parameters
            // @-prefixed, $-prefixed, and :-prefixed
            // But :prefix is the "blessed" way to do it, and as such, the only one that's
            // promoted to from a bare word `key: value` property in the record
            // In practice, users should not use @param or $param
            r#"(stor open
                | query db "INSERT INTO test_db VALUES (:name, :age, @height, $serious, :created_at, :largest_file, :time_slept, :null_field, :data)"
                -p {
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
            )"#,
            // Query our nu values and types
            r#"
                let values = (stor open | query db "SELECT * FROM test_db" | first | values);

                ($values | str join '-') + "_" + ($values | each { describe } | str join '-')
            "#
        ]));

        assert_eq!(
            results.out,
            "nimurod-20-6.0-1-2024-03-23 00:15:24-03:00-72400000-1000000--[104, 101, 108, 108, 111]_\
            string-int-float-int-string-int-int-nothing-binary"
        );
    });
}
