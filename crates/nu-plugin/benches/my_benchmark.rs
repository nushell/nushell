use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nu_plugin::{EncodingType, PluginResponse};
use nu_protocol::{Span, Value};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

// generate a new table data with `row_cnt` rows, `col_cnt` columns.
fn new_test_data(row_cnt: usize, col_cnt: usize) -> Value {
    let columns: Vec<String> = (0..col_cnt).map(|x| format!("col_{x}")).collect();
    let vals: Vec<Value> = (0..col_cnt as i64).map(|i| Value::test_int(i)).collect();

    Value::List {
        vals: (0..row_cnt)
            .map(|_| Value::test_record(columns.clone(), vals.clone()))
            .collect(),
        span: Span::test_data(),
    }
}

fn json_encode_response(c: &mut Criterion) {
    let encoder = EncodingType::try_from_bytes(b"json").unwrap();
    let test_cnt_pairs = [
        (100, 5),
        (100, 10),
        (100, 15),
        (1000, 5),
        (1000, 10),
        (1000, 15),
        (10000, 5),
        (10000, 10),
        (10000, 15),
    ];

    for (row_cnt, col_cnt) in test_cnt_pairs {
        let bench_name = format!("json encode for {row_cnt} * {col_cnt}");
        c.bench_function(&bench_name, |b| {
            let mut res = vec![];
            let test_data = PluginResponse::Value(Box::new(new_test_data(row_cnt, col_cnt)));
            b.iter(|| encoder.encode_response(&test_data, &mut res))
        });
    }
}

fn json_decode_response(c: &mut Criterion) {
    let encoder = EncodingType::try_from_bytes(b"json").unwrap();
    let test_cnt_pairs = [
        (100, 5),
        (100, 10),
        (100, 15),
        (1000, 5),
        (1000, 10),
        (1000, 15),
        (10000, 5),
        (10000, 10),
        (10000, 15),
    ];

    for (row_cnt, col_cnt) in test_cnt_pairs {
        let bench_name = format!("json decode for {row_cnt} * {col_cnt}");
        c.bench_function(&bench_name, |b| {
            let mut res = vec![];
            let test_data = PluginResponse::Value(Box::new(new_test_data(row_cnt, col_cnt)));
            encoder.encode_response(&test_data, &mut res).unwrap();
            let mut binary_data = std::io::Cursor::new(res);
            b.iter(|| {
                binary_data.set_position(0);
                encoder.decode_response(&mut binary_data)
            })
        });
    }
}

fn capnp_encode_response(c: &mut Criterion) {
    let encoder = EncodingType::try_from_bytes(b"capnp").unwrap();
    let test_cnt_pairs = [
        (100, 5),
        (100, 10),
        (100, 15),
        (1000, 5),
        (1000, 10),
        (1000, 15),
        (10000, 5),
        (10000, 10),
        (10000, 15),
    ];

    for (row_cnt, col_cnt) in test_cnt_pairs {
        let bench_name = format!("capnp encode for {row_cnt} * {col_cnt}");
        c.bench_function(&bench_name, |b| {
            let mut res = vec![];
            let test_data = PluginResponse::Value(Box::new(new_test_data(row_cnt, col_cnt)));
            b.iter(|| encoder.encode_response(&test_data, &mut res))
        });
    }
}

fn capnp_decode_response(c: &mut Criterion) {
    let encoder = EncodingType::try_from_bytes(b"capnp").unwrap();
    let test_cnt_pairs = [
        (100, 5),
        (100, 10),
        (100, 15),
        (1000, 5),
        (1000, 10),
        (1000, 15),
        (10000, 5),
        (10000, 10),
        (10000, 15),
    ];

    for (row_cnt, col_cnt) in test_cnt_pairs {
        let bench_name = format!("json decode for {row_cnt} * {col_cnt}");
        c.bench_function(&bench_name, |b| {
            let mut res = vec![];
            let test_data = PluginResponse::Value(Box::new(new_test_data(row_cnt, col_cnt)));
            encoder.encode_response(&test_data, &mut res).unwrap();
            let mut binary_data = std::io::Cursor::new(res);
            b.iter(|| {
                binary_data.set_position(0);
                encoder.decode_response(&mut binary_data)
            })
        });
    }
}
/*
fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}
*/

criterion_group!(
    benches,
    json_encode_response,
    json_decode_response,
    capnp_encode_response,
    capnp_decode_response
);
criterion_main!(benches);
