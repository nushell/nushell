# `serde-nu`

Convert any value implementing `serde::Serialize` into a
`nu_protocol::Value` using `nu_serde::to_value`. Compare the below manual
implemeentation and the one using `nu_serde`.

```rust
use nu_protocol::{Dictionary, Primitive, UntaggedValue, Value};
use nu_source::Tag;
use serde::Serialize;

#[derive(Serialize)]
struct MyStruct {
    index: usize,
    name: String,
}

fn manual(s: MyStruct, tag: Tag) -> Value {
    let mut dict = Dictionary::default();
    dict.insert(
        "index".into(),
        Value {
            value: UntaggedValue::Primitive(Primitive::Int(s.index as i64)),
            tag: tag.clone(),
        },
    );
    dict.insert(
        "name".into(),
        Value {
            value: UntaggedValue::Primitive(Primitive::String(s.name)),
            tag: tag.clone(),
        },
    );

    Value {
        value: UntaggedValue::Row(dict),
        tag,
    }
}

fn auto(s: &MyStruct, tag: Tag) -> Value {
    nu_serde::to_value(s, tag).unwrap()
}
```
