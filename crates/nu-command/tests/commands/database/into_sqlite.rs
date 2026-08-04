use chrono::{DateTime, FixedOffset};
use nu_path::AbsolutePathBuf;
use nu_protocol::{Span, Value, ast::PathMember, casing::Casing, engine::EngineState, record};
use nu_test_support::{fs::Stub, prelude::*};
use rand::{
    SeedableRng,
    distr::{Alphanumeric, SampleString, StandardUniform},
    prelude::*,
    random_range,
    rngs::{StdRng, SysRng},
};
use std::io::Write;

#[test]
fn into_sqlite_schema() -> Result {
    Playground::setup("schema", |dirs, _| {
        let testdb = make_sqlite_db(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10 11:30:00", "foo", ("binary" | into binary)],
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10 12:30:00", "bar", ("wut" | into binary)],
            ]"#,
        )?;

        let conn = rusqlite::Connection::open(testdb).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM pragma_table_info(?1)").unwrap();

        let actual_rows: Vec<_> = stmt
            .query_and_then(["main"], |row| -> rusqlite::Result<_, rusqlite::Error> {
                let name: String = row.get("name").unwrap();
                let col_type: String = row.get("type").unwrap();
                Ok((name, col_type))
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect();

        let expected_rows = vec![
            ("somebool".into(), "BOOLEAN".into()),
            ("someint".into(), "INTEGER".into()),
            ("somefloat".into(), "REAL".into()),
            ("somefilesize".into(), "INTEGER".into()),
            ("someduration".into(), "BIGINT".into()),
            ("somedate".into(), "TEXT".into()),
            ("somestring".into(), "TEXT".into()),
            ("somebinary".into(), "BLOB".into()),
        ];

        assert_eq!(expected_rows, actual_rows);
        Ok(())
    })
}

#[test]
fn into_sqlite_values() -> Result {
    Playground::setup("values", |dirs, _| {
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary), 1],
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary), null],
            ]"#,
            None,
            vec![
                TestRow(
                    true,
                    1,
                    2.0,
                    1000,
                    1000000000,
                    DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                    "foo".into(),
                    b"binary".to_vec(),
                    rusqlite::types::Value::Integer(1),
                ),
                TestRow(
                    false,
                    2,
                    3.0,
                    2000000,
                    2419200000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "bar".into(),
                    b"wut".to_vec(),
                    rusqlite::types::Value::Null,
                ),
            ],
        )
    })
}

/// When we create a new table, we use the first row to infer the schema of the
/// table. In the event that a column is null, we can't know what type the row
/// should be, so we just assume TEXT.
#[test]
#[deps(NU)]
fn into_sqlite_values_first_column_null() -> Result {
    Playground::setup("values", |dirs, _| {
        let testdir = dirs.test();
        let testdb_path =
            testdir.join(testdir.file_name().unwrap().to_str().unwrap().to_owned() + ".db");
        let expected = vec![
            TestRow(
                false,
                2,
                3.0,
                2000000,
                2419200000000000,
                DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                "bar".into(),
                b"wut".to_vec(),
                rusqlite::types::Value::Null,
            ),
            TestRow(
                true,
                1,
                2.0,
                1000,
                1000000000,
                DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                "foo".into(),
                b"binary".to_vec(),
                rusqlite::types::Value::Text("1".into()),
            ),
        ];

        let testdb = testdb_path.to_string_lossy().into_owned();
        let child_code = format!(
            r#"let db = {:?}; [
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary), null],
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary), 1],
            ] | into sqlite $db"#,
            testdb.as_str()
        );
        let result: CompleteResult = test().cwd(testdir).run_with_data(
            "let child_code = $in; nu -n -c $child_code | complete",
            child_code,
        )?;
        assert_eq!(0, result.exit_code, "{}", result.stderr);

        let conn = rusqlite::Connection::open(testdb_path).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM main;").unwrap();
        let actual_rows: Vec<_> = stmt
            .query_and_then([], |row| TestRow::try_from(row))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();

        assert_eq!(expected, actual_rows);
        Ok(())
    })
}

/// If the DB / table already exist, then the insert should end up with the
/// right data types no matter if the first row is null or not.
#[test]
fn into_sqlite_values_first_column_null_preexisting_db() -> Result {
    Playground::setup("values", |dirs, _| {
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary), 1],
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary), null],
            ]"#,
            None,
            vec![
                TestRow(
                    true,
                    1,
                    2.0,
                    1000,
                    1000000000,
                    DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                    "foo".into(),
                    b"binary".to_vec(),
                    rusqlite::types::Value::Integer(1),
                ),
                TestRow(
                    false,
                    2,
                    3.0,
                    2000000,
                    2419200000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "bar".into(),
                    b"wut".to_vec(),
                    rusqlite::types::Value::Null,
                ),
            ],
        )?;

        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [true, 3, 5.0, 3.1mb, 1wk, "2020-09-10T12:30:00-00:00", "baz", ("huh" | into binary), null],
                [true, 3, 5.0, 3.1mb, 1wk, "2020-09-10T12:30:00-00:00", "baz", ("huh" | into binary), 3],
            ]"#,
            None,
            vec![
                TestRow(
                    true,
                    1,
                    2.0,
                    1000,
                    1000000000,
                    DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                    "foo".into(),
                    b"binary".to_vec(),
                    rusqlite::types::Value::Integer(1),
                ),
                TestRow(
                    false,
                    2,
                    3.0,
                    2000000,
                    2419200000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "bar".into(),
                    b"wut".to_vec(),
                    rusqlite::types::Value::Null,
                ),
                TestRow(
                    true,
                    3,
                    5.0,
                    3100000,
                    604800000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "baz".into(),
                    b"huh".to_vec(),
                    rusqlite::types::Value::Null,
                ),
                TestRow(
                    true,
                    3,
                    5.0,
                    3100000,
                    604800000000000,
                    DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                    "baz".into(),
                    b"huh".to_vec(),
                    rusqlite::types::Value::Integer(3),
                ),
            ],
        )
    })
}

/// Opening a preexisting database should append to it
#[test]
#[deps(NU)]
fn into_sqlite_existing_db_append() -> Result {
    Playground::setup("existing_db_append", |dirs, _| {
        let testdir = dirs.test();
        let testdb_path =
            testdir.join(testdir.file_name().unwrap().to_str().unwrap().to_owned() + ".db");
        let testdb = testdb_path.to_string_lossy().into_owned();

        // create a new DB with only one row
        let child_code = format!(
            r#"let db = {:?}; [
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary), null],
            ] | into sqlite $db"#,
            testdb.as_str()
        );
        let result: CompleteResult = test().cwd(testdir).run_with_data(
            "let child_code = $in; nu -n -c $child_code | complete",
            child_code,
        )?;
        assert_eq!(0, result.exit_code, "{}", result.stderr);

        let expected = vec![TestRow(
            true,
            1,
            2.0,
            1000,
            1000000000,
            DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
            "foo".into(),
            b"binary".to_vec(),
            rusqlite::types::Value::Null,
        )];
        let conn = rusqlite::Connection::open(&testdb_path).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM main;").unwrap();
        let actual_rows: Vec<_> = stmt
            .query_and_then([], |row| TestRow::try_from(row))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        assert_eq!(expected, actual_rows);

        // open the same DB again and write one row
        let child_code = format!(
            r#"let db = {:?}; [
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary), null],
            ] | into sqlite $db"#,
            testdb.as_str()
        );
        let result: CompleteResult = test().cwd(testdir).run_with_data(
            "let child_code = $in; nu -n -c $child_code | complete",
            child_code,
        )?;
        assert_eq!(0, result.exit_code, "{}", result.stderr);

        let expected = vec![
            TestRow(
                true,
                1,
                2.0,
                1000,
                1000000000,
                DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                "foo".into(),
                b"binary".to_vec(),
                rusqlite::types::Value::Null,
            ),
            TestRow(
                false,
                2,
                3.0,
                2000000,
                2419200000000000,
                DateTime::parse_from_rfc3339("2020-09-10T12:30:00-00:00").unwrap(),
                "bar".into(),
                b"wut".to_vec(),
                rusqlite::types::Value::Null,
            ),
        ];
        let conn = rusqlite::Connection::open(testdb_path).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM main;").unwrap();
        let actual_rows: Vec<_> = stmt
            .query_and_then([], |row| TestRow::try_from(row))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();

        assert_eq!(expected, actual_rows);
        Ok(())
    })
}

/// Test inserting a good number of randomly generated rows to test an actual
/// streaming pipeline instead of a simple value
#[test]
#[deps(NU)]
fn into_sqlite_big_insert() -> Result {
    let engine_state = EngineState::new();
    // don't serialize closures
    let serialize_types = false;
    Playground::setup("big_insert", |dirs, playground| {
        const NUM_ROWS: usize = 10_000;
        const NUON_FILE_NAME: &str = "data.nuon";

        let nuon_path = dirs.test().join(NUON_FILE_NAME);

        playground.with_files(&[Stub::EmptyFile(&nuon_path.to_string_lossy())]);

        let mut expected_rows = Vec::new();
        let mut nuon_file = std::fs::OpenOptions::new()
            .write(true)
            .open(&nuon_path)
            .unwrap();

        // write the header
        for row in std::iter::repeat_with(TestRow::random).take(NUM_ROWS) {
            let mut value: Value = row.clone().into();

            // HACK: Convert to a string to get around this: https://github.com/nushell/nushell/issues/9186
            value
                .upsert_cell_path(
                    &[PathMember::String {
                        val: "somedate".into(),
                        span: Span::test_data(),
                        optional: false,
                        casing: Casing::Sensitive,
                    }],
                    Box::new(|dateval| {
                        Value::string(dateval.coerce_string().unwrap(), dateval.span())
                    }),
                )
                .unwrap();

            let nuon = nuon::to_nuon(
                &engine_state,
                &value,
                nuon::ToNuonConfig::default()
                    .span(Some(Span::test_data()))
                    .serialize_types(serialize_types),
            )
            .unwrap()
                + nu_utils::consts::LINE_SEPARATOR_STR;

            nuon_file.write_all(nuon.as_bytes()).unwrap();
            expected_rows.push(row);
        }

        let testdir = dirs.test();
        let testdb_path =
            testdir.join(testdir.file_name().unwrap().to_str().unwrap().to_owned() + ".db");
        let testdb = testdb_path.to_string_lossy().into_owned();
        let child_code = format!(
            "let db = {:?}; open --raw {} | lines | each {{ from nuon }} | into sqlite $db",
            testdb.as_str(),
            nuon_path.to_string_lossy()
        );
        let result: CompleteResult = test().cwd(testdir).run_with_data(
            "let child_code = $in; nu -n -c $child_code | complete",
            child_code,
        )?;
        assert_eq!(0, result.exit_code, "{}", result.stderr);

        let conn = rusqlite::Connection::open(testdb_path).unwrap();
        let mut stmt = conn.prepare("SELECT * FROM main;").unwrap();
        let actual_rows: Vec<_> = stmt
            .query_and_then([], |row| TestRow::try_from(row))
            .unwrap()
            .map(|row| row.unwrap())
            .collect();

        assert_eq!(expected_rows, actual_rows);
        Ok(())
    })
}

/// empty in, empty out
#[test]
fn into_sqlite_empty() -> Result {
    Playground::setup("empty", |dirs, _| {
        insert_test_rows(&dirs, "[]", Some("SELECT * FROM sqlite_schema;"), vec![])
    })
}

#[derive(Debug, PartialEq, Clone)]
struct TestRow(
    bool,
    i64,
    f64,
    i64,
    i64,
    chrono::DateTime<chrono::FixedOffset>,
    std::string::String,
    std::vec::Vec<u8>,
    rusqlite::types::Value,
);

impl TestRow {
    pub fn random() -> Self {
        StdRng::try_from_rng(&mut SysRng)
            .expect("OS RNG unavailable")
            .sample(StandardUniform)
    }
}

impl From<TestRow> for Value {
    fn from(row: TestRow) -> Self {
        Value::record(
            record! {
                "somebool" => Value::bool(row.0, Span::test_data()),
                "someint" => Value::int(row.1, Span::test_data()),
                "somefloat" => Value::float(row.2, Span::test_data()),
                "somefilesize" => Value::filesize(row.3, Span::test_data()),
                "someduration" => Value::duration(row.4, Span::test_data()),
                "somedate" => Value::date(row.5, Span::test_data()),
                "somestring" => Value::string(row.6, Span::test_data()),
                "somebinary" => Value::binary(row.7, Span::test_data()),
                "somenull" => Value::nothing(Span::test_data()),
            },
            Span::test_data(),
        )
    }
}

impl TryFrom<&rusqlite::Row<'_>> for TestRow {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> Result<Self, Self::Error> {
        let somebool: bool = row.get("somebool").unwrap();
        let someint: i64 = row.get("someint").unwrap();
        let somefloat: f64 = row.get("somefloat").unwrap();
        let somefilesize: i64 = row.get("somefilesize").unwrap();
        let someduration: i64 = row.get("someduration").unwrap();
        let somedate: DateTime<FixedOffset> = row.get("somedate").unwrap();
        let somestring: String = row.get("somestring").unwrap();
        let somebinary: Vec<u8> = row.get("somebinary").unwrap();
        let somenull: rusqlite::types::Value = row.get("somenull").unwrap();

        Ok(TestRow(
            somebool,
            someint,
            somefloat,
            somefilesize,
            someduration,
            somedate,
            somestring,
            somebinary,
            somenull,
        ))
    }
}

impl Distribution<TestRow> for StandardUniform {
    fn sample<R>(&self, rng: &mut R) -> TestRow
    where
        R: rand::RngExt + ?Sized,
    {
        let dt = DateTime::from_timestamp_millis(random_range(0..2324252554000))
            .unwrap()
            .fixed_offset();

        let rand_string = Alphanumeric.sample_string(rng, 10);

        // limit the size of the numbers to work around
        // https://github.com/nushell/nushell/issues/10612
        let filesize = random_range(-1024..=1024);
        let duration = random_range(-1024..=1024);

        TestRow(
            rng.random(),
            rng.random(),
            rng.random(),
            filesize,
            duration,
            dt,
            rand_string,
            rng.random::<u64>().to_be_bytes().to_vec(),
            rusqlite::types::Value::Null,
        )
    }
}

fn make_sqlite_db(dirs: &Dirs, nu_table: &str) -> Result<AbsolutePathBuf> {
    let testdir = dirs.test();
    let testdb_path =
        testdir.join(testdir.file_name().unwrap().to_str().unwrap().to_owned() + ".db");

    let () = test().cwd(testdir).run_with_data(
        format!("let db = $in; {nu_table} | into sqlite $db"),
        testdb_path.clone(),
    )?;

    Ok(testdb_path)
}

fn insert_test_rows(
    dirs: &Dirs,
    nu_table: &str,
    sql_query: Option<&str>,
    expected: Vec<TestRow>,
) -> Result {
    let sql_query = sql_query.unwrap_or("SELECT * FROM main;");
    let testdb = make_sqlite_db(dirs, nu_table)?;

    let conn = rusqlite::Connection::open(testdb).unwrap();
    let mut stmt = conn.prepare(sql_query).unwrap();

    let actual_rows: Vec<_> = stmt
        .query_and_then([], |row| TestRow::try_from(row))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    assert_eq!(expected, actual_rows);
    Ok(())
}

#[test]
fn test_auto_conversion() -> Result {
    Playground::setup("sqlite json auto conversion", |_, playground| {
        let raw = "{a_record:{foo:bar,baz:quux},a_list:[1,2,3],a_table:[[a,b];[0,1],[2,3]]}";
        let db = playground.cwd().join("filename.db");
        let () = test().cwd(playground.cwd()).run_with_data(
            format!("let db = $in; {raw} | into sqlite $db -t my_table"),
            db,
        )?;
        test()
            .cwd(playground.cwd())
            .run("open filename.db | get my_table.0 | to nuon --raw")
            .expect_value_eq(raw)
    })
}
