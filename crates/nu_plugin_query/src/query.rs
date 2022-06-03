use crate::query_json::execute_json_query;
use crate::query_web::parse_selector_params;
use crate::query_xml::execute_xpath_query;
use nu_engine::documentation::get_flags_section;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Signature, Spanned, Value};
use std::fmt::Write;

#[derive(Default)]
pub struct Query;

impl Query {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn usage() -> &'static str {
        "Usage: query"
    }

    pub fn query(
        &self,
        _name: &str,
        call: &EvaluatedCall,
        _value: &Value,
        _path: Option<Spanned<String>>,
    ) -> Result<Value, LabeledError> {
        let help = get_brief_subcommand_help(&Query.signature());
        Ok(Value::string(help, call.head))
    }

    pub fn query_json(
        &self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
        query: Option<Spanned<String>>,
    ) -> Result<Value, LabeledError> {
        execute_json_query(name, call, input, query)
    }
    pub fn query_web(
        &self,
        _name: &str,
        call: &EvaluatedCall,
        input: &Value,
        _rest: Option<Spanned<String>>,
    ) -> Result<Value, LabeledError> {
        parse_selector_params(call, input)
    }
    pub fn query_xml(
        &self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
        query: Option<Spanned<String>>,
    ) -> Result<Value, LabeledError> {
        execute_xpath_query(name, call, input, query)
    }
}

pub fn get_brief_subcommand_help(sigs: &[Signature]) -> String {
    let mut help = String::new();
    let _ = write!(help, "{}\n\n", sigs[0].usage);
    let _ = write!(help, "Usage:\n  > {}\n\n", sigs[0].name);
    help.push_str("Subcommands:\n");

    for x in sigs.iter().enumerate() {
        if x.0 == 0 {
            continue;
        }
        let _ = writeln!(help, "  {} - {}", x.1.name, x.1.usage);
    }

    help.push_str(&get_flags_section(&sigs[0]));
    help
}
