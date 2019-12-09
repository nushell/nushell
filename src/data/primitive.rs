use nu_parser::Number;
use nu_protocol::Primitive;

pub fn number(number: impl Into<Number>) -> Primitive {
    let number = number.into();

    match number {
        Number::Int(int) => Primitive::Int(int),
        Number::Decimal(decimal) => Primitive::Decimal(decimal),
    }
}

pub fn style_primitive(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::Bytes(0) => "c", // centre 'missing' indicator
        Primitive::Int(_) | Primitive::Bytes(_) | Primitive::Decimal(_) => "r",
        _ => "",
    }
}
