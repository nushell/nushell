use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Primitive, Signature, UntaggedValue, Value};

use crate::binaryview::view_binary;
use crate::BinaryView;

impl Plugin for BinaryView {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("binaryview")
            .desc("Autoview of binary data.")
            .switch("lores", "use low resolution output mode", Some('l')))
    }

    fn sink(&mut self, call_info: CallInfo, input: Vec<Value>) {
        for v in input {
            let value_anchor = v.anchor();
            if let UntaggedValue::Primitive(Primitive::Binary(b)) = &v.value {
                let _ = view_binary(&b, value_anchor.as_ref(), call_info.args.has("lores"));
            }
        }
    }
}
