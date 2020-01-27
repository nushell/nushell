use crate::hir::syntax_shape::flat_shape::TraceShape;
use crate::hir::SpannedExpression;
use crate::parse::token_tree::SpannedToken;
use ansi_term::Color;
use log::trace;
use nu_errors::{ParseError, ParseErrorReason};
use nu_protocol::{ShellTypeName, SpannedTypeName};
use nu_source::{DebugDoc, PrettyDebug, PrettyDebugWithSource, Span, Spanned, Text};
use ptree::*;
use std::borrow::Cow;
use std::fmt::Debug;
use std::io;

#[derive(Debug, Clone)]
pub enum FrameChild<T: SpannedTypeName> {
    Expr(T),
    Shape(Result<TraceShape, TraceShape>),
    Frame(Box<ExprFrame<T>>),
    Result(DebugDoc),
}

fn err_desc(error: &ParseError) -> &'static str {
    match error.reason() {
        ParseErrorReason::ExtraTokens { .. } => "extra tokens",
        ParseErrorReason::Mismatch { .. } => "mismatch",
        ParseErrorReason::ArgumentError { .. } => "argument error",
        ParseErrorReason::Eof { .. } => "eof",
        ParseErrorReason::InternalError { .. } => "internal error",
    }
}

impl<T: SpannedTypeName> FrameChild<T> {
    fn get_error_leaf(&self) -> Option<(&'static str, &'static str)> {
        match self {
            FrameChild::Frame(frame) => {
                if let Some(error) = &frame.error {
                    if frame.children.is_empty() {
                        Some((frame.description, err_desc(error)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn to_tree_child(&self, text: &Text) -> TreeChild {
        match self {
            FrameChild::Expr(expr) => TreeChild::OkExpr {
                source: expr.spanned_type_name().span,
                desc: expr.spanned_type_name().item,
                text: text.clone(),
            },
            FrameChild::Shape(Ok(shape)) => TreeChild::OkShape {
                source: shape.spanned_type_name().span,
                desc: shape.spanned_type_name().item,
                text: text.clone(),
                fallback: false,
            },
            FrameChild::Shape(Err(shape)) => TreeChild::OkShape {
                source: shape.spanned_type_name().span,
                desc: shape.spanned_type_name().item,
                text: text.clone(),
                fallback: true,
            },
            FrameChild::Result(result) => {
                let result = result.display();
                TreeChild::OkNonExpr(result)
            }
            FrameChild::Frame(frame) => {
                if let Some(err) = &frame.error {
                    if frame.children.is_empty() {
                        TreeChild::ErrorLeaf(
                            vec![(frame.description, err_desc(err))],
                            frame.token_desc(),
                        )
                    } else {
                        TreeChild::ErrorFrame(frame.to_tree_frame(text), text.clone())
                    }
                } else {
                    TreeChild::OkFrame(frame.to_tree_frame(text), text.clone())
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExprFrame<T: SpannedTypeName> {
    description: &'static str,
    token: Option<SpannedToken>,
    children: Vec<FrameChild<T>>,
    error: Option<ParseError>,
}

impl<T: SpannedTypeName> ExprFrame<T> {
    fn token_desc(&self) -> &'static str {
        match &self.token {
            None => "EOF",
            Some(token) => token.type_name(),
        }
    }

    fn to_tree_frame(&self, text: &Text) -> TreeFrame {
        let mut children = vec![];
        let mut errors = vec![];

        for child in &self.children {
            if let Some(error_leaf) = child.get_error_leaf() {
                errors.push(error_leaf);
                continue;
            } else if !errors.is_empty() {
                children.push(TreeChild::ErrorLeaf(errors, self.token_desc()));
                errors = vec![];
            }

            children.push(child.to_tree_child(text));
        }

        if !errors.is_empty() {
            children.push(TreeChild::ErrorLeaf(errors, self.token_desc()));
        }

        TreeFrame {
            description: self.description,
            token_desc: self.token_desc(),
            children,
            error: self.error.clone(),
        }
    }

    fn add_return(&mut self, value: T) {
        self.children.push(FrameChild::Expr(value))
    }

    fn add_shape(&mut self, shape: TraceShape) {
        self.children.push(FrameChild::Shape(Ok(shape)))
    }

    fn add_err_shape(&mut self, shape: TraceShape) {
        self.children.push(FrameChild::Shape(Err(shape)))
    }

    fn add_result(&mut self, result: impl PrettyDebug) {
        self.children.push(FrameChild::Result(result.to_doc()))
    }
}

#[derive(Debug, Clone)]
pub struct TreeFrame {
    description: &'static str,
    token_desc: &'static str,
    children: Vec<TreeChild>,
    error: Option<ParseError>,
}

impl TreeFrame {
    fn leaf_description(&self, f: &mut impl io::Write) -> io::Result<()> {
        if self.children.len() == 1 {
            if self.error.is_some() {
                write!(f, "{}", Color::Red.normal().paint(self.description))?;
            } else if self.has_descendent_green() {
                write!(f, "{}", Color::Green.normal().paint(self.description))?;
            } else {
                write!(f, "{}", Color::Yellow.bold().paint(self.description))?;
            }

            write!(
                f,
                "{}",
                Color::White.bold().paint(&format!("({})", self.token_desc))
            )?;

            write!(f, " -> ")?;
            self.children[0].leaf_description(f)
        } else {
            if self.error.is_some() {
                if self.children.is_empty() {
                    write!(
                        f,
                        "{}",
                        Color::White.bold().on(Color::Red).paint(self.description)
                    )?
                } else {
                    write!(f, "{}", Color::Red.normal().paint(self.description))?
                }
            } else if self.has_descendent_green() {
                write!(f, "{}", Color::Green.normal().paint(self.description))?
            } else {
                write!(f, "{}", Color::Yellow.bold().paint(self.description))?
            }

            write!(
                f,
                "{}",
                Color::White.bold().paint(&format!("({})", self.token_desc))
            )
        }
    }

    fn has_child_green(&self) -> bool {
        self.children.iter().any(|item| match item {
            TreeChild::OkFrame(..) | TreeChild::ErrorFrame(..) | TreeChild::ErrorLeaf(..) => false,
            TreeChild::OkExpr { .. } | TreeChild::OkShape { .. } | TreeChild::OkNonExpr(..) => true,
        })
    }

    fn any_child_frame(&self, predicate: impl Fn(&TreeFrame) -> bool) -> bool {
        for item in &self.children {
            if let TreeChild::OkFrame(frame, ..) = item {
                if predicate(frame) {
                    return true;
                }
            }
        }

        false
    }

    fn has_descendent_green(&self) -> bool {
        if self.has_child_green() {
            true
        } else {
            self.any_child_frame(|frame| frame.has_child_green())
        }
    }

    fn children_for_formatting(&self, text: &Text) -> Vec<TreeChild> {
        if self.children.len() == 1 {
            let child: &TreeChild = &self.children[0];
            match child {
                TreeChild::OkExpr { .. }
                | TreeChild::OkShape { .. }
                | TreeChild::OkNonExpr(..)
                | TreeChild::ErrorLeaf(..) => vec![],
                TreeChild::OkFrame(frame, _) | TreeChild::ErrorFrame(frame, _) => {
                    frame.children_for_formatting(text)
                }
            }
        } else {
            self.children.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub enum TreeChild {
    OkNonExpr(String),
    OkExpr {
        source: Span,
        desc: &'static str,
        text: Text,
    },
    OkShape {
        source: Span,
        desc: &'static str,
        text: Text,
        fallback: bool,
    },
    OkFrame(TreeFrame, Text),
    ErrorFrame(TreeFrame, Text),
    ErrorLeaf(Vec<(&'static str, &'static str)>, &'static str),
}

impl TreeChild {
    fn leaf_description(&self, f: &mut impl io::Write) -> io::Result<()> {
        match self {
            TreeChild::OkExpr { source, desc, text } => write!(
                f,
                "{} {} {}",
                Color::Cyan.normal().paint("returns"),
                Color::White.bold().on(Color::Green).paint(*desc),
                source.slice(text)
            ),

            TreeChild::OkShape {
                source,
                desc,
                text,
                fallback,
            } => write!(
                f,
                "{} {} {}",
                Color::Purple.normal().paint("paints"),
                Color::White.bold().on(Color::Green).paint(*desc),
                source.slice(text)
            ),

            TreeChild::OkNonExpr(result) => write!(
                f,
                "{} {}",
                Color::Cyan.normal().paint("returns"),
                Color::White
                    .bold()
                    .on(Color::Green)
                    .paint(result.to_string())
            ),

            TreeChild::ErrorLeaf(desc, token_desc) => {
                let last = desc.len() - 1;

                for (i, (desc, err_desc)) in desc.iter().enumerate() {
                    write!(f, "{}", Color::White.bold().on(Color::Red).paint(*desc))?;

                    write!(f, " {}", Color::White.bold().paint(*err_desc))?;

                    if i != last {
                        write!(f, "{}", Color::White.normal().paint(", "))?;
                    }
                }

                // write!(f, " {}", Color::Black.bold().paint(*token_desc))?;

                Ok(())
            }

            TreeChild::ErrorFrame(frame, _) | TreeChild::OkFrame(frame, _) => {
                frame.leaf_description(f)
            }
        }
    }
}

impl TreeItem for TreeChild {
    type Child = TreeChild;

    fn write_self<W: io::Write>(&self, f: &mut W, _style: &Style) -> io::Result<()> {
        self.leaf_description(f)
    }

    fn children(&self) -> Cow<[Self::Child]> {
        match self {
            TreeChild::OkExpr { .. }
            | TreeChild::OkShape { .. }
            | TreeChild::OkNonExpr(..)
            | TreeChild::ErrorLeaf(..) => Cow::Borrowed(&[]),
            TreeChild::OkFrame(frame, text) | TreeChild::ErrorFrame(frame, text) => {
                Cow::Owned(frame.children_for_formatting(text))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpandTracer<T: SpannedTypeName> {
    desc: &'static str,
    frame_stack: Vec<ExprFrame<T>>,
    source: Text,
}

impl<T: SpannedTypeName + Debug> ExpandTracer<T> {
    pub fn print(&self, source: Text) -> PrintTracer {
        let root = self.frame_stack[0].to_tree_frame(&source);

        PrintTracer {
            root,
            desc: self.desc,
            source,
        }
    }

    pub fn new(desc: &'static str, source: Text) -> ExpandTracer<T> {
        let root = ExprFrame {
            description: "Trace",
            children: vec![],
            token: None,
            error: None,
        };

        ExpandTracer {
            desc,
            frame_stack: vec![root],
            source,
        }
    }

    fn current_frame(&mut self) -> &mut ExprFrame<T> {
        let frames = &mut self.frame_stack;
        let last = frames.len() - 1;
        &mut frames[last]
    }

    fn pop_frame(&mut self) -> ExprFrame<T> {
        let result = self.frame_stack.pop().expect("Can't pop root tracer frame");

        if self.frame_stack.is_empty() {
            panic!("Can't pop root tracer frame");
        }

        self.debug();

        result
    }

    pub fn start(&mut self, description: &'static str, token: Option<SpannedToken>) {
        let frame = ExprFrame {
            description,
            children: vec![],
            token,
            error: None,
        };

        self.frame_stack.push(frame);
        self.debug();
    }

    pub fn add_return(&mut self, value: T) {
        self.current_frame().add_return(value);
    }

    pub fn add_shape(&mut self, shape: TraceShape) {
        self.current_frame().add_shape(shape);
    }

    pub fn add_err_shape(&mut self, shape: TraceShape) {
        self.current_frame().add_err_shape(shape);
    }

    pub fn finish(&mut self) {
        loop {
            if self.frame_stack.len() == 1 {
                break;
            }

            let frame = self.pop_frame();
            self.current_frame()
                .children
                .push(FrameChild::Frame(Box::new(frame)));
        }
    }

    pub fn eof_frame(&mut self) {
        let current = self.pop_frame();
        self.current_frame()
            .children
            .push(FrameChild::Frame(Box::new(current)));
    }

    pub fn add_result(&mut self, result: impl PrettyDebugWithSource) {
        let source = self.source.clone();
        self.current_frame().add_result(result.debuggable(source));
    }

    pub fn success(&mut self) {
        trace!(target: "parser::expand_syntax", "success {:#?}", self);

        let current = self.pop_frame();
        self.current_frame()
            .children
            .push(FrameChild::Frame(Box::new(current)));
    }

    pub fn failed(&mut self, error: &ParseError) {
        let mut current = self.pop_frame();
        current.error = Some(error.clone());
        self.current_frame()
            .children
            .push(FrameChild::Frame(Box::new(current)));
    }

    fn debug(&self) {
        trace!(target: "nu::parser::expand",
            "frames = {:?}",
            self.frame_stack
                .iter()
                .map(|f| f.description)
                .collect::<Vec<_>>()
        );

        trace!(target: "nu::parser::expand", "{:#?}", self);
    }
}

#[derive(Debug, Clone)]
pub struct PrintTracer {
    desc: &'static str,
    root: TreeFrame,
    source: Text,
}

impl TreeItem for PrintTracer {
    type Child = TreeChild;

    fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
        write!(f, "{}", style.paint(self.desc))
    }

    fn children(&self) -> Cow<[Self::Child]> {
        Cow::Borrowed(&self.root.children)
    }
}
