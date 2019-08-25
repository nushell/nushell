use crate::commands::WholeStreamCommand;
use crate::object::base::OF64;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use bson::{decode_document, Bson, spec::BinarySubtype};

pub struct FromBSON;

impl WholeStreamCommand for FromBSON {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_bson(args, registry)
    }

    fn name(&self) -> &str {
        "from-bson"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-bson")
    }
}

fn convert_bson_value_to_nu_value(v: &Bson, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    match v {
        Bson::FloatingPoint(n) => Value::Primitive(Primitive::Float(OF64::from(*n))).tagged(tag),
        Bson::String(s) => Value::Primitive(Primitive::String(String::from(s))).tagged(tag),
        Bson::Array(a) => Value::List(
            a.iter()
                .map(|x| convert_bson_value_to_nu_value(x, tag))
                .collect(),
        )
        .tagged(tag),
        Bson::Document(doc) => {
            let mut collected = TaggedDictBuilder::new(tag);
            for (k, v) in doc.iter() {
                collected.insert_tagged(k.clone(), convert_bson_value_to_nu_value(v, tag));
            }

            collected.into_tagged_value()
        }
        Bson::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)).tagged(tag),
        Bson::Null => Value::Primitive(Primitive::String(String::from(""))).tagged(tag),
        Bson::RegExp(r, opts) => {
             let mut collected = TaggedDictBuilder::new(tag);
             collected.insert_tagged(
                 "$regex".to_string(),
                 Value::Primitive(Primitive::String(String::from(r))).tagged(tag),
             );
             collected.insert_tagged(
                 "$options".to_string(),
                 Value::Primitive(Primitive::String(String::from(opts))).tagged(tag),
             );
             collected.into_tagged_value()
        }
        Bson::I32(n) => Value::Primitive(Primitive::Int(*n as i64)).tagged(tag),
        Bson::I64(n) => Value::Primitive(Primitive::Int(*n as i64)).tagged(tag),
        Bson::JavaScriptCode(js) => {
             let mut collected = TaggedDictBuilder::new(tag);
             collected.insert_tagged(
                 "$javascript".to_string(),
                 Value::Primitive(Primitive::String(String::from(js))).tagged(tag),
             );
             collected.into_tagged_value()
        }
        Bson::JavaScriptCodeWithScope(js, doc) => {
             let mut collected = TaggedDictBuilder::new(tag);
             collected.insert_tagged(
                 "$javascript".to_string(),
                 Value::Primitive(Primitive::String(String::from(js))).tagged(tag),
             );
             collected.insert_tagged(
                 "$scope".to_string(),
                 convert_bson_value_to_nu_value(&Bson::Document(doc.to_owned()), tag),
             );
             collected.into_tagged_value()
        }
        Bson::TimeStamp(ts) => {
             let mut collected = TaggedDictBuilder::new(tag);
             collected.insert_tagged(
                 "$timestamp".to_string(),
                 Value::Primitive(Primitive::Int(*ts as i64)).tagged(tag),
             );
             collected.into_tagged_value()
        }
        Bson::Binary(bst, bytes) => {
             let mut collected = TaggedDictBuilder::new(tag);
             collected.insert_tagged(
                 "$binary_subtype".to_string(),
                 match bst {
                     BinarySubtype::UserDefined(u) => Value::Primitive(Primitive::Int(*u as i64)),
                     _ => Value::Primitive(Primitive::String(binary_subtype_to_string(*bst))),
                 }.tagged(tag)
             );
             collected.insert_tagged(
                 "$binary".to_string(),
                 Value::Binary(bytes.to_owned()).tagged(tag),
             );
             collected.into_tagged_value()
        }
        Bson::ObjectId(obj_id) => Value::Primitive(Primitive::String(obj_id.to_hex())).tagged(tag),
        Bson::UtcDatetime(dt) => Value::Primitive(Primitive::Date(*dt)).tagged(tag),
        Bson::Symbol(s) => {
             let mut collected = TaggedDictBuilder::new(tag);
             collected.insert_tagged(
                 "$symbol".to_string(),
                 Value::Primitive(Primitive::String(String::from(s))).tagged(tag),
             );
             collected.into_tagged_value()
        }
    }
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
    }.to_string()
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
) -> bson::DecoderResult<Tagged<Value>> {
    let mut docs = Vec::new();
    let mut b_reader = BytesReader::new(bytes);
    while let Ok(v) = decode_document(&mut b_reader) {
        docs.push(Bson::Document(v));
    }
    Ok(convert_bson_value_to_nu_value(&Bson::Array(docs), tag))
}

fn from_bson(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let input = args.input;

    let stream = async_stream_block! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        for value in values {
            let value_tag = value.tag();
            let latest_tag = Some(value_tag);
            match value.item {
                Value::Binary(vb) =>
                    match from_bson_bytes_to_value(vb, span) {
                        Ok(x) => yield ReturnSuccess::value(x),
                        Err(_) => if let Some(last_tag) = latest_tag {
                            yield Err(ShellError::labeled_error_with_secondary(
                                "Could not parse as BSON",
                                "input cannot be parsed as BSON",
                                span,
                                "value originates from here",
                                last_tag.span,
                            ))
                        }
                    }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    value_tag.span,
                )),

            }
        }
    };

    Ok(stream.to_output_stream())
}
