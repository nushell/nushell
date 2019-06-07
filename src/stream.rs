use crate::prelude::*;
use futures::stream::BoxStream;

pub type InputStream = BoxStream<'static, Value>;
pub type OutputStream = BoxStream<'static, ReturnValue>;

crate fn single_output(item: Value) -> OutputStream {
    let value = ReturnValue::Value(item);
    let mut vec = VecDeque::new();
    vec.push_back(value);
    vec.boxed()
}
