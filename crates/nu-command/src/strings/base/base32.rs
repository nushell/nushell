use data_encoding::Encoding;

use nu_engine::command_prelude::*;

pub struct Base32Config {
    nopad: Option<Span>,
    dnscurve: Option<Span>,
    dnssec: Option<Span>,
}

impl Base32Config {
    pub fn from(stack: &mut Stack, call: &Call) -> Self {
        Base32Config {
            nopad: call.get_flag_span(stack, "nopad"),
            dnscurve: call.get_flag_span(stack, "dnscurve"),
            dnssec: call.get_flag_span(stack, "dnssec"),
        }
    }
}

pub fn base32_encoding(config: Base32Config) -> Result<Encoding, ShellError> {
    match (config.nopad, config.dnscurve, config.dnssec) {
        (Some(nopad), Some(dnscurve), _) => {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Inapplicable to DNSCURVE".to_string(),
                left_span: nopad,
                right_message: "Must be used standalone".to_string(),
                right_span: dnscurve,
            });
        }
        (_, Some(dnscurve), Some(dnssec)) => {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Incompatible with DNSCURVE".to_string(),
                left_span: dnssec,
                right_message: "Must be used standalone".to_string(),
                right_span: dnscurve,
            });
        }
        (Some(nopad), _, Some(dnssec)) => {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Inapplicable to DNSCURVE".to_string(),
                left_span: nopad,
                right_message: "DNSCURVE must be used standalone".to_string(),
                right_span: dnssec,
            });
        }

        (None, None, None) => Ok(data_encoding::BASE32),
        (Some(_), None, None) => Ok(data_encoding::BASE32_NOPAD),
        (None, Some(_), None) => Ok(data_encoding::BASE32_DNSCURVE),
        (None, None, Some(_)) => Ok(data_encoding::BASE32_DNSSEC),
    }
}

#[derive(Clone)]
pub struct DecodeBase32;

impl Command for DecodeBase32 {
    fn name(&self) -> &str {
        "decode base32"
    }

    fn signature(&self) -> Signature {
        Signature::build("decode base32")
            .input_output_types(vec![(Type::String, Type::Binary)])
            .allow_variants_without_examples(true)
            .switch("nopad", "Do not pad the output.", None)
            .switch("dnscurve", "Use DNSCURVE Base32 variant.", None)
            .switch("dnssec", "Use DNSSEC Base32 variant.", None)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Decode a value."
    }

    fn extra_usage(&self) -> &str {
        "TODO"
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let config = Base32Config::from(stack, call);
        let encoding = base32_encoding(config)?;
        super::decode(encoding, call.span(), input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        todo!()
    }
}

#[derive(Clone)]
pub struct EncodeBase32;

impl Command for EncodeBase32 {
    fn name(&self) -> &str {
        "encode base32"
    }

    fn signature(&self) -> Signature {
        Signature::build("encode base32")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::Binary, Type::String),
            ])
            .allow_variants_without_examples(true)
            .switch("nopad", "Don't accept padding.", None)
            .switch("dnscurve", "Parse as the DNSCURVE Base32 variant.", None)
            .switch("dnssec", "Parse as the DNSSEC Base32 variant.", None)
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Encode a value."
    }

    fn extra_usage(&self) -> &str {
        "TODO"
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let config = Base32Config::from(stack, call);
        let encoding = base32_encoding(config)?;
        super::encode(encoding, call.span(), input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples_decode() {
        crate::test_examples(DecodeBase32)
    }

    #[test]
    fn test_examples_encode() {
        crate::test_examples(EncodeBase32)
    }
}
