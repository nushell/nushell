use criterion::{criterion_group, criterion_main, Criterion};
use nu_cli::eval_source;
use nu_protocol::{PipelineData, Span, Value};
use nu_utils::{get_default_config, get_default_env};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("eval default_env.nu", |b| {
        b.iter(|| {
            let mut engine_state = nu_command::create_default_context();
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_env().as_bytes(),
                "default_env.nu",
                PipelineData::empty(),
            )
        })
    });

    c.bench_function("eval default_config.nu", |b| {
        b.iter(|| {
            let mut engine_state = nu_command::create_default_context();
            // parsing config.nu breaks without PWD set
            engine_state.add_env_var(
                "PWD".into(),
                Value::string("/some/dir".to_string(), Span::test_data()),
            );
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_config().as_bytes(),
                "default_config.nu",
                PipelineData::empty(),
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
