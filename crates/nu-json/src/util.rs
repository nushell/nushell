use std::io;
use std::str;

use super::error::{Error, ErrorCode, Result};

pub struct StringReader<Iter: Iterator<Item = u8>> {
    iter: Iter,
    line: usize,
    col: usize,
    ch: Vec<u8>,
}

impl<Iter> StringReader<Iter>
where
    Iter: Iterator<Item = u8>,
{
    #[inline]
    pub fn new(iter: Iter) -> Self {
        StringReader {
            iter,
            line: 1,
            col: 0,
            ch: Vec::new(),
        }
    }

    fn next(&mut self) -> Option<io::Result<u8>> {
        match self.iter.next() {
            None => None,
            Some(b'\n') => {
                self.line += 1;
                self.col = 0;
                Some(Ok(b'\n'))
            }
            Some(c) => {
                self.col += 1;
                Some(Ok(c))
            }
        }
    }

    pub fn pos(&mut self) -> (usize, usize) {
        (self.line, self.col)
    }

    pub fn eof(&mut self) -> Result<bool> {
        Ok(self.peek()?.is_none())
    }

    pub fn peek_next(&mut self, idx: usize) -> Result<Option<u8>> {
        while self.ch.len() <= idx {
            match self.next() {
                Some(Err(err)) => return Err(Error::Io(err)),
                Some(Ok(ch)) => self.ch.push(ch),
                None => return Ok(None),
            }
        }
        Ok(Some(self.ch[idx]))
    }

    // pub fn peek_next_or_null(&mut self, idx: usize) -> Result<u8> {
    //     Ok(try!(self.peek_next(idx)).unwrap_or(b'\x00'))
    // }

    pub fn peek(&mut self) -> Result<Option<u8>> {
        self.peek_next(0)
    }

    pub fn peek_or_null(&mut self) -> Result<u8> {
        Ok(self.peek()?.unwrap_or(b'\x00'))
    }

    pub fn eat_char(&mut self) -> u8 {
        self.ch.remove(0)
    }

    pub fn uneat_char(&mut self, ch: u8) {
        self.ch.insert(0, ch);
    }

    pub fn next_char(&mut self) -> Result<Option<u8>> {
        match self.ch.first() {
            Some(&ch) => {
                self.eat_char();
                Ok(Some(ch))
            }
            None => match self.next() {
                Some(Err(err)) => Err(Error::Io(err)),
                Some(Ok(ch)) => Ok(Some(ch)),
                None => Ok(None),
            },
        }
    }

    pub fn next_char_or_null(&mut self) -> Result<u8> {
        Ok(self.next_char()?.unwrap_or(b'\x00'))
    }

    fn eat_line(&mut self) -> Result<()> {
        loop {
            match self.peek()? {
                Some(b'\n') | None => return Ok(()),
                _ => {}
            }
            self.eat_char();
        }
    }

    pub fn parse_whitespace(&mut self) -> Result<()> {
        loop {
            match self.peek_or_null()? {
                b' ' | b'\n' | b'\t' | b'\r' => {
                    self.eat_char();
                }
                b'#' => self.eat_line()?,
                b'/' => {
                    match self.peek_next(1)? {
                        Some(b'/') => self.eat_line()?,
                        Some(b'*') => {
                            self.eat_char();
                            self.eat_char();
                            while !(self.peek()?.unwrap_or(b'*') == b'*'
                                && self.peek_next(1)?.unwrap_or(b'/') == b'/')
                            {
                                self.eat_char();
                            }
                            self.eat_char();
                            self.eat_char();
                        }
                        Some(_) => {
                            self.eat_char();
                        }
                        None => return Err(self.error(ErrorCode::TrailingCharacters)), //todo
                    }
                }
                _ => {
                    return Ok(());
                }
            }
        }
    }

    pub fn error(&mut self, reason: ErrorCode) -> Error {
        Error::Syntax(reason, self.line, self.col)
    }
}

pub enum Number {
    I64(i64),
    U64(u64),
    F64(f64),
}

pub struct ParseNumber<Iter: Iterator<Item = u8>> {
    rdr: StringReader<Iter>,
    result: Vec<u8>,
}

// macro_rules! try_or_invalid {
//     ($e:expr) => {
//         match $e {
//             Some(v) => v,
//             None => { return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0)); }
//         }
//     }
// }

impl<Iter: Iterator<Item = u8>> ParseNumber<Iter> {
    #[inline]
    pub fn new(iter: Iter) -> Self {
        ParseNumber {
            rdr: StringReader::new(iter),
            result: Vec::new(),
        }
    }

    pub fn parse(&mut self, stop_at_next: bool) -> Result<Number> {
        match self.try_parse() {
            Ok(()) => {
                self.rdr.parse_whitespace()?;

                let mut ch = self.rdr.next_char_or_null()?;

                if stop_at_next {
                    let ch2 = self.rdr.peek_or_null()?;
                    // end scan if we find a punctuator character like ,}] or a comment
                    if ch == b','
                        || ch == b'}'
                        || ch == b']'
                        || ch == b'#'
                        || ch == b'/' && (ch2 == b'/' || ch2 == b'*')
                    {
                        ch = b'\x00';
                    }
                }

                match ch {
                    b'\x00' => {
                        let res =
                            str::from_utf8(&self.result).expect("Internal error: json parsing");

                        let mut is_float = false;
                        for ch in res.chars() {
                            if ch == '.' || ch == 'e' || ch == 'E' {
                                is_float = true;
                                break;
                            }
                        }

                        if !is_float {
                            if res.starts_with('-') {
                                if let Ok(n) = res.parse::<i64>() {
                                    return Ok(Number::I64(n));
                                }
                            } else if let Ok(n) = res.parse::<u64>() {
                                return Ok(Number::U64(n));
                            }
                        }

                        match res.parse::<f64>() {
                            Ok(n) => Ok(Number::F64(n)),
                            _ => Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0)),
                        }
                    }
                    _ => Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0)),
                }
            }
            Err(e) => Err(e),
        }
    }

    fn try_parse(&mut self) -> Result<()> {
        if self.rdr.peek_or_null()? == b'-' {
            self.result.push(self.rdr.eat_char());
        }

        let mut has_value = false;

        if self.rdr.peek_or_null()? == b'0' {
            self.result.push(self.rdr.eat_char());
            has_value = true;

            // There can be only one leading '0'.
            if let b'0'..=b'9' = self.rdr.peek_or_null()? {
                return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0));
            }
        }

        loop {
            match self.rdr.peek_or_null()? {
                b'0'..=b'9' => {
                    self.result.push(self.rdr.eat_char());
                    has_value = true;
                }
                b'.' => {
                    if !has_value {
                        return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0));
                    }
                    self.rdr.eat_char();
                    return self.try_decimal();
                }
                b'e' | b'E' => {
                    if !has_value {
                        return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0));
                    }
                    self.rdr.eat_char();
                    return self.try_exponent();
                }
                _ => {
                    if !has_value {
                        return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0));
                    }
                    return Ok(());
                }
            }
        }
    }

    fn try_decimal(&mut self) -> Result<()> {
        self.result.push(b'.');

        // Make sure a digit follows the decimal place.
        match self.rdr.next_char_or_null()? {
            c @ b'0'..=b'9' => {
                self.result.push(c);
            }
            _ => {
                return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0));
            }
        };

        while let b'0'..=b'9' = self.rdr.peek_or_null()? {
            self.result.push(self.rdr.eat_char());
        }

        match self.rdr.peek_or_null()? {
            b'e' | b'E' => {
                self.rdr.eat_char();
                self.try_exponent()
            }
            _ => Ok(()),
        }
    }

    fn try_exponent(&mut self) -> Result<()> {
        self.result.push(b'e');

        match self.rdr.peek_or_null()? {
            b'+' => {
                self.result.push(self.rdr.eat_char());
            }
            b'-' => {
                self.result.push(self.rdr.eat_char());
            }
            _ => {}
        };

        // Make sure a digit follows the exponent place.
        match self.rdr.next_char_or_null()? {
            c @ b'0'..=b'9' => {
                self.result.push(c);
            }
            _ => {
                return Err(Error::Syntax(ErrorCode::InvalidNumber, 0, 0));
            }
        };

        while let b'0'..=b'9' = self.rdr.peek_or_null()? {
            self.result.push(self.rdr.eat_char());
        }

        Ok(())
    }
}
