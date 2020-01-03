use crate::hir::Expression;
use ansi_term::Color;
use log::trace;
use nu_errors::ParseError;
use nu_protocol::ShellTypeName;
use nu_source::{DebugDoc, PrettyDebug, PrettyDebugWithSource, Text};
use ptree::*;
use std::borrow::Cow;
use std::io;

#[derive(Debug)]
pub enum FrameChild {
    Expr(Expression),
    Frame(ExprFrame),
    Result(DebugDoc),
}

impl FrameChild {
    fn get_error_leaf(&self) -> Option<&'static str> {
        match self {
            FrameChild::Frame(frame) if frame.error.is_some() => {
                if frame.children.is_empty() {
                    Some(frame.description)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn to_tree_child(&self, text: &Text) -> TreeChild {
        match self {
            FrameChild::Expr(expr) => TreeChild::OkExpr(expr.clone(), text.clone()),
            FrameChild::Result(result) => {
                let result = result.display();
                TreeChild::OkNonExpr(result)
            }
            FrameChild::Frame(frame) => {
                if frame.error.is_some() {
                    if frame.children.is_empty() {
                        TreeChild::ErrorLeaf(vec![frame.description])
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

#[derive(Debug)]
pub struct ExprFrame {
    description: &'static str,
    children: Vec<FrameChild>,
    error: Option<ParseError>,
}

impl ExprFrame {
    fn to_tree_frame(&self, text: &Text) -> TreeFrame {
        let mut children = vec![];
        let mut errors = vec![];

        for child in &self.children {
            if let Some(error_leaf) = child.get_error_leaf() {
                errors.push(error_leaf);
                continue;
            } else if !errors.is_empty() {
                children.push(TreeChild::ErrorLeaf(errors));
                errors = vec![];
            }

            children.push(child.to_tree_child(text));
        }

        if !errors.is_empty() {
            children.push(TreeChild::ErrorLeaf(errors));
        }

        TreeFrame {
            description: self.description,
            children,
            error: self.error.clone(),
        }
    }

    fn add_expr(&mut self, expr: Expression) {
        self.children.push(FrameChild::Expr(expr))
    }

    fn add_result(&mut self, result: impl PrettyDebug) {
        self.children.push(FrameChild::Result(result.to_doc()))
    }
}

#[derive(Debug, Clone)]
pub struct TreeFrame {
    description: &'static str,
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

            write!(f, " -> ")?;
            self.children[0].leaf_description(f)
        } else if self.error.is_some() {
            if self.children.is_empty() {
                write!(
                    f,
                    "{}",
                    Color::White.bold().on(Color::Red).paint(self.description)
                )
            } else {
                write!(f, "{}", Color::Red.normal().paint(self.description))
            }
        } else if self.has_descendent_green() {
            write!(f, "{}", Color::Green.normal().paint(self.description))
        } else {
            write!(f, "{}", Color::Yellow.bold().paint(self.description))
        }
    }

    fn has_child_green(&self) -> bool {
        self.children.iter().any(|item| match item {
            TreeChild::OkFrame(..) | TreeChild::ErrorFrame(..) | TreeChild::ErrorLeaf(..) => false,
            TreeChild::OkExpr(..) | TreeChild::OkNonExpr(..) => true,
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
                TreeChild::OkExpr(..) | TreeChild::OkNonExpr(..) | TreeChild::ErrorLeaf(..) => {
                    vec![]
                }
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
    OkExpr(Expression, Text),
    OkFrame(TreeFrame, Text),
    ErrorFrame(TreeFrame, Text),
    ErrorLeaf(Vec<&'static str>),
}

impl TreeChild {
    fn leaf_description(&self, f: &mut impl io::Write) -> io::Result<()> {
        match self {
            TreeChild::OkExpr(expr, text) => write!(
                f,
                "{} {} {}",
                Color::Cyan.normal().paint("returns"),
                Color::White.bold().on(Color::Green).paint(expr.type_name()),
                expr.span.slice(text)
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

            TreeChild::ErrorLeaf(desc) => {
                let last = desc.len() - 1;

                for (i, item) in desc.iter().enumerate() {
                    write!(f, "{}", Color::White.bold().on(Color::Red).paint(*item))?;

                    if i != last {
                        write!(f, "{}", Color::White.normal().paint(", "))?;
                    }
                }

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
            TreeChild::OkExpr(..) | TreeChild::OkNonExpr(..) | TreeChild::ErrorLeaf(..) => {
                Cow::Borrowed(&[])
            }
            TreeChild::OkFrame(frame, text) | TreeChild::ErrorFrame(frame, text) => {
                Cow::Owned(frame.children_for_formatting(text))
            }
        }
    }
}

#[derive(Debug)]
pub struct ExpandTracer {
    frame_stack: Vec<ExprFrame>,
    source: Text,
}

impl ExpandTracer {
    pub fn print(&self, source: Text) -> PrintTracer {
        let root = self.frame_stack[0].to_tree_frame(&source);

        PrintTracer { root, source }
    }

    pub fn new(source: Text) -> ExpandTracer {
        let root = ExprFrame {
            description: "Trace",
            children: vec![],
            error: None,
        };

        ExpandTracer {
            frame_stack: vec![root],
            source,
        }
    }

    fn current_frame(&mut self) -> &mut ExprFrame {
        let frames = &mut self.frame_stack;
        let last = frames.len() - 1;
        &mut frames[last]
    }

    fn pop_frame(&mut self) -> ExprFrame {
        let result = self.frame_stack.pop().expect("Can't pop root tracer frame");

        if self.frame_stack.is_empty() {
            panic!("Can't pop root tracer frame");
        }

        self.debug();

        result
    }

    pub fn start(&mut self, description: &'static str) {
        let frame = ExprFrame {
            description,
            children: vec![],
            error: None,
        };

        self.frame_stack.push(frame);
        self.debug();
    }

    pub fn add_expr(&mut self, shape: Expression) {
        self.current_frame().add_expr(shape);
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
            .push(FrameChild::Frame(current));
    }

    pub fn failed(&mut self, error: &ParseError) {
        let mut current = self.pop_frame();
        current.error = Some(error.clone());
        self.current_frame()
            .children
            .push(FrameChild::Frame(current));
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
    root: TreeFrame,
    source: Text,
}

impl TreeItem for PrintTracer {
    type Child = TreeChild;

    fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
        write!(f, "{}", style.paint("Expansion Trace"))
    }

    fn children(&self) -> Cow<[Self::Child]> {
        Cow::Borrowed(&self.root.children)
    }
}
