use crate::hir::syntax_shape::FlatShape;
use ansi_term::Color;
use log::trace;
use nu_errors::ShellError;
use nu_source::{Spanned, Text};
use ptree::*;
use std::borrow::Cow;
use std::io;

#[derive(Debug, Clone)]
pub enum FrameChild {
    #[allow(unused)]
    Shape(Spanned<FlatShape>),
    Frame(ColorFrame),
}

impl FrameChild {
    fn colored_leaf_description(&self, text: &Text, f: &mut impl io::Write) -> io::Result<()> {
        match self {
            FrameChild::Shape(shape) => write!(
                f,
                "{} {:?}",
                Color::White
                    .bold()
                    .on(Color::Green)
                    .paint(format!("{:?}", shape.item)),
                shape.span.slice(text)
            ),

            FrameChild::Frame(frame) => frame.colored_leaf_description(f),
        }
    }

    fn into_tree_child(self, text: &Text) -> TreeChild {
        match self {
            FrameChild::Shape(shape) => TreeChild::Shape(shape, text.clone()),
            FrameChild::Frame(frame) => TreeChild::Frame(frame, text.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorFrame {
    description: &'static str,
    children: Vec<FrameChild>,
    error: Option<ShellError>,
}

impl ColorFrame {
    fn colored_leaf_description(&self, f: &mut impl io::Write) -> io::Result<()> {
        if self.has_only_error_descendents() {
            if self.children.is_empty() {
                write!(
                    f,
                    "{}",
                    Color::White.bold().on(Color::Red).paint(self.description)
                )
            } else {
                write!(f, "{}", Color::Red.normal().paint(self.description))
            }
        } else if self.has_descendent_shapes() {
            write!(f, "{}", Color::Green.normal().paint(self.description))
        } else {
            write!(f, "{}", Color::Yellow.bold().paint(self.description))
        }
    }

    fn colored_description(&self, text: &Text, f: &mut impl io::Write) -> io::Result<()> {
        if self.children.len() == 1 {
            let child = &self.children[0];

            self.colored_leaf_description(f)?;
            write!(f, " -> ")?;
            child.colored_leaf_description(text, f)
        } else {
            self.colored_leaf_description(f)
        }
    }

    fn children_for_formatting(&self, text: &Text) -> Vec<TreeChild> {
        if self.children.len() == 1 {
            let child = &self.children[0];

            match child {
                FrameChild::Shape(_) => vec![],
                FrameChild::Frame(frame) => frame.tree_children(text),
            }
        } else {
            self.tree_children(text)
        }
    }

    fn tree_children(&self, text: &Text) -> Vec<TreeChild> {
        self.children
            .clone()
            .into_iter()
            .map(|c| c.into_tree_child(text))
            .collect()
    }

    #[allow(unused)]
    fn add_shape(&mut self, shape: Spanned<FlatShape>) {
        self.children.push(FrameChild::Shape(shape))
    }

    fn has_child_shapes(&self) -> bool {
        self.any_child_shape(|_| true)
    }

    fn any_child_shape(&self, predicate: impl Fn(Spanned<FlatShape>) -> bool) -> bool {
        for item in &self.children {
            if let FrameChild::Shape(shape) = item {
                if predicate(*shape) {
                    return true;
                }
            }
        }

        false
    }

    fn any_child_frame(&self, predicate: impl Fn(&ColorFrame) -> bool) -> bool {
        for item in &self.children {
            if let FrameChild::Frame(frame) = item {
                if predicate(frame) {
                    return true;
                }
            }
        }

        false
    }

    fn has_descendent_shapes(&self) -> bool {
        if self.has_child_shapes() {
            true
        } else {
            self.any_child_frame(|frame| frame.has_descendent_shapes())
        }
    }

    fn has_only_error_descendents(&self) -> bool {
        if self.children.is_empty() {
            // if this frame has no children at all, it has only error descendents if this frame
            // is an error
            self.error.is_some()
        } else {
            // otherwise, it has only error descendents if all of its children terminate in an
            // error (transitively)

            let mut seen_error = false;

            for child in &self.children {
                match child {
                    // if this frame has at least one child shape, this frame has non-error descendents
                    FrameChild::Shape(_) => return false,
                    FrameChild::Frame(frame) => {
                        // if the chi
                        if frame.has_only_error_descendents() {
                            seen_error = true;
                        } else {
                            return false;
                        }
                    }
                }
            }

            seen_error
        }
    }
}

#[derive(Debug, Clone)]
pub enum TreeChild {
    Shape(Spanned<FlatShape>, Text),
    Frame(ColorFrame, Text),
}

impl TreeChild {
    fn colored_leaf_description(&self, f: &mut impl io::Write) -> io::Result<()> {
        match self {
            TreeChild::Shape(shape, text) => write!(
                f,
                "{} {:?}",
                Color::White
                    .bold()
                    .on(Color::Green)
                    .paint(format!("{:?}", shape.item)),
                shape.span.slice(text)
            ),

            TreeChild::Frame(frame, _) => frame.colored_leaf_description(f),
        }
    }
}

impl TreeItem for TreeChild {
    type Child = TreeChild;

    fn write_self<W: io::Write>(&self, f: &mut W, _style: &Style) -> io::Result<()> {
        match self {
            shape @ TreeChild::Shape(..) => shape.colored_leaf_description(f),

            TreeChild::Frame(frame, text) => frame.colored_description(text, f),
        }
    }

    fn children(&self) -> Cow<[Self::Child]> {
        match self {
            TreeChild::Shape(..) => Cow::Borrowed(&[]),
            TreeChild::Frame(frame, text) => Cow::Owned(frame.children_for_formatting(text)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorTracer {
    frame_stack: Vec<ColorFrame>,
    source: Text,
}

impl ColorTracer {
    pub fn print(self, source: Text) -> PrintTracer {
        PrintTracer {
            tracer: self,
            source,
        }
    }

    pub fn new(source: Text) -> ColorTracer {
        let root = ColorFrame {
            description: "Trace",
            children: vec![],
            error: None,
        };

        ColorTracer {
            frame_stack: vec![root],
            source,
        }
    }

    fn current_frame(&mut self) -> &mut ColorFrame {
        let frames = &mut self.frame_stack;
        let last = frames.len() - 1;
        &mut frames[last]
    }

    fn pop_frame(&mut self) -> ColorFrame {
        trace!(target: "nu::color_syntax", "Popping {:#?}", self);

        let result = self.frame_stack.pop().expect("Can't pop root tracer frame");

        if self.frame_stack.is_empty() {
            panic!("Can't pop root tracer frame {:#?}", self);
        }

        self.debug();

        result
    }

    pub fn start(&mut self, description: &'static str) {
        let frame = ColorFrame {
            description,
            children: vec![],
            error: None,
        };

        self.frame_stack.push(frame);
        self.debug();
    }

    pub fn eof_frame(&mut self) {
        let current = self.pop_frame();
        self.current_frame()
            .children
            .push(FrameChild::Frame(current));
    }

    #[allow(unused)]
    pub fn finish(&mut self) {
        loop {
            if self.frame_stack.len() == 1 {
                break;
            }

            let frame = self.pop_frame();
            self.current_frame().children.push(FrameChild::Frame(frame));
        }
    }

    #[allow(unused)]
    pub fn add_shape(&mut self, shape: Spanned<FlatShape>) {
        self.current_frame().add_shape(shape);
    }

    pub fn success(&mut self) {
        let current = self.pop_frame();
        self.current_frame()
            .children
            .push(FrameChild::Frame(current));
    }

    pub fn failed(&mut self, error: &ShellError) {
        let mut current = self.pop_frame();
        current.error = Some(error.clone());
        self.current_frame()
            .children
            .push(FrameChild::Frame(current));
    }

    fn debug(&self) {
        trace!(target: "nu::color_syntax",
            "frames = {:?}",
            self.frame_stack
                .iter()
                .map(|f| f.description)
                .collect::<Vec<_>>()
        );

        trace!(target: "nu::color_syntax", "{:#?}", self);
    }
}

#[derive(Debug, Clone)]
pub struct PrintTracer {
    tracer: ColorTracer,
    source: Text,
}

impl TreeItem for PrintTracer {
    type Child = TreeChild;

    fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
        write!(f, "{}", style.paint("Color Trace"))
    }

    fn children(&self) -> Cow<[Self::Child]> {
        Cow::Owned(vec![TreeChild::Frame(
            self.tracer.frame_stack[0].clone(),
            self.source.clone(),
        )])
    }
}
