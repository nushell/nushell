use nu_protocol::{Completion, Flag, FromValue, ShellError, SyntaxShape, Value};

#[derive(Clone, Copy, Debug)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    pub const NATIVE: Self = match cfg!(target_endian = "little") {
        true => Self::Little,
        false => Self::Big,
    };

    pub fn flag() -> Flag {
        Flag::new("endian")
            .short('e')
            .arg(SyntaxShape::String)
            .desc("Byte encode endian, available options: native(default), little, big.")
            .completion(Completion::new_list(&["native", "little", "big"]))
    }
}

impl Default for Endian {
    fn default() -> Self {
        Self::NATIVE
    }
}

impl FromValue for Endian {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        let s = v.into_string()?;
        match s.as_str() {
            "little" => Ok(Self::Little),
            "big" => Ok(Self::Big),
            "native" => Ok(Self::NATIVE),
            _ => Err(ShellError::TypeMismatch {
                err_message: "Endian must be one of native, little, big".to_string(),
                span,
            }),
        }
    }
}
