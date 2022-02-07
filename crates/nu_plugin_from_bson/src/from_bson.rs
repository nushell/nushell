use bigdecimal::BigDecimal;
use bson::{spec::BinarySubtype, Bson};
use nu_errors::{ExpectedRange, ShellError};
use nu_protocol::{Primitive, ReturnSuccess, ReturnValue, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{SpannedItem, Tag};
use std::str::FromStr;

#[derive(Default)]
pub struct FromBson {
    pub state: Vec<u8>,
    pub name_tag: Tag,
}

impl FromBson {
    pub fn new() -> FromBson {
        FromBson {
            state: vec![],
            name_tag: Tag::unknown(),
        }
    }
}

fn bson_array(input: &[Bson], tag: Tag) -> Result<Vec<Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(convert_bson_value_to_nu_value(value, &tag)?);
    }

    Ok(out)
}

fn convert_bson_value_to_nu_value(v: &Bson, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let tag = tag.into();
    let span = tag.span;

    Ok(match v {
        Bson::Double(n) => UntaggedValue::Primitive(Primitive::from(*n)).into_value(&tag),
        Bson::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag)
        }
        Bson::Array(a) => UntaggedValue::Table(bson_array(a, tag.clone())?).into_value(&tag),
        Bson::Document(doc) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            for (k, v) in doc {
                collected.insert_value(k.clone(), convert_bson_value_to_nu_value(v, &tag)?);
            }

            collected.into_value()
        }
        Bson::Boolean(b) => UntaggedValue::Primitive(Primitive::Boolean(*b)).into_value(&tag),
        Bson::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(&tag),
        Bson::RegularExpression(regx) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$regex".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(&regx.pattern)))
                    .into_value(&tag),
            );
            collected.insert_value(
                "$options".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(&regx.options)))
                    .into_value(&tag),
            );
            collected.into_value()
        }
        Bson::Int32(n) => UntaggedValue::int(*n).into_value(&tag),
        Bson::Int64(n) => UntaggedValue::int(*n).into_value(&tag),
        Bson::Decimal128(n) => {
            // TODO: this really isn't great, and we should update this to do a higher
            // fidelity translation
            let decimal = BigDecimal::from_str(&n.to_string()).map_err(|_| {
                ShellError::range_error(
                    ExpectedRange::BigDecimal,
                    &n.spanned(span),
                    "converting BSON Decimal128 to BigDecimal".to_owned(),
                )
            })?;
            UntaggedValue::Primitive(Primitive::Decimal(decimal)).into_value(&tag)
        }
        Bson::JavaScriptCode(js) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$javascript".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(js))).into_value(&tag),
            );
            collected.into_value()
        }
        Bson::JavaScriptCodeWithScope(js) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$javascript".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(&js.code)))
                    .into_value(&tag),
            );
            collected.insert_value(
                "$scope".to_string(),
                convert_bson_value_to_nu_value(&Bson::Document(js.scope.to_owned()), tag)?,
            );
            collected.into_value()
        }
        Bson::Timestamp(ts) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$timestamp".to_string(),
                UntaggedValue::int(ts.time).into_value(&tag),
            );
            collected.into_value()
        }
        Bson::Binary(binary) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$binary_subtype".to_string(),
                match binary.subtype {
                    BinarySubtype::UserDefined(u) => UntaggedValue::int(u),
                    _ => UntaggedValue::Primitive(Primitive::String(binary_subtype_to_string(
                        binary.subtype,
                    ))),
                }
                .into_value(&tag),
            );
            collected.insert_value(
                "$binary".to_string(),
                UntaggedValue::Primitive(Primitive::Binary(binary.bytes.to_owned()))
                    .into_value(&tag),
            );
            collected.into_value()
        }
        Bson::ObjectId(obj_id) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$object_id".to_string(),
                UntaggedValue::Primitive(Primitive::String(obj_id.to_hex())).into_value(&tag),
            );
            collected.into_value()
        }
        Bson::DateTime(dt) => {
            UntaggedValue::Primitive(Primitive::Date(dt.to_chrono().into())).into_value(&tag)
        }
        Bson::Symbol(s) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$symbol".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag),
            );
            collected.into_value()
        }
        Bson::Undefined | Bson::MaxKey | Bson::MinKey | Bson::DbPointer(_) => {
            // TODO  Impelmenting Bson::Undefined, Bson::MaxKey, Bson::MinKey and Bson::DbPointer
            // These Variants weren't present in the previous version.
            TaggedDictBuilder::new(tag).into_value()
        }
    })
}

fn binary_subtype_to_string(bst: BinarySubtype) -> String {
    match bst {
        BinarySubtype::Generic => "generic",
        BinarySubtype::Function => "function",
        BinarySubtype::BinaryOld => "binary_old",
        BinarySubtype::UuidOld => "uuid_old",
        BinarySubtype::Uuid => "uuid",
        BinarySubtype::Md5 => "md5",
        _ => unreachable!(),
    }
    .to_string()
}

#[derive(Debug)]
struct BytesReader {
    pos: usize,
    inner: Vec<u8>,
}

impl BytesReader {
    fn new(bytes: Vec<u8>) -> BytesReader {
        BytesReader {
            pos: 0,
            inner: bytes,
        }
    }
}

impl std::io::Read for BytesReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let src: &mut &[u8] = &mut self.inner[self.pos..].as_ref();
        let diff = src.read(buf)?;
        self.pos += diff;
        Ok(diff)
    }
}

pub fn from_bson_bytes_to_value(bytes: Vec<u8>, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    let mut docs = Vec::new();
    let mut b_reader = BytesReader::new(bytes);
    while let Ok(v) = bson::de::from_reader(&mut b_reader) {
        docs.push(Bson::Document(v));
    }

    convert_bson_value_to_nu_value(&Bson::Array(docs), tag)
}

pub fn from_bson(bytes: Vec<u8>, name_tag: Tag) -> Result<Vec<ReturnValue>, ShellError> {
    match from_bson_bytes_to_value(bytes, name_tag.clone()) {
        Ok(x) => Ok(vec![ReturnSuccess::value(x)]),
        Err(_) => Err(ShellError::labeled_error(
            "Could not parse as BSON",
            "input cannot be parsed as BSON",
            name_tag,
        )),
    }
}
