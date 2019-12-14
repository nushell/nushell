use crate::parse::token_tree::{ParseErrorFn, SpannedToken, TokenType};
use nu_errors::ParseError;
use std::borrow::Cow;

pub struct Pattern<T> {
    parts: Vec<Box<dyn TokenType<Output = T>>>,
}

impl<T> TokenType for Pattern<T> {
    type Output = T;

    fn desc(&self) -> Cow<'static, str> {
        Cow::Borrowed("pattern")
    }

    fn extract_token_value(
        &self,
        token: &SpannedToken,
        err: ParseErrorFn<Self::Output>,
    ) -> Result<Self::Output, ParseError> {
        for part in &self.parts {
            match part.extract_token_value(token, err) {
                Err(_) => {}
                Ok(result) => return Ok(result),
            }
        }

        err()
    }
}
