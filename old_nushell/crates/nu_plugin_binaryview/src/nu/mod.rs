use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Primitive, Signature, SyntaxShape, UntaggedValue, Value};

use crate::binaryview::view_binary;
use crate::BinaryView;

impl Plugin for BinaryView {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("binaryview")
            .desc("Autoview of binary data.")
            .switch("lores", "use low resolution output mode", Some('l'))
            .named(
                "skip",
                SyntaxShape::Int,
                "skip x number of bytes",
                Some('s'),
            )
            .named(
                "bytes",
                SyntaxShape::Int,
                "show y number of bytes",
                Some('b'),
            ))
    }

    fn sink(&mut self, call_info: CallInfo, input: Vec<Value>) {
        for v in input {
            let value_anchor = v.anchor();
            if let UntaggedValue::Primitive(Primitive::Binary(b)) = &v.value {
                let low_res = call_info.args.has("lores");
                let _ = view_binary(b, value_anchor.as_ref(), low_res);
            }
        }
    }
}
