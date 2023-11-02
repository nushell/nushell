use crate::{record, Config, Span, Value};

pub(super) fn reconstruct_external_completer(config: &Config, span: Span) -> Value {
    if let Some(block) = config.external_completer {
        Value::block(block, span)
    } else {
        Value::nothing(span)
    }
}

pub(super) fn reconstruct_external(config: &Config, span: Span) -> Value {
    Value::record(
        record! {
            "max_results" => Value::int(config.max_external_completion_results, span),
            "completer" => reconstruct_external_completer(config, span),
            "enable" => Value::bool(config.enable_external_completion, span),
        },
        span,
    )
}
