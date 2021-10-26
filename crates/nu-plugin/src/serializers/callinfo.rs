use super::{call, value};
use crate::value_capnp::call_info;
use capnp::serialize_packed;
use nu_protocol::{ast::Call, ShellError, Value};

pub fn write_buffer(
    call: &Call,
    input: &Value,
    writer: &mut impl std::io::Write,
) -> Result<(), ShellError> {
    let mut message = ::capnp::message::Builder::new_default();

    let mut builder = message.init_root::<call_info::Builder>();
    let value_builder = builder
        .reborrow()
        .get_input()
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    value::serialize_value(input, value_builder);

    let call_builder = builder
        .reborrow()
        .get_call()
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    call::serialize_call(call, call_builder)
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    serialize_packed::write_message(writer, &message)
        .map_err(|e| ShellError::EncodingError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{
        ast::{Call, Expr, Expression},
        Span, Spanned, Value,
    };

    #[test]
    fn callinfo_round_trip() {
        let value = Value::Bool {
            val: false,
            span: Span { start: 1, end: 20 },
        };

        let call = Call {
            decl_id: 1,
            head: Span { start: 0, end: 10 },
            positional: vec![
                Expression {
                    expr: Expr::Float(1.0),
                    span: Span { start: 0, end: 10 },
                    ty: nu_protocol::Type::Float,
                    custom_completion: None,
                },
                Expression {
                    expr: Expr::String("something".into()),
                    span: Span { start: 0, end: 10 },
                    ty: nu_protocol::Type::Float,
                    custom_completion: None,
                },
            ],
            named: vec![(
                Spanned {
                    item: "name".to_string(),
                    span: Span { start: 0, end: 10 },
                },
                Some(Expression {
                    expr: Expr::Float(1.0),
                    span: Span { start: 0, end: 10 },
                    ty: nu_protocol::Type::Float,
                    custom_completion: None,
                }),
            )],
        };

        let mut buffer: Vec<u8> = Vec::new();
        write_buffer(&call, &value, &mut buffer).expect("unable to serialize message");
        println!("{:?}", buffer);
    }
}
