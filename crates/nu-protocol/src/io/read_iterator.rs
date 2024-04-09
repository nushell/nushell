use std::io::{self, Cursor, Read};

use crate::ShellError;

pub struct ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]>,
{
    iter: I,
    cursor: Option<Cursor<I::Item>>,
}

impl<I> ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]> + Default,
{
    pub fn new(iter: I) -> Self {
        Self::with_empty(iter, I::Item::default())
    }
}

impl<I> ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]>,
{
    pub fn with_empty(iter: I, empty: I::Item) -> Self {
        Self {
            iter: iter.into_iter(),
            cursor: Some(Cursor::new(empty)),
        }
    }
}

impl<I> Read for ReadIterator<I>
where
    I: Iterator,
    I::Item: AsRef<[u8]>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while let Some(cursor) = self.cursor.as_mut() {
            let read = cursor.read(buf)?;
            if read == 0 {
                self.cursor = self.iter.next().map(Cursor::new);
            } else {
                return Ok(read);
            }
        }
        Ok(0)
    }
}

pub struct ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]>,
{
    iter: I,
    cursor: Option<Cursor<T>>,
}

impl<I, T> ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]> + Default,
{
    pub fn new(iter: I) -> Self {
        Self::with_empty(iter, T::default())
    }
}

impl<I, T> ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]>,
{
    pub fn with_empty(iter: I, empty: T) -> Self {
        Self {
            iter: iter.into_iter(),
            cursor: Some(Cursor::new(empty)),
        }
    }
}

impl<I, T> Read for ReadResultIterator<I, T>
where
    I: Iterator<Item = Result<T, ShellError>>,
    T: AsRef<[u8]>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while let Some(cursor) = self.cursor.as_mut() {
            let read = cursor.read(buf)?;
            if read == 0 {
                self.cursor = self.iter.next().transpose()?.map(Cursor::new);
            } else {
                return Ok(read);
            }
        }
        Ok(0)
    }
}
