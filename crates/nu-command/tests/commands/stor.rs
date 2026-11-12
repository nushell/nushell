use nu_test_support::prelude::*;

#[test]
fn stor_insert() -> Result {
    let code = r#"
        stor create --table-name stor_insert_table --columns { id: int, value: str };
        stor insert -t stor_insert_table --data-record {
            id: 1
            value: "'Initial value'"
        };
        stor open | query db 'select value from stor_insert_table' | get 0.value
    "#;

    test().run(code).expect_value_eq("'Initial value'")
}

#[test]
fn stor_update_with_quote() -> Result {
    let code = r#"
        stor create --table-name stor_update_table --columns { id: int, value: str };
        stor insert -t stor_update_table --data-record {
            id: 1
            value: "'Initial value'"
        };
        stor update -t stor_update_table --where-clause 'id = 1' --update-record {
            id: 1
            value: "This didn't work, but should now."
        };
        stor open | query db 'select value from stor_update_table' | get 0.value
    "#;

    test()
        .run(code)
        .expect_value_eq("This didn't work, but should now.")
}

// The first statement of a block receives the block's input, so these calls get both that
// input and the record flag. The flag wins.
#[test]
fn stor_insert_data_record_in_block() -> Result {
    let code = "
        stor create --table-name stor_insert_block_table --columns { a: int };
        [{a: 1} {a: 2}] | each {|r| stor insert -t stor_insert_block_table -d $r };
        stor open | query db 'select a from stor_insert_block_table' | get a
    ";

    test().run(code).expect_value_eq([1, 2])
}

#[test]
fn stor_update_update_record_in_block() -> Result {
    let code = "
        stor create --table-name stor_update_block_table --columns { a: int };
        stor insert -t stor_update_block_table -d {a: 1};
        {a: 9} | do { stor update -t stor_update_block_table -u {a: 2} };
        stor open | query db 'select a from stor_update_block_table' | get a
    ";

    test().run(code).expect_value_eq([2])
}

// `stor import` hands the path to sqlite, which resolves a relative one against the process
// working directory. The test harness does not keep that in sync with `$env.PWD`, so these
// tests use an absolute path inside the playground rather than setting `cwd`.
#[test]
fn stor_import_missing_file_errors() -> Result {
    Playground::setup("stor_import_missing_file", |dirs, _| {
        let missing = dirs.test().join("missing.sqlite");

        test()
            .env("MISSING_DB", missing.to_string_lossy().into_owned())
            .run::<Value>("stor import --file-name $env.MISSING_DB")
            .expect_error_code_eq("nu::shell::io::file_not_found")
    })
}

#[test]
fn stor_import_missing_file_keeps_existing_data() -> Result {
    Playground::setup("stor_import_keeps_existing_data", |dirs, _| {
        let missing = dirs.test().join("missing.sqlite");
        let code = "
            stor create --table-name stor_import_table --columns { id: int };
            stor insert -t stor_import_table --data-record { id: 1 };
            try { stor import --file-name $env.MISSING_DB };
            stor open | query db 'select id from stor_import_table' | get 0.id
        ";

        test()
            .env("MISSING_DB", missing.to_string_lossy().into_owned())
            .run(code)
            .expect_value_eq(1i64)
    })
}
