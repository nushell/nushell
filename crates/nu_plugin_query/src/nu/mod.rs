use crate::Query;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, Signature, Spanned, SyntaxShape, Value};

impl Plugin for Query {
    fn signature(&self) -> Vec<Signature> {
        vec![
            Signature::build("query")
            .usage("Show all the query commands")
            .category(Category::Filters),

            Signature::build("query json")
            .usage("execute json query on json file (open --raw <file> | query json 'query string')")
            .required("query", SyntaxShape::String, "json query")
            .category(Category::Filters),

            Signature::build("query xml")
            .usage("execute xpath query on xml")
            .required("query", SyntaxShape::String, "xpath query")
            .category(Category::Filters),

            Signature::build("query web")
            .usage("execute selector query on html/web")
            .named("query", SyntaxShape::String, "selector query", Some('q'))
            .switch("as-html", "return the query output as html", Some('m'))
            .named(
                "attribute",
                SyntaxShape::String,
                "downselect based on the given attribute",
                Some('a'),
            )
            .named(
                "as-table",
                SyntaxShape::Table,
                "find table based on column header list",
                Some('t'),
            )
            .switch(
                "inspect",
                "run in inspect mode to provide more information for determining column headers",
                Some('i'),
            )
            .category(Category::Network),
            ]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        // You can use the name to identify what plugin signature was called
        let path: Option<Spanned<String>> = call.opt(0)?;

        match name {
            "query" => {
                self.query(name, call, input, path)
            }
            "query json" => self.query_json( name, call, input, path),
            "query web" => self.query_web(name, call, input, path),
            "query xml" => self.query_xml(name, call, input, path),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
