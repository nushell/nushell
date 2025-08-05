use chrono::{DateTime, FixedOffset};
use nu_path::AbsolutePathBuf;
use nu_protocol::{Span, Value, ast::PathMember, casing::Casing, engine::EngineState, record};
use nu_test_support::{
    fs::{Stub, line_ending},
    nu, pipeline,
    playground::{Dirs, Playground},
};
use rand::{
    Rng, SeedableRng,
    distr::{Alphanumeric, SampleString, StandardUniform},
    prelude::Distribution,
    random_range,
    rngs::StdRng,
};
use std::io::Write;

#[test]
fn into_sqlite_schema() {
    Playground::setup("schema", |dirs, _| {
        let testdb = make_sqlite_db(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10 11:30:00", "foo", ("binary" | into binary)],
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10 12:30:00", "bar", ("wut" | into binary)],
            ]"#,
        );

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
    });
}

#[test]
fn into_sqlite_values() {
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
        );
    });
}

/// When we create a new table, we use the first row to infer the schema of the
/// table. In the event that a column is null, we can't know what type the row
/// should be, so we just assume TEXT.
#[test]
fn into_sqlite_values_first_column_null() {
    Playground::setup("values", |dirs, _| {
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary), null],
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary), 1],
            ]"#,
            None,
            vec![
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
            ],
        );
    });
}

/// If the DB / table already exist, then the insert should end up with the
/// right data types no matter if the first row is null or not.
#[test]
fn into_sqlite_values_first_column_null_preexisting_db() {
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
        );

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
        );
    });
}

/// Opening a preexisting database should append to it
#[test]
fn into_sqlite_existing_db_append() {
    Playground::setup("existing_db_append", |dirs, _| {
        // create a new DB with only one row
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [true, 1, 2.0, 1kb, 1sec, "2023-09-10T11:30:00-00:00", "foo", ("binary" | into binary), null],
            ]"#,
            None,
            vec![TestRow(
                true,
                1,
                2.0,
                1000,
                1000000000,
                DateTime::parse_from_rfc3339("2023-09-10T11:30:00-00:00").unwrap(),
                "foo".into(),
                b"binary".to_vec(),
                rusqlite::types::Value::Null,
            )],
        );

        // open the same DB again and write one row
        insert_test_rows(
            &dirs,
            r#"[
                [somebool, someint, somefloat, somefilesize, someduration, somedate, somestring, somebinary, somenull];
                [false, 2, 3.0, 2mb, 4wk, "2020-09-10T12:30:00-00:00", "bar", ("wut" | into binary), null],
            ]"#,
            None,
            // it should have both rows
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
            ],
        );
    });
}

/// Test inserting a good number of randomly generated rows to test an actual
/// streaming pipeline instead of a simple value
#[test]
fn into_sqlite_big_insert() {
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
                        span: Span::unknown(),
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
                nuon::ToStyle::Default,
                Some(Span::unknown()),
                serialize_types,
            )
            .unwrap()
                + &line_ending();

            nuon_file.write_all(nuon.as_bytes()).unwrap();
            expected_rows.push(row);
        }

        insert_test_rows(
            &dirs,
            &format!(
                "open --raw {} | lines | each {{ from nuon }}",
                nuon_path.to_string_lossy()
            ),
            None,
            expected_rows,
        );
    });
}

/// empty in, empty out
#[test]
fn into_sqlite_empty() {
    Playground::setup("empty", |dirs, _| {
        insert_test_rows(&dirs, r#"[]"#, Some("SELECT * FROM sqlite_schema;"), vec![]);
    });
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
        StdRng::from_os_rng().sample(StandardUniform)
    }
}

impl From<TestRow> for Value {
    fn from(row: TestRow) -> Self {
        Value::record(
            record! {
                "somebool" => Value::bool(row.0, Span::unknown()),
                "someint" => Value::int(row.1, Span::unknown()),
                "somefloat" => Value::float(row.2, Span::unknown()),
                "somefilesize" => Value::filesize(row.3, Span::unknown()),
                "someduration" => Value::duration(row.4, Span::unknown()),
                "somedate" => Value::date(row.5, Span::unknown()),
                "somestring" => Value::string(row.6, Span::unknown()),
                "somebinary" => Value::binary(row.7, Span::unknown()),
                "somenull" => Value::nothing(Span::unknown()),
            },
            Span::unknown(),
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
        R: rand::Rng + ?Sized,
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

fn make_sqlite_db(dirs: &Dirs, nu_table: &str) -> AbsolutePathBuf {
    let testdir = dirs.test();
    let testdb_path =
        testdir.join(testdir.file_name().unwrap().to_str().unwrap().to_owned() + ".db");
    let testdb = testdb_path.to_str().unwrap();

    let nucmd = nu!(
        cwd: testdir,
        pipeline(&format!("{nu_table} | into sqlite {testdb}"))
    );

    assert!(nucmd.status.success());
    testdb_path
}

fn insert_test_rows(dirs: &Dirs, nu_table: &str, sql_query: Option<&str>, expected: Vec<TestRow>) {
    let sql_query = sql_query.unwrap_or("SELECT * FROM main;");
    let testdb = make_sqlite_db(dirs, nu_table);

    let conn = rusqlite::Connection::open(testdb).unwrap();
    let mut stmt = conn.prepare(sql_query).unwrap();

    let actual_rows: Vec<_> = stmt
        .query_and_then([], |row| TestRow::try_from(row))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();

    assert_eq!(expected, actual_rows);
}

#[test]
fn test_auto_conversion() {
    Playground::setup("sqlite json auto conversion", |_, playground| {
        let raw = "{a_record:{foo:bar,baz:quux},a_list:[1,2,3],a_table:[[a,b];[0,1],[2,3]]}";
        nu!(cwd: playground.cwd(), "{} | into sqlite filename.db -t my_table", raw);
        let outcome = nu!(
            cwd: playground.cwd(),
            "open filename.db | get my_table.0 | to nuon --raw"
        );
        assert_eq!(outcome.out, raw);
    });
}
