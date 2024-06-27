use crate::Query;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Value};

pub struct QueryWebpageInfo;

impl SimplePluginCommand for QueryWebpageInfo {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query webpage-info"
    }

    fn usage(&self) -> &str {
        "uses the webpage crate to extract info from html: title, description, language, HTTP info, links, RSS feeds, Opengraph, Schema.org, and more"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Network)
    }

    fn examples(&self) -> Vec<Example> {
        web_examples()
    }

    fn run(
        &self,
        _plugin: &Query,
        _engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = input.span();
        match input {
            Value::String { val, .. } => Ok(input.clone()),
            _ => Err(LabeledError::new("Requires text input")
                .with_label("expected text from pipeline", span)),
        }
    }
}

pub fn web_examples() -> Vec<Example<'static>> {
    vec![Example {
        example: "http get https://phoronix.com | query webpage-info",
        description: "extract detailed info from phoronix.com website",
        result: None,
    }]
}

/*
fn execute_webpage(
    input_string: &str,
    span: Span,
) -> Value {
    let doc = Html::parse_fragment(input_string);

    let vals: Vec<Value> = match as_html {
        true => doc
            .select(&css(query_string, inspect))
            .map(|selection| Value::string(selection.html(), span))
            .collect(),
        false => doc
            .select(&css(query_string, inspect))
            .map(|selection| {
                Value::list(
                    selection
                        .text()
                        .map(|text| Value::string(text, span))
                        .collect(),
                    span,
                )
            })
            .collect(),
    };

    Value::list(vals, span)
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: &str = r#"
         <html><head><meta><title>My Title</title></head></html>
     "#;

    #[test]
    fn test_first_child_is_not_empty() {
        assert!(!execute_selector_query(
            SIMPLE_LIST,
            "li:first-child",
            false,
            false,
            Span::test_data()
        )
        .is_empty())
    }
}
