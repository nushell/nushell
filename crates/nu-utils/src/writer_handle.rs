use std::{
    cell::RefCell,
    io::{self, Write},
    rc::Rc,
};

/// A cloneable handle to a single-threaded writer.
///
/// Clones of this handle write to the same underlying writer.
///
/// This type uses interior mutability and is not thread-safe.
/// It is not [`Send`] or [`Sync`].
/// 
/// # Example
/// 
/// ```rust
/// use std::io::Write;
/// use nu_utils::WriterHandle;
/// 
/// let mut bytes = Vec::new();
/// let mut a = WriterHandle::new(&mut bytes);
/// let mut b = a.clone();
/// a.write_all(b"abc").unwrap();
/// b.write_all(b"def").unwrap();
/// drop(a);
/// drop(b);
/// 
/// assert_eq!(bytes, b"abcdef");
/// ```
#[derive(Debug)]
pub struct WriterHandle<W>(Rc<RefCell<W>>);

impl<W> WriterHandle<W> {
    pub fn new(writer: W) -> Self {
        Self(Rc::new(RefCell::new(writer)))
    }
}

impl<W: Write> Write for WriterHandle<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.borrow_mut().flush()
    }
}

impl<W> Clone for WriterHandle<W> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
