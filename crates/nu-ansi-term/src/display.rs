use crate::ansi::RESET;
use crate::difference::Difference;
use crate::style::{Color, Style};
use crate::write::AnyWrite;
use std::borrow::Cow;
use std::fmt;
use std::io;
use std::ops::Deref;

/// An `ANSIGenericString` includes a generic string type and a `Style` to
/// display that string.  `ANSIString` and `ANSIByteString` are aliases for
/// this type on `str` and `\[u8]`, respectively.
#[derive(PartialEq, Debug)]
pub struct AnsiGenericString<'a, S: 'a + ToOwned + ?Sized>
where
    <S as ToOwned>::Owned: fmt::Debug,
{
    style: Style,
    string: Cow<'a, S>,
}

/// Cloning an `ANSIGenericString` will clone its underlying string.
///
/// # Examples
///
/// ```
/// use nu_ansi_term::ANSIString;
///
/// let plain_string = ANSIString::from("a plain string");
/// let clone_string = plain_string.clone();
/// assert_eq!(clone_string, plain_string);
/// ```
impl<'a, S: 'a + ToOwned + ?Sized> Clone for AnsiGenericString<'a, S>
where
    <S as ToOwned>::Owned: fmt::Debug,
{
    fn clone(&self) -> AnsiGenericString<'a, S> {
        AnsiGenericString {
            style: self.style,
            string: self.string.clone(),
        }
    }
}

// You might think that the hand-written Clone impl above is the same as the
// one that gets generated with #[derive]. But it’s not *quite* the same!
//
// `str` is not Clone, and the derived Clone implementation puts a Clone
// constraint on the S type parameter (generated using --pretty=expanded):
//
//                  ↓_________________↓
//     impl <'a, S: ::std::clone::Clone + 'a + ToOwned + ?Sized> ::std::clone::Clone
//     for ANSIGenericString<'a, S> where
//     <S as ToOwned>::Owned: fmt::Debug { ... }
//
// This resulted in compile errors when you tried to derive Clone on a type
// that used it:
//
//     #[derive(PartialEq, Debug, Clone, Default)]
//     pub struct TextCellContents(Vec<ANSIString<'static>>);
//                                 ^^^^^^^^^^^^^^^^^^^^^^^^^
//     error[E0277]: the trait `std::clone::Clone` is not implemented for `str`
//
// The hand-written impl above can ignore that constraint and still compile.

/// An ANSI String is a string coupled with the `Style` to display it
/// in a terminal.
///
/// Although not technically a string itself, it can be turned into
/// one with the `to_string` method.
///
/// # Examples
///
/// ```
/// use nu_ansi_term::ANSIString;
/// use nu_ansi_term::Color::Red;
///
/// let red_string = Red.paint("a red string");
/// println!("{}", red_string);
/// ```
///
/// ```
/// use nu_ansi_term::ANSIString;
///
/// let plain_string = ANSIString::from("a plain string");
/// assert_eq!(&*plain_string, "a plain string");
/// ```
pub type AnsiString<'a> = AnsiGenericString<'a, str>;

/// An `AnsiByteString` represents a formatted series of bytes.  Use
/// `AnsiByteString` when styling text with an unknown encoding.
pub type AnsiByteString<'a> = AnsiGenericString<'a, [u8]>;

impl<'a, I, S: 'a + ToOwned + ?Sized> From<I> for AnsiGenericString<'a, S>
where
    I: Into<Cow<'a, S>>,
    <S as ToOwned>::Owned: fmt::Debug,
{
    fn from(input: I) -> AnsiGenericString<'a, S> {
        AnsiGenericString {
            string: input.into(),
            style: Style::default(),
        }
    }
}

impl<'a, S: 'a + ToOwned + ?Sized> AnsiGenericString<'a, S>
where
    <S as ToOwned>::Owned: fmt::Debug,
{
    /// Directly access the style
    pub fn style_ref(&self) -> &Style {
        &self.style
    }

    /// Directly access the style mutably
    pub fn style_ref_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl<'a, S: 'a + ToOwned + ?Sized> Deref for AnsiGenericString<'a, S>
where
    <S as ToOwned>::Owned: fmt::Debug,
{
    type Target = S;

    fn deref(&self) -> &S {
        self.string.deref()
    }
}

/// A set of `AnsiGenericStrings`s collected together, in order to be
/// written with a minimum of control characters.
#[derive(Debug, PartialEq)]
pub struct AnsiGenericStrings<'a, S: 'a + ToOwned + ?Sized>(pub &'a [AnsiGenericString<'a, S>])
where
    <S as ToOwned>::Owned: fmt::Debug,
    S: PartialEq;

/// A set of `AnsiString`s collected together, in order to be written with a
/// minimum of control characters.
pub type AnsiStrings<'a> = AnsiGenericStrings<'a, str>;

/// A function to construct an `AnsiStrings` instance.
#[allow(non_snake_case)]
pub fn AnsiStrings<'a>(arg: &'a [AnsiString<'a>]) -> AnsiStrings<'a> {
    AnsiGenericStrings(arg)
}

/// A set of `AnsiByteString`s collected together, in order to be
/// written with a minimum of control characters.
pub type AnsiByteStrings<'a> = AnsiGenericStrings<'a, [u8]>;

/// A function to construct an `ANSIByteStrings` instance.
#[allow(non_snake_case)]
pub fn ANSIByteStrings<'a>(arg: &'a [AnsiByteString<'a>]) -> AnsiByteStrings<'a> {
    AnsiGenericStrings(arg)
}

// ---- paint functions ----

impl Style {
    /// Paints the given text with this color, returning an ANSI string.
    #[must_use]
    pub fn paint<'a, I, S: 'a + ToOwned + ?Sized>(self, input: I) -> AnsiGenericString<'a, S>
    where
        I: Into<Cow<'a, S>>,
        <S as ToOwned>::Owned: fmt::Debug,
    {
        AnsiGenericString {
            string: input.into(),
            style: self,
        }
    }
}

impl Color {
    /// Paints the given text with this color, returning an ANSI string.
    /// This is a short-cut so you don’t have to use `Blue.normal()` just
    /// to get blue text.
    ///
    /// ```
    /// use nu_ansi_term::Color::Blue;
    /// println!("{}", Blue.paint("da ba dee"));
    /// ```
    #[must_use]
    pub fn paint<'a, I, S: 'a + ToOwned + ?Sized>(self, input: I) -> AnsiGenericString<'a, S>
    where
        I: Into<Cow<'a, S>>,
        <S as ToOwned>::Owned: fmt::Debug,
    {
        AnsiGenericString {
            string: input.into(),
            style: self.normal(),
        }
    }
}

// ---- writers for individual ANSI strings ----

impl<'a> fmt::Display for AnsiString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let w: &mut dyn fmt::Write = f;
        self.write_to_any(w)
    }
}

impl<'a> AnsiByteString<'a> {
    /// Write an `ANSIByteString` to an `io::Write`.  This writes the escape
    /// sequences for the associated `Style` around the bytes.
    pub fn write_to<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        let w: &mut dyn io::Write = w;
        self.write_to_any(w)
    }
}

impl<'a, S: 'a + ToOwned + ?Sized> AnsiGenericString<'a, S>
where
    <S as ToOwned>::Owned: fmt::Debug,
    &'a S: AsRef<[u8]>,
{
    fn write_to_any<W: AnyWrite<Wstr = S> + ?Sized>(&self, w: &mut W) -> Result<(), W::Error> {
        write!(w, "{}", self.style.prefix())?;
        w.write_str(self.string.as_ref())?;
        write!(w, "{}", self.style.suffix())
    }
}

// ---- writers for combined ANSI strings ----

impl<'a> fmt::Display for AnsiStrings<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let f: &mut dyn fmt::Write = f;
        self.write_to_any(f)
    }
}

impl<'a> AnsiByteStrings<'a> {
    /// Write `ANSIByteStrings` to an `io::Write`.  This writes the minimal
    /// escape sequences for the associated `Style`s around each set of
    /// bytes.
    pub fn write_to<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        let w: &mut dyn io::Write = w;
        self.write_to_any(w)
    }
}

impl<'a, S: 'a + ToOwned + ?Sized + PartialEq> AnsiGenericStrings<'a, S>
where
    <S as ToOwned>::Owned: fmt::Debug,
    &'a S: AsRef<[u8]>,
{
    fn write_to_any<W: AnyWrite<Wstr = S> + ?Sized>(&self, w: &mut W) -> Result<(), W::Error> {
        use self::Difference::*;

        let first = match self.0.first() {
            None => return Ok(()),
            Some(f) => f,
        };

        write!(w, "{}", first.style.prefix())?;
        w.write_str(first.string.as_ref())?;

        for window in self.0.windows(2) {
            match Difference::between(&window[0].style, &window[1].style) {
                ExtraStyles(style) => write!(w, "{}", style.prefix())?,
                Reset => write!(w, "{}{}", RESET, window[1].style.prefix())?,
                Empty => { /* Do nothing! */ }
            }

            w.write_str(&window[1].string)?;
        }

        // Write the final reset string after all of the ANSIStrings have been
        // written, *except* if the last one has no styles, because it would
        // have already been written by this point.
        if let Some(last) = self.0.last() {
            if !last.style.is_plain() {
                write!(w, "{}", RESET)?;
            }
        }

        Ok(())
    }
}

// ---- tests ----

#[cfg(test)]
mod tests {
    pub use super::super::AnsiStrings;
    pub use crate::style::Color::*;
    pub use crate::style::Style;

    #[test]
    fn no_control_codes_for_plain() {
        let one = Style::default().paint("one");
        let two = Style::default().paint("two");
        let output = format!("{}", AnsiStrings(&[one, two]));
        assert_eq!(&*output, "onetwo");
    }
}
