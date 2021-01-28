use nu_errors::{ParseError, ParseWarning};

#[derive(new)]
pub struct ParseResult<Value> {
    pub value: Value,
    pub i: usize,
    pub err: Option<ParseError>,
    pub warnings: Vec<ParseWarning>,
}

impl<Value> From<(Value, usize, Option<ParseError>, Vec<ParseWarning>)> for ParseResult<Value> {
    fn from(
        (value, i, err, warnings): (Value, usize, Option<ParseError>, Vec<ParseWarning>),
    ) -> Self {
        Self::new(value, i, err, warnings)
    }
}

impl<Value> From<(Value, usize, Option<ParseError>)> for ParseResult<Value> {
    fn from((value, i, err): (Value, usize, Option<ParseError>)) -> Self {
        Self::new(value, i, err, vec![])
    }
}

impl<Value> From<(Value, usize)> for ParseResult<Value> {
    fn from((value, i): (Value, usize)) -> Self {
        Self::new(value, i, None, vec![])
    }
}

impl<Value> From<ParseResult<Value>> for (Value, usize, Option<ParseError>, Vec<ParseWarning>) {
    fn from(result: ParseResult<Value>) -> Self {
        (result.value, result.i, result.err, result.warnings)
    }
}
