use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::errors::ExpectedRange;
use crate::prelude::*;
use bson::{decode_document, spec::BinarySubtype, Bson};
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

fn bson_array(input: &Vec<Bson>, tag: Tag) -> Result<Vec<Tagged<Value>>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(convert_bson_value_to_nu_value(value, &tag)?);
    }

    Ok(out)
}

fn convert_bson_value_to_nu_value(
    v: &Bson,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, ShellError> {
    let tag = tag.into();

    Ok(match v {
        Bson::FloatingPoint(n) => Value::Primitive(Primitive::from(*n)).tagged(&tag),
        Bson::String(s) => Value::Primitive(Primitive::String(String::from(s))).tagged(&tag),
        Bson::Array(a) => Value::Table(bson_array(a, tag.clone())?).tagged(&tag),
        Bson::Document(doc) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            for (k, v) in doc.iter() {
                collected.insert_tagged(k.clone(), convert_bson_value_to_nu_value(v, &tag)?);
            }

            collected.into_tagged_value()
        }
        Bson::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(&tag),
        Bson::Null => Value::Primitive(Primitive::Nothing).tagged(&tag),
        Bson::RegExp(r, opts) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged(
                "$regex".to_string(),
                Value::Primitive(Primitive::String(String::from(r))).tagged(&tag),
            );
            collected.insert_tagged(
                "$options".to_string(),
                Value::Primitive(Primitive::String(String::from(opts))).tagged(&tag),
            );
            collected.into_tagged_value()
        }
        Bson::I32(n) => Value::number(n).tagged(&tag),
        Bson::I64(n) => Value::number(n).tagged(&tag),
        Bson::Decimal128(n) => {
            // TODO: this really isn't great, and we should update this to do a higher
            // fidelity translation
            let decimal = BigDecimal::from_str(&format!("{}", n)).map_err(|_| {
                ShellError::range_error(
                    ExpectedRange::BigDecimal,
                    &n.tagged(&tag),
                    format!("converting BSON Decimal128 to BigDecimal"),
                )
            })?;
            Value::Primitive(Primitive::Decimal(decimal)).tagged(&tag)
        }
        Bson::JavaScriptCode(js) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged(
                "$javascript".to_string(),
                Value::Primitive(Primitive::String(String::from(js))).tagged(&tag),
            );
            collected.into_tagged_value()
        }
        Bson::JavaScriptCodeWithScope(js, doc) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged(
                "$javascript".to_string(),
                Value::Primitive(Primitive::String(String::from(js))).tagged(&tag),
            );
            collected.insert_tagged(
                "$scope".to_string(),
                convert_bson_value_to_nu_value(&Bson::Document(doc.to_owned()), tag.clone())?,
            );
            collected.into_tagged_value()
        }
        Bson::TimeStamp(ts) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged("$timestamp".to_string(), Value::number(ts).tagged(&tag));
            collected.into_tagged_value()
        }
        Bson::Binary(bst, bytes) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged(
                "$binary_subtype".to_string(),
                match bst {
                    BinarySubtype::UserDefined(u) => Value::number(u),
                    _ => Value::Primitive(Primitive::String(binary_subtype_to_string(*bst))),
                }
                .tagged(&tag),
            );
            collected.insert_tagged(
                "$binary".to_string(),
                Value::Primitive(Primitive::Binary(bytes.to_owned())).tagged(&tag),
            );
            collected.into_tagged_value()
        }
        Bson::ObjectId(obj_id) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged(
                "$object_id".to_string(),
                Value::Primitive(Primitive::String(obj_id.to_hex())).tagged(&tag),
            );
            collected.into_tagged_value()
        }
        Bson::UtcDatetime(dt) => Value::Primitive(Primitive::Date(*dt)).tagged(&tag),
        Bson::Symbol(s) => {
            let mut collected = TaggedDictBuilder::new(tag.clone());
            collected.insert_tagged(
                "$symbol".to_string(),
                Value::Primitive(Primitive::String(String::from(s))).tagged(&tag),
            );
            collected.into_tagged_value()
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

pub fn from_bson_bytes_to_value(
    bytes: Vec<u8>,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, ShellError> {
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
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        for value in values {
            let value_tag = value.tag();
            match value.item {
                Value::Primitive(Primitive::Binary(vb)) =>
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
