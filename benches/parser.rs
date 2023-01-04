use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nu_parser::parse;
use nu_protocol::{Value, Span};
use nu_utils::{get_default_config, get_default_env};

fn criterion_benchmark(c: &mut Criterion) {
    let mut engine_state = nu_command::create_default_context();
    // parsing breaks without PWD set
    engine_state.add_env_var("PWD".into(),  Value::string(std::env::current_dir().unwrap().to_string_lossy().to_owned(), Span::test_data()));
    let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
    let default_config = get_default_config().as_bytes();

    c.bench_function("parse config", |b| b.iter(|| {
        nu_parser::parse(&mut working_set, None, default_config, false, &[]);
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);