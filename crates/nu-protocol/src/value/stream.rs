use crate::*;
use std::fmt::Debug;

pub struct ValueStream(pub Box<dyn Iterator<Item = Value> + Send + 'static>);

impl ValueStream {
    pub fn into_string(self) -> String {
        format!(
            "[{}]",
            self.map(|x: Value| x.into_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    pub fn collect_string(self) -> String {
        self.map(|x: Value| x.collect_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn from_stream(input: impl Iterator<Item = Value> + Send + 'static) -> ValueStream {
        ValueStream(Box::new(input))
    }
}

impl Debug for ValueStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueStream").finish()
    }
}

impl Iterator for ValueStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        {
            self.0.next()
        }
    }
}

// impl Serialize for ValueStream {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut seq = serializer.serialize_seq(None)?;

//         for element in self.0.borrow_mut().into_iter() {
//             seq.serialize_element(&element)?;
//         }
//         seq.end()
//     }
// }

// impl<'de> Deserialize<'de> for ValueStream {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         deserializer.deserialize_seq(MySeqVisitor)
//     }
// }

// struct MySeqVisitor;

// impl<'a> serde::de::Visitor<'a> for MySeqVisitor {
//     type Value = ValueStream;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("a value stream")
//     }

//     fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
//     where
//         A: serde::de::SeqAccess<'a>,
//     {
//         let mut output: Vec<Value> = vec![];

//         while let Some(value) = seq.next_element()? {
//             output.push(value);
//         }

//         Ok(ValueStream(Rc::new(RefCell::new(output.into_iter()))))
//     }
// }

// pub trait IntoValueStream {
//     fn into_value_stream(self) -> ValueStream;
// }

// impl<T> IntoValueStream for T
// where
//     T: Iterator<Item = Value> + 'static,
// {
//     fn into_value_stream(self) -> ValueStream {
//         ValueStream::from_stream(self)
//     }
// }
