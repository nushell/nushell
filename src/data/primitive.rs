use chrono_humanize::Humanize;
use nu_parser::Number;
use nu_protocol::Primitive;
use nu_source::PrettyDebug;

pub fn number(number: impl Into<Number>) -> Primitive {
    let number = number.into();

    match number {
        Number::Int(int) => Primitive::Int(int),
        Number::Decimal(decimal) => Primitive::Decimal(decimal),
    }
}

pub fn format_primitive(primitive: &Primitive, field_name: Option<&String>) -> String {
    match primitive {
        Primitive::Nothing => String::new(),
        Primitive::BeginningOfStream => String::new(),
        Primitive::EndOfStream => String::new(),
        Primitive::Path(p) => format!("{}", p.display()),
        Primitive::Bytes(b) => {
            let byte = byte_unit::Byte::from_bytes(*b as u128);

            if byte.get_bytes() == 0u128 {
                return "—".to_string();
            }

            let byte = byte.get_appropriate_unit(false);

            match byte.get_unit() {
                byte_unit::ByteUnit::B => format!("{} B ", byte.get_value()),
                _ => format!("{}", byte.format(1)),
            }
        }
        Primitive::Duration(sec) => format_duration(*sec),
        Primitive::Int(i) => format!("{}", i),
        Primitive::Decimal(decimal) => format!("{}", decimal),
        Primitive::Pattern(s) => format!("{}", s),
        Primitive::String(s) => format!("{}", s),
        Primitive::ColumnPath(p) => {
            let mut members = p.iter();
            let mut f = String::new();

            f.push_str(
                &members
                    .next()
                    .expect("BUG: column path with zero members")
                    .display(),
            );

            for member in members {
                f.push_str(".");
                f.push_str(&member.display())
            }

            f
        }
        Primitive::Boolean(b) => match (b, field_name) {
            (true, None) => format!("Yes"),
            (false, None) => format!("No"),
            (true, Some(s)) if !s.is_empty() => format!("{}", s),
            (false, Some(s)) if !s.is_empty() => format!(""),
            (true, Some(_)) => format!("Yes"),
            (false, Some(_)) => format!("No"),
        },
        Primitive::Binary(_) => format!("<binary>"),
        Primitive::Date(d) => format!("{}", d.humanize()),
    }
}

pub fn style_primitive(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::Bytes(0) => "c", // centre 'missing' indicator
        Primitive::Int(_) | Primitive::Bytes(_) | Primitive::Decimal(_) => "r",
        _ => "",
    }
}

fn format_duration(sec: u64) -> String {
    let (minutes, seconds) = (sec / 60, sec % 60);
    let (hours, minutes) = (minutes / 60, minutes % 60);
    let (days, hours) = (hours / 24, hours % 24);

    match (days, hours, minutes, seconds) {
        (0, 0, 0, 1) => format!("1 sec"),
        (0, 0, 0, s) => format!("{} secs", s),
        (0, 0, m, s) => format!("{}:{:02}", m, s),
        (0, h, m, s) => format!("{}:{:02}:{:02}", h, m, s),
        (d, h, m, s) => format!("{}:{:02}:{:02}:{:02}", d, h, m, s),
    }
}
