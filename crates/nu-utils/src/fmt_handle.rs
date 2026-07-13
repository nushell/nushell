use std::{cell::RefCell, fmt, rc::Rc};

/// A cloneable handle to a single-threaded [`fmt`] writer.
///  
/// Clones of this handle write to the same underlying writer.
/// This type uses interior mutability and is not thread-safe, i.e. it is not [Send] nor [Sync].
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use nu_utils::FmtHandle;
///
/// let mut string = String::new();
/// let mut a = FmtHandle::new(&mut string);
/// let mut b = a.clone();
/// a.write_str("abc").unwrap();
/// b.write_str("def").unwrap();
/// drop(a);
/// drop(b);
///
/// assert_eq!(string, "abcdef");
/// ```
#[derive(Debug)]
pub struct FmtHandle<W>(Rc<RefCell<W>>);

impl<W> FmtHandle<W> {
    pub fn new(writer: W) -> Self {
        Self(Rc::new(RefCell::new(writer)))
    }
}

impl<W: Default> FmtHandle<W> {
    pub fn take(&self) -> W {
        self.0.take()
    }
}

impl<W> Clone for FmtHandle<W> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<W: fmt::Write> fmt::Write for FmtHandle<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.borrow_mut().write_str(s)
    }
}
