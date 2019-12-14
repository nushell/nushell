use crate::hir::syntax_shape::flat_shape::{FlatShape, ShapeResult};
use nu_source::{Span, Spanned, SpannedItem};

pub struct FlatShapes {
    shapes: Vec<ShapeResult>,
}

impl<'a> IntoIterator for &'a FlatShapes {
    type Item = &'a ShapeResult;
    type IntoIter = std::slice::Iter<'a, ShapeResult>;

    fn into_iter(self) -> Self::IntoIter {
        self.shapes.iter()
    }
}

pub trait IntoShapes: 'static {
    fn into_shapes(self, span: Span) -> FlatShapes;
}

impl IntoShapes for FlatShape {
    fn into_shapes(self, span: Span) -> FlatShapes {
        FlatShapes {
            shapes: vec![ShapeResult::Success(self.spanned(span))],
        }
    }
}

impl IntoShapes for Vec<Spanned<FlatShape>> {
    fn into_shapes(self, _span: Span) -> FlatShapes {
        FlatShapes {
            shapes: self
                .into_iter()
                .map(|shape| ShapeResult::Success(shape))
                .collect(),
        }
    }
}

impl IntoShapes for Vec<ShapeResult> {
    fn into_shapes(self, _span: Span) -> FlatShapes {
        FlatShapes { shapes: self }
    }
}

impl IntoShapes for () {
    fn into_shapes(self, _span: Span) -> FlatShapes {
        FlatShapes { shapes: vec![] }
    }
}

impl IntoShapes for Option<FlatShape> {
    fn into_shapes(self, span: Span) -> FlatShapes {
        match self {
            Option::None => ().into_shapes(span),
            Option::Some(shape) => shape.into_shapes(span),
        }
    }
}
