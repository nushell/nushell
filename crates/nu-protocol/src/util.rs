use crate::ShellError;
use std::io::{BufRead, BufReader, Read};

pub struct BufferedReader<R: Read> {
    pub input: BufReader<R>,
}

impl<R: Read> BufferedReader<R> {
    pub fn new(input: BufReader<R>) -> Self {
        Self { input }
    }
}

impl<R: Read> Iterator for BufferedReader<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = self.input.fill_buf();
        match buffer {
            Ok(s) => {
                let result = s.to_vec();

                let buffer_len = s.len();

                if buffer_len == 0 {
                    None
                } else {
                    self.input.consume(buffer_len);

                    Some(Ok(result))
                }
            }
            Err(e) => Some(Err(ShellError::IOError { msg: e.to_string() })),
        }
    }
}
