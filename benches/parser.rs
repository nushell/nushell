use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use nu_parser::parse;
use nu_protocol::{Span, Value};
use nu_utils::{get_default_config, get_default_env};

fn criterion_benchmark(c: &mut Criterion) {
    let mut engine_state = nu_command::create_default_context();
    // parsing breaks without PWD set
    engine_state.add_env_var(
        "PWD".into(),
        Value::string(
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_owned(),
            Span::test_data(),
        ),
    );

    let default_config = get_default_config().as_bytes();
    c.bench_function("parse config.nu", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| parse(&mut working_set, None, default_config, false, &[]),
            BatchSize::SmallInput,
        )
    });

    let default_env = get_default_env().as_bytes();
    c.bench_function("parse env.nu", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| parse(&mut working_set, None, default_env, false, &[]),
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
