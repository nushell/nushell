use crate::parser_state::Type;
use crate::ParserWorkingSet;
use std::ops::Range;

pub use crate::Span;

#[derive(Debug)]
pub enum ParseError {
    ExtraTokens(Span),
    ExtraPositional(Span),
    UnexpectedEof(String, Span),
    Unclosed(String, Span),
    UnknownStatement(Span),
    Expected(String, Span),
    Mismatch(String, String, Span), // expected, found, span
    UnsupportedOperation(Span, Span, Type, Span, Type),
    ExpectedKeyword(String, Span),
    MultipleRestParams(Span),
    VariableNotFound(Span),
    UnknownCommand(Span),
    NonUtf8(Span),
    UnknownFlag(Span),
    UnknownType(Span),
    MissingFlagParam(Span),
    ShortFlagBatchCantTakeArg(Span),
    MissingPositional(String, Span),
    MissingType(Span),
    TypeMismatch(Type, Type, Span), // expected, found, span
    MissingRequiredFlag(String, Span),
    IncompleteMathExpression(Span),
    UnknownState(String, Span),
    IncompleteParser(Span),
}

impl<'a> codespan_reporting::files::Files<'a> for ParserWorkingSet<'a> {
    type FileId = usize;

    type Name = String;

    type Source = String;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(self.get_filename(id))
    }

    fn source(
        &'a self,
        id: Self::FileId,
    ) -> Result<Self::Source, codespan_reporting::files::Error> {
        Ok(self.get_file_source(id))
    }

    fn line_index(
        &'a self,
        id: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        let source = self.get_file_source(id);

        let mut count = 0;

        for byte in source.bytes().enumerate() {
            if byte.0 == byte_index {
                // println!("count: {} for file: {} index: {}", count, id, byte_index);
                return Ok(count);
            }
            if byte.1 == b'\n' {
                count += 1;
            }
        }

        // println!("count: {} for file: {} index: {}", count, id, byte_index);
        Ok(count)
    }

    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<Range<usize>, codespan_reporting::files::Error> {
        let source = self.get_file_source(id);

        let mut count = 0;

        let mut start = Some(0);
        let mut end = None;

        for byte in source.bytes().enumerate() {
            #[allow(clippy::comparison_chain)]
            if count > line_index {
                let start = start.expect("internal error: couldn't find line");
                let end = end.expect("internal error: couldn't find line");

                // println!(
                //     "Span: {}..{} for fileid: {} index: {}",
                //     start, end, id, line_index
                // );
                return Ok(start..end);
            } else if count == line_index {
                end = Some(byte.0 + 1);
            }

            #[allow(clippy::comparison_chain)]
            if byte.1 == b'\n' {
                count += 1;
                if count > line_index {
                    break;
                } else if count == line_index {
                    start = Some(byte.0 + 1);
                }
            }
        }

        match (start, end) {
            (Some(start), Some(end)) => {
                // println!(
                //     "Span: {}..{} for fileid: {} index: {}",
                //     start, end, id, line_index
                // );
                Ok(start..end)
            }
            _ => Err(codespan_reporting::files::Error::FileMissing),
        }
    }
}
