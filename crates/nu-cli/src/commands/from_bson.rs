use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use bson::{decode_document, spec::BinarySubtype, Bson};
use nu_errors::{ExpectedRange, ShellError};
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::SpannedItem;
use std::str::FromStr;

pub struct FromBSON;

impl WholeStreamCommand for FromBSON {
    fn name(&self) -> &str {
        "from-bson"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-bson")
    }

    fn usage(&self) -> &str {
        "Parse text as .bson and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_bson(args, registry)
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
        Bson::FloatingPoint(n) => UntaggedValue::Primitive(Primitive::from(*n)).into_value(&tag),
        Bson::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag)
        }
        Bson::Array(a) => UntaggedValue::Table(bson_array(a, tag.clone())?).into_value(&tag),
        Bson::Document(doc) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            for (k, v) in doc.iter() {
                collected.insert_value(k.clone(), convert_bson_value_to_nu_value(v, &tag)?);
            }

            collected.into_value()
        }
        Bson::Boolean(b) => UntaggedValue::Primitive(Primitive::Boolean(*b)).into_value(&tag),
        Bson::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(&tag),
        Bson::RegExp(r, opts) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$regex".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(r))).into_value(&tag),
            );
            collected.insert_value(
                "$options".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(opts))).into_value(&tag),
            );
            collected.into_value()
        }
        Bson::I32(n) => UntaggedValue::int(*n).into_value(&tag),
        Bson::I64(n) => UntaggedValue::int(*n).into_value(&tag),
        Bson::Decimal128(n) => {
            // TODO: this really isn't great, and we should update this to do a higher
            // fidelity translation
            let decimal = BigDecimal::from_str(&format!("{}", n)).map_err(|_| {
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
        Bson::JavaScriptCodeWithScope(js, doc) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$javascript".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(js))).into_value(&tag),
            );
            collected.insert_value(
                "$scope".to_string(),
                convert_bson_value_to_nu_value(&Bson::Document(doc.to_owned()), tag)?,
            );
            collected.into_value()
        }
        Bson::TimeStamp(ts) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$timestamp".to_string(),
                UntaggedValue::int(*ts).into_value(&tag),
            );
            collected.into_value()
        }
        Bson::Binary(bst, bytes) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$binary_subtype".to_string(),
                match bst {
                    BinarySubtype::UserDefined(u) => UntaggedValue::int(*u),
                    _ => {
                        UntaggedValue::Primitive(Primitive::String(binary_subtype_to_string(*bst)))
                    }
                }
                .into_value(&tag),
            );
            collected.insert_value(
                "$binary".to_string(),
                UntaggedValue::Primitive(Primitive::Binary(bytes.to_owned())).into_value(&tag),
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
        Bson::UtcDatetime(dt) => UntaggedValue::Primitive(Primitive::Date(*dt)).into_value(&tag),
        Bson::Symbol(s) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_value(
                "$symbol".to_string(),
                UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag),
            );
            collected.into_value()
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
    while let Ok(v) = decode_document(&mut b_reader) {
        docs.push(Bson::Document(v));
    }

    convert_bson_value_to_nu_value(&Bson::Array(docs), tag)
}

fn from_bson(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        for value in values {
            let value_tag = &value.tag;
            match value.value {
                UntaggedValue::Primitive(Primitive::Binary(vb)) =>
                    match from_bson_bytes_to_value(vb, tag.clone()) {
                        Ok(x) => yield ReturnSuccess::value(x),
                        Err(_) => {
                            yield Err(ShellError::labeled_error_with_secondary(
                                "Could not parse as BSON",
                                "input cannot be parsed as BSON",
                                tag.clone(),
                                "value originates from here",
                                value_tag,
                            ))
                        }
                    }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    tag.clone(),
                    "value originates from here",
                    value_tag,
                )),

            }
        }
    };

    Ok(stream.to_output_stream())
}
