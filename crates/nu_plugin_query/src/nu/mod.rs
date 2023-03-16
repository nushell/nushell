use crate::Query;
use nu_plugin::{EvaluatedCall, LabeledError, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, Spanned, SyntaxShape, Value};

impl Plugin for Query {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![
            PluginSignature::build("query")
            .usage("Show all the query commands")
            .category(Category::Filters),

            PluginSignature::build("query json")
            .usage("execute json query on json file (open --raw <file> | query json 'query string')")
            .required("query", SyntaxShape::String, "json query")
            .category(Category::Filters),

            PluginSignature::build("query xml")
            .usage("execute xpath query on xml")
            .required("query", SyntaxShape::String, "xpath query")
            .category(Category::Filters),

            PluginSignature::build("query web")
            .usage("execute selector query on html/web")
            .named("query", SyntaxShape::String, "selector query", Some('q'))
            .switch("as-html", "return the query output as html", Some('m'))
            .plugin_examples(web_examples())
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

pub fn web_examples() -> Vec<PluginExample> {
    vec![PluginExample {
        example: "http get https://phoronix.com | query web -q 'header'".into(),
        description: "Retrieve all `<header>` elements from phoronix.com website".into(),
        result: None,
    }, PluginExample {
        example: "http get https://en.wikipedia.org/wiki/List_of_cities_in_India_by_population
    | query web -t [Rank City 'Population(2011)[3]' 'Population(2001)' 'State or union territory']".into(),
        description: "Retrieve a html table from Wikipedia and parse it into a nushell table using table headers as guides".into(),
        result: None
    },
    PluginExample {
        example: "http get https://www.nushell.sh | query web -q 'h2, h2 + p' | group 2 | each {rotate --ccw tagline description} | flatten".into(),
        description: "Pass multiple css selectors to extract several elements within single query, group the query results together and rotate them to create a table".into(),
        result: None,
    },
    PluginExample {
        example: "http get https://example.org | query web --query a --attribute href".into(),
        description: "Retrieve a specific html attribute instead of the default text".into(),
        result: None,
    }]
}
