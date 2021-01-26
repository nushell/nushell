use nu_errors::ParseError;

#[derive(new)]
pub struct ParseResult<Value> {
    pub value: Value,
    pub i: usize,
    pub err: Option<ParseError>,
}

impl<Value> From<ParseResult<Value>> for (Value, usize, Option<ParseError>) {
    fn from(result: ParseResult<Value>) -> Self {
        (result.value, result.i, result.err)
    }
}

impl<Value> From<(Value, usize, Option<ParseError>)> for ParseResult<Value> {
    fn from((value, i, err): (Value, usize, Option<ParseError>)) -> Self {
        Self { value, i, err }
    }
}
