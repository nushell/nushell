use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, Type, Value};

use crate::Query;

pub struct QueryWebpageInfo;

impl SimplePluginCommand for QueryWebpageInfo {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query webpage-info"
    }

    fn description(&self) -> &str {
        "uses the webpage crate to extract info from html: title, description, language, links, RSS feeds, Opengraph, Schema.org, and more"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::String, Type::record())
            .category(Category::Network)
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
            Value::String { val, .. } => execute_webpage(val, span),
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

fn execute_webpage(html: &str, span: Span) -> Result<Value, LabeledError> {
    let info = webpage::HTML::from_string(html.to_string(), None)
        .map_err(|e| LabeledError::new(e.to_string()).with_label("error parsing html", span))?;

    let value = nu_protocol::serde::to_value(&info, span).map_err(|e| {
        LabeledError::new(e.to_string()).with_label("error convert Value::Record", span)
    })?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: &str = r#"
         <html><head><meta><title>My Title</title></head></html>
     "#;

    #[test]
    fn test_basics() {
        let info = execute_webpage(HTML, Span::test_data()).unwrap();
        let record = info.as_record().unwrap();
        assert_eq!(record.get("title").unwrap().as_str().unwrap(), "My Title");
    }
}
