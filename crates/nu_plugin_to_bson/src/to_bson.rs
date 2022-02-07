use bson::{oid::ObjectId, spec::BinarySubtype, Bson, Document};
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{
    Dictionary, Primitive, ReturnSuccess, ReturnValue, SpannedTypeName, UnspannedPathMember,
    UntaggedValue, Value,
};
use nu_source::{Tag, TaggedItem};
use num_traits::ToPrimitive;
use std::convert::TryInto;

#[derive(Default)]
pub struct ToBson {
    pub state: Vec<Value>,
}

impl ToBson {
    pub fn new() -> ToBson {
        ToBson { state: vec![] }
    }
}

pub fn value_to_bson_value(v: &Value) -> Result<Bson, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => Bson::Boolean(*b),
        // FIXME: What about really big decimals?
        UntaggedValue::Primitive(Primitive::Filesize(decimal)) => Bson::Double(
            (decimal)
                .to_f64()
                .expect("Unimplemented BUG: What about big decimals?"),
        ),
        UntaggedValue::Primitive(Primitive::Duration(i)) => Bson::String(i.to_string()),
        UntaggedValue::Primitive(Primitive::Date(d)) => {
            Bson::DateTime(bson::DateTime::from_chrono(*d))
        }
        UntaggedValue::Primitive(Primitive::EndOfStream) => Bson::Null,
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => Bson::Null,
        UntaggedValue::Primitive(Primitive::Decimal(d)) => {
            Bson::Double(d.to_f64().ok_or_else(|| {
                ShellError::labeled_error(
                    "Could not convert value to decimal",
                    "could not convert to decimal",
                    &v.tag,
                )
            })?)
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => Bson::Int64(*i),
        UntaggedValue::Primitive(Primitive::BigInt(i)) => {
            Bson::Int64(i.tagged(&v.tag).coerce_into("converting to BSON")?)
        }
        UntaggedValue::Primitive(Primitive::Nothing) => Bson::Null,
        UntaggedValue::Primitive(Primitive::String(s)) => Bson::String(s.clone()),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => Bson::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => Ok(Bson::String(string.clone())),
                    UnspannedPathMember::Int(int) => Ok(Bson::Int64(*int)),
                })
                .collect::<Result<Vec<Bson>, ShellError>>()?,
        ),
        UntaggedValue::Primitive(Primitive::GlobPattern(p)) => Bson::String(p.clone()),
        UntaggedValue::Primitive(Primitive::FilePath(s)) => Bson::String(s.display().to_string()),
        UntaggedValue::Table(l) => Bson::Array(
            l.iter()
                .map(value_to_bson_value)
                .collect::<Result<_, _>>()?,
        ),
        UntaggedValue::Block(_) | UntaggedValue::Primitive(Primitive::Range(_)) => Bson::Null,
        #[cfg(feature = "dataframe")]
        UntaggedValue::DataFrame(_) | UntaggedValue::FrameStruct(_) => Bson::Null,
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Primitive(Primitive::Binary(b)) => Bson::Binary(bson::Binary {
            subtype: BinarySubtype::Generic,
            bytes: b.clone(),
        }),
        UntaggedValue::Row(o) => object_value_to_bson(o)?,
        // TODO  Impelmenting Bson::Undefined, Bson::MaxKey, Bson::MinKey and Bson::DbPointer
        // These Variants weren't present in the previous version.
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
                match (r, opts) {
                    (Ok(pattern), Ok(options)) => {
                        Ok(Bson::RegularExpression(bson::Regex { pattern, options }))
                    }
                    _ => generic_object_value_to_bson(o),
                }
            }
            _ => generic_object_value_to_bson(o),
        },
        Some((javascript, tagged_javascript_value)) if javascript == "$javascript" => {
            match it.next() {
                Some((scope, tagged_scope_value)) if scope == "$scope" => {
                    let js: Result<String, _> = tagged_javascript_value.try_into();
                    let s: Result<&Dictionary, _> = tagged_scope_value.try_into();

                    match (js, s) {
                        (Ok(code), Ok(s)) => {
                            if let Bson::Document(scope) = object_value_to_bson(s)? {
                                Ok(Bson::JavaScriptCodeWithScope(
                                    bson::JavaScriptCodeWithScope { code, scope },
                                ))
                            } else {
                                generic_object_value_to_bson(o)
                            }
                        }
                        _ => generic_object_value_to_bson(o),
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
            if let Ok(time) = ts {
                Ok(Bson::Timestamp(bson::Timestamp {
                    time: time as u32,
                    increment: Default::default(),
                }))
            } else {
                generic_object_value_to_bson(o)
            }
        }
        Some((binary_subtype, tagged_binary_subtype_value))
            if binary_subtype == "$binary_subtype" =>
        {
            match it.next() {
                Some((binary, tagged_bin_value)) if binary == "$binary" => {
                    let bst = get_binary_subtype(tagged_binary_subtype_value);
                    let bin: Result<Vec<u8>, _> = tagged_bin_value.try_into();

                    match (bin, bst) {
                        (Ok(bin), Ok(subtype)) => Ok(Bson::Binary(bson::Binary {
                            subtype,
                            bytes: bin,
                        })),
                        _ => generic_object_value_to_bson(o),
                    }
                }
                _ => generic_object_value_to_bson(o),
            }
        }
        Some((object_id, tagged_object_id_value)) if object_id == "$object_id" => {
            let obj_id: Result<String, _> = tagged_object_id_value.try_into();

            if let Ok(obj_id) = obj_id {
                let obj_id = ObjectId::parse_str(&obj_id);

                if let Ok(obj_id) = obj_id {
                    Ok(Bson::ObjectId(obj_id))
                } else {
                    generic_object_value_to_bson(o)
                }
            } else {
                generic_object_value_to_bson(o)
            }
        }
        Some((symbol, tagged_symbol_value)) if symbol == "$symbol" => {
            let sym: Result<String, _> = tagged_symbol_value.try_into();
            if let Ok(sym) = sym {
                Ok(Bson::Symbol(sym))
            } else {
                generic_object_value_to_bson(o)
            }
        }
        _ => generic_object_value_to_bson(o),
    }
}

fn get_binary_subtype(tagged_value: &Value) -> Result<BinarySubtype, ShellError> {
    match &tagged_value.value {
        UntaggedValue::Primitive(Primitive::String(s)) => Ok(match s.as_ref() {
            "generic" => BinarySubtype::Generic,
            "function" => BinarySubtype::Function,
            "binary_old" => BinarySubtype::BinaryOld,
            "uuid_old" => BinarySubtype::UuidOld,
            "uuid" => BinarySubtype::Uuid,
            "md5" => BinarySubtype::Md5,
            _ => unreachable!(),
        }),
        UntaggedValue::Primitive(Primitive::BigInt(i)) => Ok(BinarySubtype::UserDefined(
            i.tagged(&tagged_value.tag)
                .coerce_into("converting to BSON binary subtype")?,
        )),
        _ => Err(ShellError::type_error(
            "bson binary",
            tagged_value.spanned_type_name(),
        )),
    }
}

// generic_object_value_bson handles any Object that does not
// correspond to a special bson type (things like regex or javascript code).
fn generic_object_value_to_bson(o: &Dictionary) -> Result<Bson, ShellError> {
    let mut doc = Document::new();
    for (k, v) in &o.entries {
        doc.insert(k.clone(), value_to_bson_value(v)?);
    }
    Ok(Bson::Document(doc))
}

fn shell_encode_document(writer: &mut Vec<u8>, doc: Document, tag: Tag) -> Result<(), ShellError> {
    match doc.to_writer(writer) {
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
            for v in a {
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

pub fn to_bson(input: Vec<Value>, name_tag: Tag) -> Vec<ReturnValue> {
    let name_span = name_tag.span;

    let to_process_input = match input.len() {
        x if x > 1 => {
            let tag = input[0].tag.clone();
            vec![Value {
                value: UntaggedValue::Table(input),
                tag,
            }]
        }
        1 => input,
        _ => vec![],
    };

    to_process_input
        .into_iter()
        .map(move |value| match value_to_bson_value(&value) {
            Ok(bson_value) => {
                let value_span = value.tag.span;

                match bson_value_to_bytes(bson_value, name_tag.clone()) {
                    Ok(x) => ReturnSuccess::value(UntaggedValue::binary(x).into_value(name_span)),
                    _ => Err(ShellError::labeled_error_with_secondary(
                        "Expected a table with BSON-compatible structure from pipeline",
                        "requires BSON-compatible input",
                        name_span,
                        "originates from here".to_string(),
                        value_span,
                    )),
                }
            }
            _ => Err(ShellError::labeled_error(
                "Expected a table with BSON-compatible structure from pipeline",
                "requires BSON-compatible input",
                &name_tag,
            )),
        })
        .collect()
}
