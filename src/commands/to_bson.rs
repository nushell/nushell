use crate::commands::WholeStreamCommand;
use crate::data::{Dictionary, Primitive, Value};
use crate::prelude::*;
use crate::RawPathMember;
use bson::{encode_document, oid::ObjectId, spec::BinarySubtype, Bson, Document};
use std::convert::TryInto;

pub struct ToBSON;

impl WholeStreamCommand for ToBSON {
    fn name(&self) -> &str {
        "to-bson"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-bson")
    }

    fn usage(&self) -> &str {
        "Convert table into .bson text."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_bson(args, registry)
    }

    fn is_binary(&self) -> bool {
        true
    }
}

pub fn value_to_bson_value(v: &Tagged<Value>) -> Result<Bson, ShellError> {
    Ok(match &v.item {
        Value::Primitive(Primitive::Boolean(b)) => Bson::Boolean(*b),
        // FIXME: What about really big decimals?
        Value::Primitive(Primitive::Bytes(decimal)) => Bson::FloatingPoint(
            (decimal)
                .to_f64()
                .expect("Unimplemented BUG: What about big decimals?"),
        ),
        Value::Primitive(Primitive::Duration(secs)) => Bson::I64(*secs as i64),
        Value::Primitive(Primitive::Date(d)) => Bson::UtcDatetime(*d),
        Value::Primitive(Primitive::EndOfStream) => Bson::Null,
        Value::Primitive(Primitive::BeginningOfStream) => Bson::Null,
        Value::Primitive(Primitive::Decimal(d)) => Bson::FloatingPoint(d.to_f64().unwrap()),
        Value::Primitive(Primitive::Int(i)) => {
            Bson::I64(i.tagged(&v.tag).coerce_into("converting to BSON")?)
        }
        Value::Primitive(Primitive::Nothing) => Bson::Null,
        Value::Primitive(Primitive::String(s)) => Bson::String(s.clone()),
        Value::Primitive(Primitive::ColumnPath(path)) => Bson::Array(
            path.iter()
                .map(|x| match &x.item {
                    RawPathMember::String(string) => Ok(Bson::String(string.clone())),
                    RawPathMember::Int(int) => Ok(Bson::I64(
                        int.tagged(&v.tag).coerce_into("converting to BSON")?,
                    )),
                })
                .collect::<Result<Vec<Bson>, ShellError>>()?,
        ),
        Value::Primitive(Primitive::Pattern(p)) => Bson::String(p.clone()),
        Value::Primitive(Primitive::Path(s)) => Bson::String(s.display().to_string()),
        Value::Table(l) => Bson::Array(
            l.iter()
                .map(|x| value_to_bson_value(x))
                .collect::<Result<_, _>>()?,
        ),
        Value::Block(_) => Bson::Null,
        Value::Error(e) => return Err(e.clone()),
        Value::Primitive(Primitive::Binary(b)) => Bson::Binary(BinarySubtype::Generic, b.clone()),
        Value::Row(o) => object_value_to_bson(o)?,
    })
}

// object_value_to_bson handles all Objects, even those that correspond to special
// types (things like regex or javascript code).
fn object_value_to_bson(o: &Dictionary) -> Result<Bson, ShellError> {
    let mut it = o.entries.iter();
    if it.len() > 2 {
        return generic_object_value_to_bson(o);
    }
    match it.next() {
        Some((regex, tagged_regex_value)) if regex == "$regex" => match it.next() {
            Some((options, tagged_opts_value)) if options == "$options" => {
                let r: Result<String, _> = tagged_regex_value.try_into();
                let opts: Result<String, _> = tagged_opts_value.try_into();
                if r.is_err() || opts.is_err() {
                    generic_object_value_to_bson(o)
                } else {
                    Ok(Bson::RegExp(r.unwrap(), opts.unwrap()))
                }
            }
            _ => generic_object_value_to_bson(o),
        },
        Some((javascript, tagged_javascript_value)) if javascript == "$javascript" => {
            match it.next() {
                Some((scope, tagged_scope_value)) if scope == "$scope" => {
                    let js: Result<String, _> = tagged_javascript_value.try_into();
                    let s: Result<&Dictionary, _> = tagged_scope_value.try_into();
                    if js.is_err() || s.is_err() {
                        generic_object_value_to_bson(o)
                    } else {
                        if let Bson::Document(doc) = object_value_to_bson(s.unwrap())? {
                            Ok(Bson::JavaScriptCodeWithScope(js.unwrap(), doc))
                        } else {
                            generic_object_value_to_bson(o)
                        }
                    }
                }
                None => {
                    let js: Result<String, _> = tagged_javascript_value.try_into();

                    match js {
                        Err(_) => generic_object_value_to_bson(o),
                        Ok(v) => Ok(Bson::JavaScriptCode(v)),
                    }
                }
                _ => generic_object_value_to_bson(o),
            }
        }
        Some((timestamp, tagged_timestamp_value)) if timestamp == "$timestamp" => {
            let ts: Result<i64, _> = tagged_timestamp_value.try_into();
            if ts.is_err() {
                generic_object_value_to_bson(o)
            } else {
                Ok(Bson::TimeStamp(ts.unwrap()))
            }
        }
        Some((binary_subtype, tagged_binary_subtype_value))
            if binary_subtype == "$binary_subtype" =>
        {
            match it.next() {
                Some((binary, tagged_bin_value)) if binary == "$binary" => {
                    let bst = get_binary_subtype(tagged_binary_subtype_value);
                    let bin: Result<Vec<u8>, _> = tagged_bin_value.try_into();

                    match bst {
                        Err(_) => generic_object_value_to_bson(o),
                        Ok(v) => Ok(Bson::Binary(v, bin.unwrap())),
                    }
                }
                _ => generic_object_value_to_bson(o),
            }
        }
        Some((object_id, tagged_object_id_value)) if object_id == "$object_id" => {
            let obj_id: Result<String, _> = tagged_object_id_value.try_into();
            if obj_id.is_err() {
                generic_object_value_to_bson(o)
            } else {
                let obj_id = ObjectId::with_string(&obj_id.unwrap());
                if obj_id.is_err() {
                    generic_object_value_to_bson(o)
                } else {
                    Ok(Bson::ObjectId(obj_id.unwrap()))
                }
            }
        }
        Some((symbol, tagged_symbol_value)) if symbol == "$symbol" => {
            let sym: Result<String, _> = tagged_symbol_value.try_into();
            if sym.is_err() {
                generic_object_value_to_bson(o)
            } else {
                Ok(Bson::Symbol(sym.unwrap()))
            }
        }
        _ => generic_object_value_to_bson(o),
    }
}

fn get_binary_subtype<'a>(tagged_value: &'a Tagged<Value>) -> Result<BinarySubtype, ShellError> {
    match tagged_value.item() {
        Value::Primitive(Primitive::String(s)) => Ok(match s.as_ref() {
            "generic" => BinarySubtype::Generic,
            "function" => BinarySubtype::Function,
            "binary_old" => BinarySubtype::BinaryOld,
            "uuid_old" => BinarySubtype::UuidOld,
            "uuid" => BinarySubtype::Uuid,
            "md5" => BinarySubtype::Md5,
            _ => unreachable!(),
        }),
        Value::Primitive(Primitive::Int(i)) => Ok(BinarySubtype::UserDefined(
            i.tagged(&tagged_value.tag)
                .coerce_into("converting to BSON binary subtype")?,
        )),
        _ => Err(ShellError::type_error(
            "bson binary",
            tagged_value.type_name().spanned(tagged_value.span()),
        )),
    }
}

// generic_object_value_bson handles any Object that does not
// correspond to a special bson type (things like regex or javascript code).
fn generic_object_value_to_bson(o: &Dictionary) -> Result<Bson, ShellError> {
    let mut doc = Document::new();
    for (k, v) in o.entries.iter() {
        doc.insert(k.clone(), value_to_bson_value(v)?);
    }
    Ok(Bson::Document(doc))
}

fn shell_encode_document(writer: &mut Vec<u8>, doc: Document, tag: Tag) -> Result<(), ShellError> {
    match encode_document(writer, &doc) {
        Err(e) => Err(ShellError::labeled_error(
            format!("Failed to encode document due to: {:?}", e),
            "requires BSON-compatible document",
            tag,
        )),
        _ => Ok(()),
    }
}

fn bson_value_to_bytes(bson: Bson, tag: Tag) -> Result<Vec<u8>, ShellError> {
    let mut out = Vec::new();
    match bson {
        Bson::Array(a) => {
            for v in a.into_iter() {
                match v {
                    Bson::Document(d) => shell_encode_document(&mut out, d, tag.clone())?,
                    _ => {
                        return Err(ShellError::labeled_error(
                            format!("All top level values must be Documents, got {:?}", v),
                            "requires BSON-compatible document",
                            &tag,
                        ))
                    }
                }
            }
        }
        Bson::Document(d) => shell_encode_document(&mut out, d, tag)?,
        _ => {
            return Err(ShellError::labeled_error(
                format!("All top level values must be Documents, got {:?}", bson),
                "requires BSON-compatible document",
                tag,
            ))
        }
    }
    Ok(out)
}

fn to_bson(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_tag = args.name_tag();
    let stream = async_stream! {
        let input: Vec<Tagged<Value>> = args.input.values.collect().await;

        let to_process_input = if input.len() > 1 {
            let tag = input[0].tag.clone();
            vec![Tagged { item: Value::Table(input), tag } ]
        } else if input.len() == 1 {
            input
        } else {
            vec![]
        };

        for value in to_process_input {
            match value_to_bson_value(&value) {
                Ok(bson_value) => {
                    match bson_value_to_bytes(bson_value, name_tag.clone()) {
                        Ok(x) => yield ReturnSuccess::value(
                            Value::binary(x).tagged(&name_tag),
                        ),
                        _ => yield Err(ShellError::labeled_error_with_secondary(
                            "Expected a table with BSON-compatible structure.tag() from pipeline",
                            "requires BSON-compatible input",
                            &name_tag,
                            "originates from here".to_string(),
                            value.tag(),
                        )),
                    }
                }
                _ => yield Err(ShellError::labeled_error(
                    "Expected a table with BSON-compatible structure from pipeline",
                    "requires BSON-compatible input",
                    &name_tag))
            }
        }
    };

    Ok(stream.to_output_stream())
}
