use super::{call, value};
use crate::value_capnp::call_info;
use capnp::serialize_packed;
use nu_protocol::{ast::Call, ShellError, Value};

#[derive(Debug)]
pub struct CallInfo {
    pub call: Call,
    pub input: Value,
}

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

pub fn read_buffer(reader: &mut impl std::io::BufRead) -> Result<CallInfo, ShellError> {
    let message_reader =
        serialize_packed::read_message(reader, ::capnp::message::ReaderOptions::new()).unwrap();

    let reader = message_reader
        .get_root::<call_info::Reader>()
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    let call_reader = reader
        .get_call()
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    let call = call::deserialize_call(call_reader)
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    let value_reader = reader
        .get_input()
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    let input = value::deserialize_value(value_reader)
        .map_err(|e| ShellError::DecodingError(e.to_string()))?;

    Ok(CallInfo { call, input })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{
        ast::{Call, Expr, Expression},
        Span, Spanned, Value,
    };

    fn compare_expressions(lhs: &Expression, rhs: &Expression) {
        match (&lhs.expr, &rhs.expr) {
            (Expr::Bool(a), Expr::Bool(b)) => assert_eq!(a, b),
            (Expr::Int(a), Expr::Int(b)) => assert_eq!(a, b),
            (Expr::Float(a), Expr::Float(b)) => assert_eq!(a, b),
            (Expr::String(a), Expr::String(b)) => assert_eq!(a, b),
            _ => panic!("not matching values"),
        }
    }

    #[test]
    fn callinfo_round_trip() {
        let input = Value::Bool {
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
        write_buffer(&call, &input, &mut buffer).expect("unable to serialize message");
        let call_info = read_buffer(&mut buffer.as_slice()).expect("unable to read message");

        assert_eq!(input, call_info.input);
        assert_eq!(call.head, call_info.call.head);
        assert_eq!(call.positional.len(), call_info.call.positional.len());

        call.positional
            .iter()
            .zip(call_info.call.positional.iter())
            .for_each(|(lhs, rhs)| compare_expressions(lhs, rhs));

        call.named
            .iter()
            .zip(call_info.call.named.iter())
            .for_each(|(lhs, rhs)| {
                // Comparing the keys
                assert_eq!(lhs.0.item, rhs.0.item);

                match (&lhs.1, &rhs.1) {
                    (None, None) => {}
                    (Some(a), Some(b)) => compare_expressions(a, b),
                    _ => panic!("not matching values"),
                }
            });
    }
}
