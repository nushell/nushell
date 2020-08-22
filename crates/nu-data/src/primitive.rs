use nu_protocol::{hir::Number, Primitive};
use nu_table::TextStyle;

pub fn number(number: impl Into<Number>) -> Primitive {
    let number = number.into();

    match number {
        Number::Int(int) => Primitive::Int(int),
        Number::Decimal(decimal) => Primitive::Decimal(decimal),
    }
}

pub fn style_primitive(primitive: &Primitive) -> TextStyle {
    match primitive {
        Primitive::Int(_) | Primitive::Filesize(_) | Primitive::Decimal(_) => {
            TextStyle::basic_right()
        }
        _ => TextStyle::basic(),
    }
}
