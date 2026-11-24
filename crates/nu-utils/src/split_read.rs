use std::io::{BufRead, ErrorKind};

use memchr::memmem::Finder;

pub struct SplitRead<R> {
    reader: Option<R>,
    buf: Option<Vec<u8>>,
    start: usize,
    finder: Finder<'static>,
}

impl<R: BufRead> SplitRead<R> {
    pub fn new(reader: R, delim: impl AsRef<[u8]>) -> Self {
        // empty delimiter results in an infinite stream of empty items
        debug_assert!(!delim.as_ref().is_empty(), "delimiter can't be empty");
        Self {
            reader: Some(reader),
            buf: Some(Vec::new()),
            start: 0,
            finder: Finder::new(delim.as_ref()).into_owned(),
        }
    }

    /// Whether the iterator has returned its final segment or a read error.
    pub fn is_finished(&self) -> bool {
        self.buf.is_none()
    }

    /// Discard the remaining bytes and report any error encountered before EOF.
    pub fn drain(self) -> std::io::Result<()> {
        let Some(mut reader) = self.reader else {
            return Ok(());
        };
        loop {
            match reader.fill_buf() {
                Ok([]) => return Ok(()),
                Ok(bytes) => {
                    let len = bytes.len();
                    reader.consume(len);
                }
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }
    }
}

impl<R: BufRead> Iterator for SplitRead<R> {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let buf = self.buf.as_mut()?;
        let mut search_start = self.start;

        loop {
            if let Some(i) = self.finder.find(&buf[search_start..]) {
                let end = search_start + i;
                let next_start = end + self.finder.needle().len();
                if self.start == 0 && end >= buf.len() - next_start {
                    // Give a large segment the existing allocation and copy the smaller tail.
                    let tail = buf.split_off(next_start);
                    let mut segment = std::mem::replace(buf, tail);
                    segment.truncate(end);
                    return Some(Ok(segment));
                }
                let segment = buf[self.start..end].to_vec();
                self.start = next_start;
                return Some(Ok(segment));
            }

            if let Some(r) = self.reader.as_mut() {
                // Compact consumed bytes once per refill, even when it contains many segments.
                if self.start != 0 {
                    buf.drain(..self.start);
                    self.start = 0;
                }
                search_start = buf.len().saturating_sub(self.finder.needle().len() - 1);
                let available = match r.fill_buf() {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => {
                        self.reader = None;
                        self.buf = None;
                        return Some(Err(e));
                    }
                };

                buf.extend_from_slice(available);
                let used = available.len();
                r.consume(used);
                if used == 0 {
                    self.reader = None;
                }
                continue;
            } else {
                return self.buf.take().map(Ok);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor, Read};

    #[test]
    fn simple() {
        let s = "foo-bar-baz";
        let cursor = Cursor::new(String::from(s));
        let mut split = SplitRead::new(cursor, "-").map(|r| String::from_utf8(r.unwrap()).unwrap());

        assert_eq!(split.next().as_deref(), Some("foo"));
        assert_eq!(split.next().as_deref(), Some("bar"));
        assert_eq!(split.next().as_deref(), Some("baz"));
        assert_eq!(split.next(), None);
    }

    #[test]
    fn with_empty_fields() -> Result<(), io::Error> {
        let s = "\0\0foo\0\0bar\0\0\0\0baz\0\0";
        let cursor = Cursor::new(String::from(s));
        let mut split =
            SplitRead::new(cursor, "\0\0").map(|r| String::from_utf8(r.unwrap()).unwrap());

        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), Some("foo"));
        assert_eq!(split.next().as_deref(), Some("bar"));
        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), Some("baz"));
        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), None);

        Ok(())
    }

    #[test]
    fn complex_delimiter() -> Result<(), io::Error> {
        let s = "<|>foo<|>bar<|><|>baz<|>";
        let cursor = Cursor::new(String::from(s));
        let mut split =
            SplitRead::new(cursor, "<|>").map(|r| String::from_utf8(r.unwrap()).unwrap());

        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), Some("foo"));
        assert_eq!(split.next().as_deref(), Some("bar"));
        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), Some("baz"));
        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), None);

        Ok(())
    }

    #[test]
    fn all_empty() -> Result<(), io::Error> {
        let s = "<><>";
        let cursor = Cursor::new(String::from(s));
        let mut split =
            SplitRead::new(cursor, "<>").map(|r| String::from_utf8(r.unwrap()).unwrap());

        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next().as_deref(), Some(""));
        assert_eq!(split.next(), None);

        Ok(())
    }

    #[should_panic = "delimiter can't be empty"]
    #[test]
    fn empty_delimiter() {
        let s = "abc";
        let cursor = Cursor::new(String::from(s));
        let _split = SplitRead::new(cursor, "").map(|e| e.unwrap());
    }

    #[test]
    fn delimiter_spread_across_reads() {
        let reader = Cursor::new("<|>foo<|")
            .chain(Cursor::new(">bar<|><"))
            .chain(Cursor::new("|>baz<|>"));

        let mut split =
            SplitRead::new(reader, "<|>").map(|r| String::from_utf8(r.unwrap()).unwrap());

        assert_eq!(split.next().unwrap(), "");
        assert_eq!(split.next().unwrap(), "foo");
        assert_eq!(split.next().unwrap(), "bar");
        assert_eq!(split.next().unwrap(), "");
        assert_eq!(split.next().unwrap(), "baz");
        assert_eq!(split.next().unwrap(), "");
        assert_eq!(split.next(), None);
    }

    #[test]
    fn retries_interrupted_read() {
        struct InterruptedOnce(Cursor<&'static str>, bool);

        impl Read for InterruptedOnce {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if std::mem::replace(&mut self.1, false) {
                    return Err(ErrorKind::Interrupted.into());
                }
                self.0.read(buf)
            }
        }

        let reader = io::BufReader::new(InterruptedOnce(Cursor::new("a--b"), true));
        let mut split = SplitRead::new(reader, "--");
        assert_eq!(split.next().unwrap().unwrap(), b"a");
        assert_eq!(split.next().unwrap().unwrap(), b"b");
        assert!(split.is_finished());
        assert!(split.next().is_none());
    }
}
