use crate::traits::ShellAnnotation;
use pretty::{Render, RenderAnnotated};
use std::io;
use termcolor::WriteColor;

pub struct TermColored<'a, W> {
    color_stack: Vec<ShellAnnotation>,
    upstream: &'a mut W,
}

impl<'a, W> TermColored<'a, W> {
    pub fn new(upstream: &'a mut W) -> TermColored<'a, W> {
        TermColored {
            color_stack: Vec::new(),
            upstream,
        }
    }
}

impl<'a, W> Render for TermColored<'a, W>
where
    W: io::Write,
{
    type Error = io::Error;

    fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.upstream.write(s.as_bytes())
    }

    fn write_str_all(&mut self, s: &str) -> io::Result<()> {
        self.upstream.write_all(s.as_bytes())
    }
}

impl<'a, W> RenderAnnotated<ShellAnnotation> for TermColored<'a, W>
where
    W: WriteColor,
{
    fn push_annotation(&mut self, ann: &ShellAnnotation) -> Result<(), Self::Error> {
        self.color_stack.push(*ann);
        self.upstream.set_color(&(*ann).into())
    }

    fn pop_annotation(&mut self) -> Result<(), Self::Error> {
        self.color_stack.pop();
        match self.color_stack.last() {
            Some(previous) => self.upstream.set_color(&(*previous).into()),
            None => self.upstream.reset(),
        }
    }
}
