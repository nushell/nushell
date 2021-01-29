use crate::type_name::PrettyType;
use crate::value::primitive::Primitive;
use crate::value::{UntaggedValue, Value};
use nu_source::{DbgDocBldr, DebugDocBuilder, PrettyDebug};

impl PrettyDebug for &Value {
    /// Get a borrowed Value ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        PrettyDebug::pretty(*self)
    }
}

impl PrettyDebug for Value {
    /// Get a Value ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        match &self.value {
            UntaggedValue::Primitive(p) => p.pretty(),
            UntaggedValue::Row(row) => row.pretty_builder().nest(1).group().into(),
            UntaggedValue::Table(table) => DbgDocBldr::delimit(
                "[",
                DbgDocBldr::intersperse(table, DbgDocBldr::space()),
                "]",
            )
            .nest(),
            UntaggedValue::Error(_) => DbgDocBldr::error("error"),
            UntaggedValue::Block(_) => DbgDocBldr::opaque("block"),
        }
    }
}

impl PrettyType for Primitive {
    /// Find the type of the Value and prepare it for pretty-printing
    fn pretty_type(&self) -> DebugDocBuilder {
        match self {
            Primitive::Nothing => ty("nothing"),
            Primitive::Int(_) => ty("integer"),
            Primitive::Range(_) => ty("range"),
            Primitive::Decimal(_) => ty("decimal"),
            Primitive::Filesize(_) => ty("filesize"),
            Primitive::String(_) => ty("string"),
            Primitive::ColumnPath(_) => ty("column-path"),
            Primitive::GlobPattern(_) => ty("pattern"),
            Primitive::Boolean(_) => ty("boolean"),
            Primitive::Date(_) => ty("date"),
            Primitive::Duration(_) => ty("duration"),
            Primitive::FilePath(_) => ty("path"),
            Primitive::Binary(_) => ty("binary"),
            Primitive::BeginningOfStream => DbgDocBldr::keyword("beginning-of-stream"),
            Primitive::EndOfStream => DbgDocBldr::keyword("end-of-stream"),
        }
    }
}

impl PrettyDebug for Primitive {
    /// Get a Primitive value ready to be pretty-printed
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            Primitive::Nothing => DbgDocBldr::primitive("nothing"),
            Primitive::Int(int) => prim(format_args!("{}", int)),
            Primitive::Decimal(decimal) => prim(format_args!("{}", decimal)),
            Primitive::Range(range) => {
                let (left, left_inclusion) = &range.from;
                let (right, right_inclusion) = &range.to;

                DbgDocBldr::typed(
                    "range",
                    (left_inclusion.debug_left_bracket()
                        + left.pretty()
                        + DbgDocBldr::operator(",")
                        + DbgDocBldr::space()
                        + right.pretty()
                        + right_inclusion.debug_right_bracket())
                    .group(),
                )
            }
            Primitive::Filesize(bytes) => primitive_doc(bytes, "filesize"),
            Primitive::String(string) => prim(string),
            Primitive::ColumnPath(path) => path.pretty(),
            Primitive::GlobPattern(pattern) => primitive_doc(pattern, "pattern"),
            Primitive::Boolean(boolean) => match boolean {
                true => DbgDocBldr::primitive("$yes"),
                false => DbgDocBldr::primitive("$no"),
            },
            Primitive::Date(date) => primitive_doc(date, "date"),
            Primitive::Duration(duration) => primitive_doc(duration, "nanoseconds"),
            Primitive::FilePath(path) => primitive_doc(path, "path"),
            Primitive::Binary(_) => DbgDocBldr::opaque("binary"),
            Primitive::BeginningOfStream => DbgDocBldr::keyword("beginning-of-stream"),
            Primitive::EndOfStream => DbgDocBldr::keyword("end-of-stream"),
        }
    }
}

fn prim(name: impl std::fmt::Debug) -> DebugDocBuilder {
    DbgDocBldr::primitive(format!("{:?}", name))
}

fn primitive_doc(name: impl std::fmt::Debug, ty: impl Into<String>) -> DebugDocBuilder {
    DbgDocBldr::primitive(format!("{:?}", name))
        + DbgDocBldr::delimit("(", DbgDocBldr::kind(ty.into()), ")")
}

fn ty(name: impl std::fmt::Debug) -> DebugDocBuilder {
    DbgDocBldr::kind(format!("{:?}", name))
}
