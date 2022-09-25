use criterion::{criterion_group, criterion_main, Criterion};
use nu_plugin::{EncodingType, PluginResponse};
use nu_protocol::{Span, Value};

// generate a new table data with `row_cnt` rows, `col_cnt` columns.
fn new_test_data(row_cnt: usize, col_cnt: usize) -> Value {
    let columns: Vec<String> = (0..col_cnt).map(|x| format!("col_{x}")).collect();
    let vals: Vec<Value> = (0..col_cnt as i64).map(Value::test_int).collect();

    Value::List {
        vals: (0..row_cnt)
            .map(|_| Value::test_record(columns.clone(), vals.clone()))
            .collect(),
        span: Span::test_data(),
    }
}

fn bench_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encoding");
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
    for (row_cnt, col_cnt) in test_cnt_pairs.into_iter() {
        for fmt in ["json", "msgpack"] {
            group.bench_function(&format!("{fmt} encode {row_cnt} * {col_cnt}"), |b| {
                let mut res = vec![];
                let test_data = PluginResponse::Value(Box::new(new_test_data(row_cnt, col_cnt)));
                let encoder = EncodingType::try_from_bytes(fmt.as_bytes()).unwrap();
                b.iter(|| encoder.encode_response(&test_data, &mut res))
            });
        }
    }
    group.finish();
}

fn bench_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("Decoding");
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
    for (row_cnt, col_cnt) in test_cnt_pairs.into_iter() {
        for fmt in ["json", "msgpack"] {
            group.bench_function(&format!("{fmt} decode for {row_cnt} * {col_cnt}"), |b| {
                let mut res = vec![];
                let test_data = PluginResponse::Value(Box::new(new_test_data(row_cnt, col_cnt)));
                let encoder = EncodingType::try_from_bytes(fmt.as_bytes()).unwrap();
                encoder.encode_response(&test_data, &mut res).unwrap();
                let mut binary_data = std::io::Cursor::new(res);
                b.iter(|| {
                    binary_data.set_position(0);
                    encoder.decode_response(&mut binary_data)
                })
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_encoding, bench_decoding);
criterion_main!(benches);
