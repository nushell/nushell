use data_encoding::Encoding;

use nu_engine::command_prelude::*;

pub struct Base32Config {
    lower: bool,
    lower_span: Option<Span>,
    nopad: bool,
    nopad_span: Option<Span>,
    dnscurve: bool,
    dnscurve_span: Option<Span>,
    dnssec: bool,
    dnssec_span: Option<Span>,
}

impl Base32Config {
    pub fn new(engine_state: &EngineState, stack: &mut Stack, call: &Call) -> Self {
        Base32Config {
            lower: call.get_flag(engine_state, stack, "lower"),
            lower_span: call.get_flag_span(stack, "lower"),
            nopad: call.get_flag(engine_state, stack, "nopad"),
            nopad_span: call.get_flag_span(stack, "nopad"),
            dnscurve: call.get_flag(engine_state, stack, "dnscurve"),
            dnscurve_span: call.get_flag_span(stack, "dnscurve"),
            dnssec: call.get_flag(engine_state, stack, "dnssec"),
            dnssec_span: call.get_flag_span(stack, "dnssec"),
        }
    }
}

fn base32_encoding(config: Base32Config) -> Result<Encoding, ShellError> {
    if let Some(dnscurve_span) = config.dnscurve_span {
        if let Some(lower_span) = config.lower_span {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Inapplicable to DNSCURVE".to_string(),
                left_span: lower_span,
                right_message: "DNSCURVE must be used standalone".to_string(),
                right_span: dnscurve_span,
            });
        }
        if let Some(nopad_span) = config.nopad_span {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Inapplicable to DNSCURVE".to_string(),
                left_span: nopad_span,
                right_message: "DNSCURVE must be used standalone".to_string(),
                right_span: dnscurve_span,
            });
        }
        if let Some(dnssec) = config.dnssec_span {
            return Err(ShellError::IncompatibleParameters {
                left_message: "Inapplicable to DNSCURVE".to_string(),
                left_span: nopad_span,
                right_message: "DNSCURVE must be used standalone".to_string(),
                right_span: dnscurve_span,
            });
        }

	return Ok(data_encoding::BASE32_DNSCURVE);
    }

    todo!()
}
