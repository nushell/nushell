use crate::CommandRegistry;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;
use nu_protocol::{
    hir::{
        Binary, Block, ClassifiedCommand, Commands, Expression, Literal, NamedArguments,
        NamedValue, Operator, SpannedExpression, Variable,
    },
    NamedType, PositionalType, Signature, SyntaxShape,
};
use nu_source::Span;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash};

use itertools::{merge_join_by, EitherOrBoth, Itertools};
use log::trace;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDeclaration {
    pub name: String,
    // type_decl: Option<UntaggedValue>,
    // scope: ?
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Deduction {
    VarShapeDeduction(Vec<VarShapeDeduction>),
    //A deduction for VarArgs will have a different layout than for a normal var
    //Therefore Deduction is implemented as a enum
    // VarArgShapeDeduction(VarArgShapeDeduction),
}

// That would be one possible layout for a var arg shape deduction
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct VarArgShapeDeduction {
//     /// Spans pointing to the source of the deduction.
//     /// The spans locate positions within the tag of var_decl
//     pub deduced_from: Vec<Span>,
//     pub pos_shapes: Vec<(PositionalType, String)>,
//     pub rest_shape: Option<(SyntaxShape, String)>,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarShapeDeduction {
    pub deduction: SyntaxShape,
    /// Spans pointing to the source of the deduction.
    /// The spans locate positions within the tag of var_decl
    pub deducted_from: Vec<Span>,
}

impl VarShapeDeduction {
    pub fn from_usage(usage: &Span, deduced_shape: &SyntaxShape) -> VarShapeDeduction {
        VarShapeDeduction {
            deduction: *deduced_shape,
            deducted_from: vec![*usage],
        }
    }

    pub fn from_usage_with_alternatives(
        usage: &Span,
        alternatives: &[SyntaxShape],
    ) -> Vec<VarShapeDeduction> {
        alternatives
            .iter()
            .map(|shape| VarShapeDeduction::from_usage(usage, shape))
            .collect()
    }
}

struct FakeVarGen {
    counter: usize,
    fake_var_prefix: String,
}
impl FakeVarGen {
    pub fn new() -> Self {
        FakeVarGen {
            counter: 0,
            fake_var_prefix: "$DFSasfjqiDFJSnSbwbqWF".to_string(),
        }
    }
    pub fn next(&mut self) -> String {
        let mut fake_var = self.fake_var_prefix.clone();
        fake_var.push_str(&self.counter.to_string());
        self.counter += 1;
        fake_var
    }

    pub fn next_as_expr(&mut self) -> (VarUsage, Expression) {
        let var = self.next();
        (
            VarUsage::new(&var, &Span::unknown()),
            Expression::Variable(Variable::Other(var, Span::unknown())),
        )
    }
}

pub struct VarSyntaxShapeDeductor {
    //Initial set of caller provided var declarations
    var_declarations: Vec<VarDeclaration>,
    //Inferences for variables
    inferences: HashMap<VarUsage, Deduction>,
    //Var depending on another var via a operator
    //First is a variable
    //Second is a operator
    //Third is a variable
    dependencies: Vec<(VarUsage, (Operator, Span), VarUsage)>,
    //A var depending on the result type of a spanned_expr
    //First argument is var,
    //Second is binary containing var op and result_bin_expr
    //Third is binary expr, which result shape var depends on
    //This list is populated for binaries like: $var + $baz * $bar
    dependencies_on_result_type: Vec<(VarUsage, Operator, SpannedExpression)>,

    fake_var_generator: FakeVarGen,
}

#[derive(Clone, Debug, Eq)]
pub struct VarUsage {
    pub name: String,
    /// Span describing where this var is used
    pub span: Span,
    //See below
    //pub scope: ?
}
impl VarUsage {
    pub fn new(name: &str, span: &Span) -> VarUsage {
        VarUsage {
            name: name.to_string(),
            span: *span,
        }
    }
}

impl PartialEq<VarUsage> for VarUsage {
    // When searching through the expressions, only the name of the
    // Variable is available. (TODO And their scope). Their full definition is not available.
    // Therefore the equals relationship is relaxed
    fn eq(&self, other: &VarUsage) -> bool {
        // TODO when scripting is available scope has to be respected
        self.name == other.name
        // && self.scope == other.scope
    }
}

impl Hash for VarUsage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl From<VarDeclaration> for VarUsage {
    fn from(decl: VarDeclaration) -> Self {
        //Span unknown as var can be used in multiple places including none
        VarUsage::new(&decl.name, &Span::unknown())
    }
}
impl From<&VarDeclaration> for VarUsage {
    fn from(decl: &VarDeclaration) -> Self {
        //Span unknown as var can be used in multiple places including none
        VarUsage::new(&decl.name, &Span::unknown())
    }
}

//REVIEW these 4 functions if correct types are returned
fn get_shapes_allowed_in_table_header() -> Vec<SyntaxShape> {
    vec![SyntaxShape::String]
}

fn get_shapes_allowed_in_path() -> Vec<SyntaxShape> {
    vec![SyntaxShape::Int, SyntaxShape::String]
}

fn get_shapes_decay_able_to_bool() -> Vec<SyntaxShape> {
    vec![SyntaxShape::Int]
}

fn get_shapes_allowed_in_range() -> Vec<SyntaxShape> {
    vec![SyntaxShape::Int]
}

fn op_of(bin: &SpannedExpression) -> Operator {
    match &bin.expr {
        Expression::Binary(bin) => match bin.op.expr {
            Expression::Literal(Literal::Operator(oper)) => oper,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
fn change_op_to_assignment(mut bin: SpannedExpression) -> SpannedExpression {
    match &mut bin.expr {
        Expression::Binary(bin) => {
            match &mut bin.op.expr {
                Expression::Literal(Literal::Operator(op)) => {
                    //Currently there is no assignment operator.
                    //Plus does the same thing
                    *op = Operator::Equal;
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
    bin
}

//TODO in the future there should be a unit interface
//which offers this functionality; SyntaxShape::Unit would then be
//SyntaxShape::Unit(UnitType)
/// Get the resulting type if op is applied to l_shape and r_shape
/// Throws error if types are not coerceable
///
fn get_result_shape_of(
    l_shape: SyntaxShape,
    op_expr: &SpannedExpression,
    r_shape: SyntaxShape,
) -> Result<SyntaxShape, ShellError> {
    let op = match op_expr.expr {
        Expression::Literal(Literal::Operator(op)) => op,
        _ => unreachable!("Passing anything but the op expr is invalid"),
    };
    //TODO one should check that the types are coerceable.
    //There is some code for that in the evaluator already.
    //One might reuse it.
    //For now we ignore this issue
    Ok(match op {
        Operator::Equal
        | Operator::NotEqual
        | Operator::LessThan
        | Operator::GreaterThan
        | Operator::In
        | Operator::NotIn
        | Operator::And
        | Operator::Or
        | Operator::LessThanOrEqual
        | Operator::GreaterThanOrEqual
        | Operator::Contains
        | Operator::NotContains => {
            //TODO introduce syntaxshape boolean
            SyntaxShape::Int
        }
        Operator::Plus | Operator::Minus => {
            //l_type +/- r_type gives l_type again (if no weird coercion)
            l_shape
        }
        Operator::Multiply => {
            if l_shape == SyntaxShape::Unit || r_shape == SyntaxShape::Unit {
                SyntaxShape::Unit
            } else {
                SyntaxShape::Number
            }
        }
        Operator::Divide => {
            if l_shape == r_shape {
                SyntaxShape::Number
            } else if l_shape == SyntaxShape::Unit {
                l_shape
            } else {
                SyntaxShape::Number
            }
        }
        Operator::Modulo => SyntaxShape::Number,
    })
}

enum BinarySide {
    Left,
    Right,
}

impl VarSyntaxShapeDeductor {
    /// Deduce vars_to_find in block.
    /// Returns: Mapping from var_to_find -> Vec<shape_deduction>
    /// in which each shape_deduction is one possible deduction for the variable.
    /// If a variable is used in at least 2 places with different
    /// required shapes, that do not coerce into each other,
    /// an error is returned.
    /// If Option<Deduction> is None, no deduction can be made (for example if
    /// the variable is not present in the block).
    pub fn infer_vars(
        vars_to_find: &[VarDeclaration],
        block: &Block,
        registry: &CommandRegistry,
    ) -> Result<Vec<(VarDeclaration, Option<Deduction>)>, ShellError> {
        trace!("Deducing shapes for vars: {:?}", vars_to_find);

        let mut deducer = VarSyntaxShapeDeductor {
            var_declarations: vars_to_find.to_owned(),
            inferences: HashMap::new(),
            // block,
            dependencies: Vec::new(),
            dependencies_on_result_type: Vec::new(),
            fake_var_generator: FakeVarGen::new(),
        };
        deducer.infer_shape(block, registry)?;

        deducer.solve_dependencies();
        trace!("Found shapes for vars: {:?}", deducer.inferences);

        Ok(deducer
            .var_declarations
            .iter()
            .map(|decl| {
                let usage: VarUsage = decl.into();
                let deduction = match deducer.inferences.get(&usage) {
                    Some(vec) => Some(vec.clone()),
                    None => None,
                };
                (decl.clone(), deduction)
            })
            .collect())
    }

    fn infer_shape(&mut self, block: &Block, registry: &CommandRegistry) -> Result<(), ShellError> {
        trace!("Infering vars in shape");
        for pipeline in &block.block {
            self.infer_pipeline(pipeline, registry)?;
        }
        Ok(())
    }

    pub fn infer_pipeline(
        &mut self,
        pipeline: &Commands,
        registry: &CommandRegistry,
    ) -> Result<(), ShellError> {
        trace!("Infering vars in pipeline");
        for (cmd_pipeline_idx, classified) in pipeline.list.iter().enumerate() {
            match &classified {
                ClassifiedCommand::Internal(internal) => {
                    if let Some(signature) = registry.get(&internal.name) {
                        //When the signature is given vars directly used as named or positional
                        //arguments can be deduced
                        //e.G. cp $var1 $var2
                        if let Some(positional) = &internal.args.positional {
                            //Infer shapes in positional
                            self.infer_shapes_based_on_signature_positional(
                                positional, &signature,
                            )?;
                        }
                        if let Some(named) = &internal.args.named {
                            //Infer shapes in named
                            self.infer_shapes_based_on_signature_named(named, &signature)?;
                        }
                    }
                    //vars in expressions can be deduced by their usage
                    //e.G. 1..$var ($var is of type Int)
                    if let Some(positional) = &internal.args.positional {
                        //Infer shapes in positional
                        for (_pos_idx, pos_expr) in positional.iter().enumerate() {
                            self.infer_shapes_in_expr(
                                (cmd_pipeline_idx, pipeline),
                                pos_expr,
                                registry,
                            )?;
                        }
                    }
                    if let Some(named) = &internal.args.named {
                        trace!("Infering vars in named exprs");
                        for (_name, val) in named.iter() {
                            if let NamedValue::Value(_, named_expr) = val {
                                self.infer_shapes_in_expr(
                                    (cmd_pipeline_idx, pipeline),
                                    named_expr,
                                    registry,
                                )?;
                            }
                        }
                    }
                }
                ClassifiedCommand::Expr(spanned_expr) => {
                    trace!(
                        "Infering shapes in ClassifiedCommand::Expr: {:?}",
                        spanned_expr
                    );
                    self.infer_shapes_in_expr(
                        (cmd_pipeline_idx, pipeline),
                        spanned_expr,
                        registry,
                    )?;
                }
                ClassifiedCommand::Dynamic(_) | ClassifiedCommand::Error(_) => unimplemented!(),
            }
        }
        Ok(())
    }

    fn infer_shapes_based_on_signature_positional(
        &mut self,
        positionals: &[SpannedExpression],
        signature: &Signature,
    ) -> Result<(), ShellError> {
        trace!("Infering vars in positionals");
        //TODO currently correct inference for optional positionals is not implemented.
        // See https://github.com/nushell/nushell/pull/2486 for a discussion about this
        // For now we assume every variable in an optional positional is used as this optional
        // argument
        trace!("Positionals len: {:?}", positionals.len());
        for (pos_idx, positional) in positionals.iter().enumerate().rev() {
            trace!("Handling pos_idx: {:?} of type: {:?}", pos_idx, positional);
            if let Expression::Variable(Variable::Other(var_name, _)) = &positional.expr {
                let deduced_shape = {
                    if pos_idx >= signature.positional.len() {
                        if let Some((shape, _)) = &signature.rest_positional {
                            Some(shape)
                        } else {
                            None
                        }
                    } else {
                        match &signature.positional[pos_idx].0 {
                            PositionalType::Mandatory(_, shape)
                            | PositionalType::Optional(_, shape) => Some(shape),
                        }
                    }
                };
                trace!(
                    "Found var: {:?} in positional_idx: {:?} of shape: {:?}",
                    var_name,
                    pos_idx,
                    deduced_shape
                );
                if let Some(shape) = deduced_shape {
                    self.checked_insert(
                        &VarUsage::new(var_name, &positional.span),
                        vec![VarShapeDeduction::from_usage(&positional.span, shape)],
                    )?;
                }
            }
        }
        Ok(())
    }

    fn infer_shapes_based_on_signature_named(
        &mut self,
        named: &NamedArguments,
        signature: &Signature,
    ) -> Result<(), ShellError> {
        trace!("Infering vars in named");
        for (name, val) in named.iter() {
            if let NamedValue::Value(span, spanned_expr) = &val {
                if let Expression::Variable(Variable::Other(var_name, _)) = &spanned_expr.expr {
                    if let Some((named_type, _)) = signature.named.get(name) {
                        if let NamedType::Mandatory(_, shape) | NamedType::Optional(_, shape) =
                            named_type
                        {
                            trace!(
                                "Found var: {:?} in named: {:?} of shape: {:?}",
                                var_name,
                                name,
                                shape
                            );
                            self.checked_insert(
                                &VarUsage::new(var_name, span),
                                vec![VarShapeDeduction::from_usage(span, shape)],
                            )?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn infer_shapes_in_expr(
        &mut self,
        (pipeline_idx, pipeline): (usize, &Commands),
        spanned_expr: &SpannedExpression,
        registry: &CommandRegistry,
    ) -> Result<(), ShellError> {
        match &spanned_expr.expr {
            Expression::Binary(_) => {
                trace!("Infering vars in bin expr");
                self.infer_shapes_in_binary_expr((pipeline_idx, pipeline), spanned_expr, registry)?;
            }
            Expression::Block(b) => {
                trace!("Infering vars in block");
                self.infer_shape(&b, registry)?;
            }
            Expression::Path(path) => {
                trace!("Infering vars in path");
                match &path.head.expr {
                    //PathMember can't be var yet (?)
                    //TODO Iterate over path parts and find var when implemented
                    Expression::Invocation(b) => self.infer_shape(&b, registry)?,
                    Expression::Variable(Variable::Other(var_name, span)) => {
                        self.checked_insert(
                            &VarUsage::new(var_name, span),
                            VarShapeDeduction::from_usage_with_alternatives(
                                &span,
                                &get_shapes_allowed_in_path(),
                            ),
                        )?;
                    }
                    _ => (),
                }
            }
            Expression::Range(range) => {
                trace!("Infering vars in range");
                if let Some(range_left) = &range.left {
                    if let Expression::Variable(Variable::Other(var_name, _)) = &range_left.expr {
                        self.checked_insert(
                            &VarUsage::new(var_name, &spanned_expr.span),
                            VarShapeDeduction::from_usage_with_alternatives(
                                &spanned_expr.span,
                                &get_shapes_allowed_in_range(),
                            ),
                        )?;
                    }
                }
                if let Some(range_right) = &range.right {
                    if let Expression::Variable(Variable::Other(var_name, span)) = &range_right.expr
                    {
                        self.checked_insert(
                            &VarUsage::new(&var_name, &spanned_expr.span),
                            VarShapeDeduction::from_usage_with_alternatives(
                                &span,
                                &get_shapes_allowed_in_range(),
                            ),
                        )?;
                    }
                }
            }
            Expression::List(inner_exprs) => {
                trace!("Infering vars in list");
                for expr in inner_exprs {
                    self.infer_shapes_in_expr((pipeline_idx, pipeline), expr, registry)?;
                }
            }
            Expression::Invocation(invoc) => {
                trace!("Infering vars in invocation: {:?}", invoc);
                self.infer_shape(invoc, registry)?;
            }
            Expression::Table(header, _rows) => {
                self.infer_shapes_in_table_header(header)?;
                // Shapes within columns can be heterogenous as long as
                // https://github.com/nushell/rfcs/pull/3
                // didn't land
                // self.infer_shapes_in_rows(_rows)?;
            }
            Expression::Variable(_) => {}
            Expression::Literal(_) => {}
            Expression::ExternalWord => {}
            Expression::Synthetic(_) => {}
            Expression::FilePath(_) => {}
            Expression::ExternalCommand(_) => {}
            Expression::Command => {}
            Expression::Boolean(_) => {}
            Expression::Garbage => {}
        };

        Ok(())
    }

    fn infer_shapes_in_table_header(
        &mut self,
        header: &[SpannedExpression],
    ) -> Result<(), ShellError> {
        for expr in header {
            if let Expression::Variable(Variable::Other(name, _)) = &expr.expr {
                let var = VarUsage::new(name, &expr.span);
                self.checked_insert(
                    &var,
                    VarShapeDeduction::from_usage_with_alternatives(
                        &var.span,
                        &get_shapes_allowed_in_table_header(),
                    ),
                )?;
            }
        }
        Ok(())
    }

    fn get_shape_of_expr_or_insert_dependency(
        &mut self,
        var: &VarUsage,
        (op, span): (Operator, Span),
        expr: &SpannedExpression,
    ) -> Option<SyntaxShape> {
        match &expr.expr {
            Expression::Variable(Variable::Other(name, _)) => {
                self.dependencies
                    .push((var.clone(), (op, span), VarUsage::new(name, &expr.span)));
                None
            }
            Expression::Variable(Variable::It(_)) => {
                //TODO infer type of $it
                //therefore pipeline idx, pipeline and registry has to be passed here
                None
            }
            Expression::Literal(literal) => {
                match literal {
                    nu_protocol::hir::Literal::Number(number) => match number {
                        nu_protocol::hir::Number::Int(_) => Some(SyntaxShape::Int),
                        nu_protocol::hir::Number::Decimal(_) => Some(SyntaxShape::Number),
                    },
                    nu_protocol::hir::Literal::Size(_, _) => Some(SyntaxShape::Unit),
                    nu_protocol::hir::Literal::String(_) => Some(SyntaxShape::String),
                    //Rest should have failed at parsing stage?
                    nu_protocol::hir::Literal::GlobPattern(_) => Some(SyntaxShape::String),
                    nu_protocol::hir::Literal::Operator(_) => Some(SyntaxShape::Operator),
                    nu_protocol::hir::Literal::ColumnPath(_) => Some(SyntaxShape::ColumnPath),
                    nu_protocol::hir::Literal::Bare(_) => Some(SyntaxShape::String),
                }
            }
            //Synthetic are expressions that are generated by the parser and not inputed by the user
            //ExternalWord is anything sent to external commands (?)
            Expression::ExternalWord => Some(SyntaxShape::String),
            Expression::Synthetic(_) => Some(SyntaxShape::String),

            Expression::Binary(_) => Some(SyntaxShape::Math),
            Expression::Range(_) => Some(SyntaxShape::Range),
            Expression::List(_) => Some(SyntaxShape::Table),
            Expression::Boolean(_) => Some(SyntaxShape::String),

            Expression::Path(_) => Some(SyntaxShape::ColumnPath),
            Expression::FilePath(_) => Some(SyntaxShape::Path),
            Expression::Block(_) => Some(SyntaxShape::Block),
            Expression::ExternalCommand(_) => Some(SyntaxShape::String),
            Expression::Table(_, _) => Some(SyntaxShape::Table),
            Expression::Command => Some(SyntaxShape::String),
            Expression::Invocation(_) => Some(SyntaxShape::Block),
            Expression::Garbage => unreachable!("Should have failed at parsing stage"),
        }
    }

    fn get_shape_of_binary_arg_or_insert_dependency(
        &mut self,
        //var depending on shape of expr (arg)
        (var, expr): (&VarUsage, &SpannedExpression),
        //source_bin is binary having var on one and expr on other side
        source_bin: &SpannedExpression,
        (pipeline_idx, pipeline): (usize, &Commands),
        registry: &CommandRegistry,
    ) -> Result<Option<SyntaxShape>, ShellError> {
        trace!("Getting shape of binary arg {:?} for var {:?}", expr, var);
        if let Some(shape) = self.get_shape_of_expr_or_insert_dependency(
            var,
            (op_of(source_bin), source_bin.span),
            expr,
        ) {
            trace!("> Found shape: {:?}", shape);
            Ok(match shape {
                SyntaxShape::Math => {
                    //If execution happens here, the situation is as follows:
                    //There is an Binary expression (source_bin) with a variable on one side
                    //and a binary (lets call it "deep binary") on the other:
                    //e.G. $var + 1 * 1
                    //Now we try to infer the shapes inside the deep binary, compute the resulting
                    //shape based on the operator (see get_result_shape_of) and return that.
                    //That won't work if one of the deeper binary left/right expr is a variable.
                    //Then we insert an element into
                    //VarSyntaxShapeDeductor.dependencies_on_result_type
                    //
                    //If the deeper binary contains a binary on one side, we check if that binary
                    //has a computable result type (e.G. has no variable in it) by recursively
                    //calling this function and if so return it.
                    //If the result type is not computable (the deep deep binary had a variable), we substitute
                    //the deep deep binary side of the deep binary with a variable (fake_var) and
                    //insert a dependency from the fake_var to the deep deep binary in
                    //VarSyntaxShapeDeductor.dependencies_on_result_type
                    //The $var on the source_bin will then (as described above) depend on the deep
                    //binary (as it has a variable (fake_var)) on one side.
                    //
                    //The dependencies gets resolved at the end, when most information about all
                    //variables is accessable.
                    //
                    //
                    //
                    //
                    //Expression is of type binary
                    //We have to descend deeper into tree
                    //And compute result shape based on operator
                    let bin = match &expr.expr {
                        Expression::Binary(bin) => bin,
                        _ => unreachable!("SyntaxShape::Math means expression binary"),
                    };
                    match (&bin.left.expr, &bin.right.expr) {
                        //$it should return a shape in get_shape_of_expr_or_insert_dependency below
                        //no need to check it here.
                        (
                            Expression::Variable(Variable::Other(_, _)),
                            Expression::Variable(Variable::Other(_, _)),
                        )
                        | (Expression::Variable(Variable::Other(_, _)), _)
                        | (_, Expression::Variable(Variable::Other(_, _))) => {
                            //Example of this case is: $foo + $bar * $baz
                            //foo = var (depending of shape of arg (bar * baz))
                            self.dependencies_on_result_type.push((
                                var.clone(),
                                op_of(source_bin),
                                expr.clone(),
                            ));
                            None
                        }
                        (Expression::Binary(_), Expression::Binary(_)) => {
                            let (l_fake_var, l_fake_var_expr) =
                                self.fake_var_generator.next_as_expr();
                            let (r_fake_var, r_fake_var_expr) =
                                self.fake_var_generator.next_as_expr();
                            let fake_bin = change_op_to_assignment(expr.clone());
                            match (
                                self.get_shape_of_binary_arg_or_insert_dependency(
                                    (&l_fake_var, &bin.left),
                                    &fake_bin,
                                    (pipeline_idx, pipeline),
                                    registry,
                                )?,
                                self.get_shape_of_binary_arg_or_insert_dependency(
                                    (&r_fake_var, &bin.right),
                                    &fake_bin,
                                    (pipeline_idx, pipeline),
                                    registry,
                                )?,
                            ) {
                                (Some(l_shape), Some(r_shape)) => {
                                    //Both sides could be evaluated
                                    Some(get_result_shape_of(l_shape, &bin.op, r_shape)?)
                                }
                                (None, Some(_)) => {
                                    self.substitute_left_with_fake_var_and_insert_dependencies(
                                        bin,
                                        &l_fake_var_expr,
                                        (source_bin, var),
                                    );
                                    None
                                }
                                (Some(_), None) => {
                                    self.substitute_right_with_fake_var_and_insert_dependencies(
                                        bin,
                                        &r_fake_var_expr,
                                        (source_bin, var),
                                    );
                                    None
                                }
                                (None, None) => {
                                    //Substitute both with fake var and insert dependencies
                                    let mut fake_bin = bin.clone();
                                    fake_bin.right.expr = r_fake_var_expr;
                                    fake_bin.left.expr = l_fake_var_expr;
                                    let op = op_of(source_bin);
                                    self.dependencies_on_result_type.push((
                                        var.clone(),
                                        op,
                                        SpannedExpression::new(
                                            Expression::Binary(fake_bin),
                                            source_bin.span,
                                        ),
                                    ));
                                    None
                                }
                            }
                        }
                        //After here every invocation on get_shape_of_expr_or_insert_dependency(expr) should
                        //give a result shape
                        (Expression::Binary(_), _) => {
                            let (l_fake_var, l_fake_var_expr) =
                                self.fake_var_generator.next_as_expr();
                            let (r_fake_var, _) = self.fake_var_generator.next_as_expr();
                            let fake_bin = change_op_to_assignment(expr.clone());
                            match (
                                self.get_shape_of_binary_arg_or_insert_dependency(
                                    (&l_fake_var, &bin.left),
                                    &fake_bin,
                                    (pipeline_idx, pipeline),
                                    registry,
                                )?,
                                self.get_shape_of_expr_or_insert_dependency(
                                    &r_fake_var,
                                    (Operator::Equal, source_bin.span),
                                    &bin.right,
                                ),
                            ) {
                                (Some(l_shape), Some(r_shape)) => {
                                    Some(get_result_shape_of(l_shape, &bin.op, r_shape)?)
                                }
                                (None, _) => {
                                    self.substitute_left_with_fake_var_and_insert_dependencies(
                                        bin,
                                        &l_fake_var_expr,
                                        (source_bin, var),
                                    );
                                    None
                                }
                                (Some(_), None) => {
                                    unreachable!("At this point shape should be deducable!")
                                }
                            }
                        }
                        (_, Expression::Binary(_)) => {
                            let (l_fake_var, _) = self.fake_var_generator.next_as_expr();
                            let (r_fake_var, r_fake_var_expr) =
                                self.fake_var_generator.next_as_expr();
                            let fake_bin = change_op_to_assignment(expr.clone());
                            match (
                                self.get_shape_of_expr_or_insert_dependency(
                                    &l_fake_var,
                                    (Operator::Equal, source_bin.span),
                                    &bin.left,
                                ),
                                self.get_shape_of_binary_arg_or_insert_dependency(
                                    (&r_fake_var, &bin.right),
                                    &fake_bin,
                                    (pipeline_idx, pipeline),
                                    registry,
                                )?,
                            ) {
                                (Some(l_shape), Some(r_shape)) => {
                                    Some(get_result_shape_of(l_shape, &bin.op, r_shape)?)
                                }
                                (_, None) => {
                                    self.substitute_right_with_fake_var_and_insert_dependencies(
                                        bin,
                                        &r_fake_var_expr,
                                        (source_bin, var),
                                    );
                                    None
                                }
                                (None, Some(_)) => {
                                    unreachable!("At this point shape should be deducable!")
                                }
                            }
                        }
                        (_, _) => {
                            let (l_fake_var, _) = self.fake_var_generator.next_as_expr();
                            let (r_fake_var, _) = self.fake_var_generator.next_as_expr();
                            match (
                                self.get_shape_of_expr_or_insert_dependency(&l_fake_var,
                                    (Operator::Equal, source_bin.span),
                                    &bin.left),
                                self.get_shape_of_expr_or_insert_dependency(&r_fake_var,
                                    (Operator::Equal, source_bin.span),
                                    &bin.right)
                            ) {
                                ( Some(l_shape), Some(r_shape) ) => {
                                    Some(get_result_shape_of(l_shape, &bin.op, r_shape)?)
                                }
                                _ => unreachable!("This should be unreachable as neither expr is real var or binary")

                            }
                        }
                    }
                }
                _ => Some(shape),
            })
        } else {
            trace!("> Could not deduce shape in expr");
            Ok(None)
        }
    }

    fn substitute_right_with_fake_var_and_insert_dependencies(
        &mut self,
        //Bin in which to substitute
        bin: &Binary,
        //The var with which to substitute (as usage and expr)
        r_fake_var_expr: &Expression,
        //The source bin having var on one side and above bin on other
        (source_bin, var): (&SpannedExpression, &VarUsage),
    ) {
        let mut fake_bin = Box::new(bin.clone());
        fake_bin.right.expr = r_fake_var_expr.clone();
        let op = op_of(source_bin);
        self.dependencies_on_result_type.push((
            var.clone(),
            op,
            SpannedExpression::new(Expression::Binary(fake_bin), source_bin.span),
        ));
    }
    fn substitute_left_with_fake_var_and_insert_dependencies(
        &mut self,
        //Bin in which to substitute
        bin: &Binary,
        //The var with which to substitute (as usage and expr)
        l_fake_var_expr: &Expression,
        //The source bin having var on one side and above bin on other
        (source_bin, var): (&SpannedExpression, &VarUsage),
    ) {
        let mut fake_bin = Box::new(bin.clone());
        fake_bin.left.expr = l_fake_var_expr.clone();
        let op = op_of(source_bin);
        self.dependencies_on_result_type.push((
            var.clone(),
            op,
            SpannedExpression::new(Expression::Binary(fake_bin), source_bin.span),
        ));
    }

    fn get_shapes_in_list_or_insert_dependency(
        &mut self,
        var: &VarUsage,
        bin_spanned: &SpannedExpression,
        list: &[SpannedExpression],
        (_pipeline_idx, _pipeline): (usize, &Commands),
        _registry: &CommandRegistry,
    ) -> Option<Vec<SyntaxShape>> {
        let shapes_in_list = list
            .iter()
            .filter_map(|expr| {
                self.get_shape_of_expr_or_insert_dependency(
                    var,
                    (Operator::Equal, bin_spanned.span),
                    expr,
                )
            })
            .collect_vec();
        if shapes_in_list.is_empty() {
            None
        } else {
            Some(shapes_in_list)
        }
    }

    fn infer_shapes_between_var_and_expr(
        &mut self,
        (var, expr): (&VarUsage, &SpannedExpression),
        var_side: BinarySide,
        //Binary having expr on one side and var on other
        bin_spanned: &SpannedExpression,
        (pipeline_idx, pipeline): (usize, &Commands),
        registry: &CommandRegistry,
    ) -> Result<(), ShellError> {
        trace!("Infering shapes between var {:?} and expr {:?}", var, expr);
        let bin = match &bin_spanned.expr {
            Expression::Binary(bin) => bin,
            _ => unreachable!(),
        };
        if let Expression::Literal(Literal::Operator(op)) = bin.op.expr {
            match &op {
                //For || and && we insert shapes decay able to bool
                Operator::And | Operator::Or => {
                    let shapes = get_shapes_decay_able_to_bool();
                    // shapes.push(SyntaxShape::Math);
                    self.checked_insert(
                        &var,
                        VarShapeDeduction::from_usage_with_alternatives(&var.span, &shapes),
                    )?;
                }
                Operator::Contains | Operator::NotContains => {
                    self.checked_insert(
                        var,
                        vec![VarShapeDeduction::from_usage(
                            &var.span,
                            &SyntaxShape::String,
                        )],
                    )?;
                }
                Operator::In | Operator::NotIn => {
                    match var_side {
                        BinarySide::Left => match &expr.expr {
                            Expression::List(list) => {
                                if !list.is_empty() {
                                    let shapes_in_list = self
                                        .get_shapes_in_list_or_insert_dependency(
                                            var,
                                            bin_spanned,
                                            &list,
                                            (pipeline_idx, pipeline),
                                            registry,
                                        );
                                    match shapes_in_list {
                                        None => {}
                                        Some(shapes_in_list) => {
                                            self.checked_insert(
                                                var,
                                                VarShapeDeduction::from_usage_with_alternatives(
                                                    &var.span,
                                                    &shapes_in_list,
                                                ),
                                            )?;
                                        }
                                    }
                                }
                            }
                            //REVIEW is var in table legal? Shouldn't the rhs in "$p in [a b c]" be 
                            //parsed as a list?
                            Expression::Table(_, _) => {}

                            Expression::Literal(_)
                            | Expression::ExternalWord
                            | Expression::Synthetic(_)
                            | Expression::Variable(_)
                            | Expression::Binary(_)
                            | Expression::Range(_)
                            | Expression::Block(_)
                            | Expression::Path(_)
                            | Expression::FilePath(_)
                            | Expression::ExternalCommand(_)
                            | Expression::Command
                            | Expression::Invocation(_)
                            | Expression::Boolean(_)
                            | Expression::Garbage => {unreachable!("Parser should have rejected code. In only applicable with rhs of type List")}
                        },
                        BinarySide::Right => {
                            self.checked_insert(
                                var,
                                VarShapeDeduction::from_usage_with_alternatives(
                                    &var.span,
                                    &[SyntaxShape::Table],
                                ),
                            )?;
                        }
                    }
                }
                Operator::Modulo => {
                    self.checked_insert(
                        var,
                        VarShapeDeduction::from_usage_with_alternatives(
                            &var.span,
                            &[SyntaxShape::Int, SyntaxShape::Number],
                        ),
                    )?;
                }
                Operator::Equal
                | Operator::NotEqual
                | Operator::LessThan
                | Operator::GreaterThan
                | Operator::LessThanOrEqual
                | Operator::GreaterThanOrEqual
                | Operator::Plus
                | Operator::Minus => {
                    if let Some(shape) = self.get_shape_of_binary_arg_or_insert_dependency(
                        (var, expr),
                        bin_spanned,
                        (pipeline_idx, pipeline),
                        registry,
                    )? {
                        match shape {
                            SyntaxShape::Int | SyntaxShape::Number => {
                                self.checked_insert(
                                    var,
                                    VarShapeDeduction::from_usage_with_alternatives(
                                        &var.span,
                                        &[SyntaxShape::Number, SyntaxShape::Int],
                                    ),
                                )?;
                            }
                            SyntaxShape::Unit => {
                                self.checked_insert(
                                    var,
                                    VarShapeDeduction::from_usage_with_alternatives(
                                        &var.span,
                                        &[SyntaxShape::Unit],
                                    ),
                                )?;
                            }
                            s => unreachable!(format!(
                                "Shape of {:?} should have failed at parsing stage",
                                s
                            )),
                        }
                    }
                }
                Operator::Multiply => {
                    if let Some(shape) = self.get_shape_of_binary_arg_or_insert_dependency(
                        (var, expr),
                        bin_spanned,
                        (pipeline_idx, pipeline),
                        registry,
                    )? {
                        if shape == SyntaxShape::Unit {
                            //TODO at the moment unit * unit is not possible
                            //As soon as more complex units land this changes!
                            self.checked_insert(
                                var,
                                vec![VarShapeDeduction::from_usage(
                                    &var.span,
                                    &SyntaxShape::Number,
                                )],
                            )?;
                        } else if shape == SyntaxShape::Number || shape == SyntaxShape::Int {
                            self.checked_insert(
                                var,
                                VarShapeDeduction::from_usage_with_alternatives(
                                    &var.span,
                                    &[SyntaxShape::Number, SyntaxShape::Unit],
                                ),
                            )?;
                        } else {
                            unreachable!("Only int or number or unit in binary with op = * allowed")
                        }
                    }
                }
                Operator::Divide => {
                    if let Some(shape) = self.get_shape_of_binary_arg_or_insert_dependency(
                        (var, expr),
                        bin_spanned,
                        (pipeline_idx, pipeline),
                        registry,
                    )? {
                        match shape {
                            SyntaxShape::Int | SyntaxShape::Number => {
                                match var_side {
                                    BinarySide::Left => {
                                        //Binary is on left, number / int on right
                                        self.checked_insert(
                                            var,
                                            VarShapeDeduction::from_usage_with_alternatives(
                                                &var.span,
                                                &[
                                                    SyntaxShape::Number,
                                                    SyntaxShape::Int,
                                                    SyntaxShape::Unit,
                                                ],
                                            ),
                                        )?;
                                    }
                                    BinarySide::Right => {
                                        //TODO currently no unit type is supports 1/unit. This
                                        //changes if there would be ever more complex unit types
                                        //e.G. Frequency
                                        self.checked_insert(
                                            var,
                                            VarShapeDeduction::from_usage_with_alternatives(
                                                &var.span,
                                                &[SyntaxShape::Number, SyntaxShape::Int],
                                            ),
                                        )?;
                                    }
                                }
                            }
                            SyntaxShape::Unit => {
                                match var_side {
                                    BinarySide::Left => {
                                        //Must be unit / unit
                                        self.checked_insert(
                                            var,
                                            vec![VarShapeDeduction::from_usage(
                                                &var.span,
                                                &SyntaxShape::Unit,
                                            )],
                                        )?;
                                    }
                                    BinarySide::Right => {
                                        //Unit / var
                                        self.checked_insert(
                                            var,
                                            VarShapeDeduction::from_usage_with_alternatives(
                                                &var.span,
                                                &[
                                                    SyntaxShape::Number,
                                                    SyntaxShape::Int,
                                                    SyntaxShape::Unit,
                                                ],
                                            ),
                                        )?;
                                    }
                                }
                            }
                            s => unreachable!(format!(
                                "Shape of {:?} should have failed at parsing stage",
                                s
                            )),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn infer_shapes_in_binary_expr(
        &mut self,
        (pipeline_idx, pipeline): (usize, &Commands),
        bin_spanned: &SpannedExpression,
        registry: &CommandRegistry,
    ) -> Result<(), ShellError> {
        let bin = match &bin_spanned.expr {
            Expression::Binary(bin) => bin,
            _ => unreachable!(),
        };

        if let Expression::Variable(Variable::Other(left_var_name, l_span)) = &bin.left.expr {
            if let Expression::Variable(Variable::Other(right_var_name, r_span)) = &bin.right.expr {
                if left_var_name != right_var_name {
                    //type can't be deduced out of this, so add it to resolve it later
                    self.dependencies.push((
                        VarUsage::new(left_var_name, l_span),
                        (op_of(bin_spanned), bin_spanned.span),
                        VarUsage::new(right_var_name, r_span),
                    ));
                }
                //No further inference possible
                return Ok(());
            }
        }
        if let Expression::Variable(Variable::It(_)) = bin.left.expr {
            if let Expression::Variable(Variable::Other(_right_var_name, _)) = &bin.right.expr {
                todo!("Check return type of source (first command in pipeline), check that only data access (get row etc.) or data manipulation (not manipulating type), are between this cmd and source and if so, set right_var_name shape to return type of source.");
                //No further inference possible
                // return Ok(());
            }
        }
        if let Expression::Variable(Variable::It(_)) = bin.right.expr {
            if let Expression::Variable(Variable::Other(_left_var_name, _)) = &bin.left.expr {
                todo!("Check return type of source (first command in pipeline), check that only data access (get row etc.) or data manipulation (not manipulating type), are between this cmd and source and if so, set right_var_name shape to return type of source.");
                //No further inference possible
                // return Ok(());
            }
        }
        if let Expression::Variable(Variable::Other(left_var_name, l_span)) = &bin.left.expr {
            self.infer_shapes_between_var_and_expr(
                (&VarUsage::new(left_var_name, l_span), &bin.right),
                BinarySide::Left,
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
        }
        if let Expression::Variable(Variable::Other(right_var_name, r_span)) = &bin.right.expr {
            self.infer_shapes_between_var_and_expr(
                (&VarUsage::new(right_var_name, r_span), &bin.left),
                BinarySide::Right,
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
        }
        //Descend deeper into bin tree
        self.infer_shapes_in_expr((pipeline_idx, pipeline), &bin.right, registry)?;
        //Descend deeper into bin tree
        self.infer_shapes_in_expr((pipeline_idx, pipeline), &bin.left, registry)?;

        Ok(())
    }

    fn solve_dependencies(&mut self) {
        // Solves dependencies between variables
        // e.G. $var1 < $var2
        // If $var2 is of type Unit, $var1 has to be the same
        // TODO impl this
        //
        // I would check for global/environment variables.
        // Lookup their types.
        // Then check each node not pointing to others
        // These are free variables - no inference can be made for them
        //
        // Variables having cycles between them (eg. a -> b and b -> a) have to be of the same type
        //
        // Then try to inference the variables depending on the result types again.
    }

    /// Inserts the new deductions. Each VarShapeDeduction represents one alternative for
    /// the variable described by var_usage

    /// Each of the deductions is assumed to be for the same variable
    /// Each of the deductions is assumed to be unique of shape
    fn checked_insert(
        &mut self,
        var_usage: &VarUsage,
        new_deductions: Vec<VarShapeDeduction>,
    ) -> Result<(), ShellError> {
        trace!(
            "Trying to insert for: {:?} possible shapes:{:?}",
            var_usage.name,
            new_deductions
                .iter()
                .map(|d| d.deduction)
                .collect::<Vec<_>>()
        );

        //Every insertion is sorted by shape!
        //Everything within self.inferences is sorted by shape!
        let mut new_deductions = new_deductions;
        new_deductions.sort_unstable_by(|a, b| (a.deduction as i32).cmp(&(b.deduction as i32)));

        let (insert_k, insert_v) = match self.inferences.get_key_value(&var_usage) {
            Some((k, existing_deductions)) => {
                let Deduction::VarShapeDeduction(existing_deductions) = existing_deductions;

                // If there is one any in one deduction, this deduction is capable of representing the other
                // deduction and vice versa
                let (any_in_new, new_vec) = (
                    new_deductions
                        .iter()
                        .any(|deduc| deduc.deduction == SyntaxShape::Any),
                    &new_deductions,
                );
                let (any_in_existing, existing_vec) = (
                    existing_deductions
                        .iter()
                        .any(|deduc| deduc.deduction == SyntaxShape::Any),
                    existing_deductions,
                );

                let combined_deductions =
                    match ((any_in_new, new_vec), (any_in_existing, existing_vec)) {
                        ((true, a), (true, b)) => {
                            //In each alternative there is any
                            //complete merge each set |
                            //TODO move closure into function. But the compiler sheds tears to much for me :F
                            merge_join_by(a, b, |a, b| {
                                (a.deduction as i32).cmp(&(b.deduction as i32))
                            })
                            .map(|either_or| match either_or {
                                EitherOrBoth::Left(deduc) | EitherOrBoth::Right(deduc) => {
                                    deduc.clone()
                                }
                                EitherOrBoth::Both(a_elem, b_elem) => {
                                    let mut combination = a_elem.clone();
                                    combination.deducted_from.extend(&b_elem.deducted_from);
                                    combination
                                }
                            })
                            .collect()
                        }
                        ((false, a), (true, b)) | ((true, b), (false, a)) => {
                            //B has an any. So A can be applied as a whole
                            // So result is intersection(b,a) + a
                            merge_join_by(a, b, |a, b| {
                                (a.deduction as i32).cmp(&(b.deduction as i32))
                            })
                            .map(|either_or| match either_or {
                                //Left is a, right is b
                                //(a + none) + a is a
                                EitherOrBoth::Left(deduc) => Some(deduc.clone()),
                                //(none + b) + a is a
                                EitherOrBoth::Right(_) => None,
                                //(a + b) + a is (a + b)
                                EitherOrBoth::Both(a_elem, b_elem) => {
                                    let mut combination = a_elem.clone();
                                    combination.deducted_from.extend(&b_elem.deducted_from);
                                    Some(combination)
                                }
                            })
                            .filter_map(|elem| elem)
                            .collect()
                        }
                        //No any's intersection of both is result
                        ((false, a), (false, b)) => {
                            let intersection: Vec<VarShapeDeduction> =
                                merge_join_by(a, b, |a, b| {
                                    (a.deduction as i32).cmp(&(b.deduction as i32))
                                })
                                .map(|either_or| match either_or {
                                    //Left is a, right is b
                                    EitherOrBoth::Left(_) => None,
                                    EitherOrBoth::Right(_) => None,
                                    EitherOrBoth::Both(a_elem, b_elem) => {
                                        let mut combination = a_elem.clone();
                                        combination
                                            .deducted_from
                                            .extend(b_elem.deducted_from.clone());
                                        Some(combination)
                                    }
                                })
                                .filter_map(|elem| elem)
                                .collect();
                            if intersection.is_empty() {
                                //TODO pass all labels somehow
                                // let labels = a
                                //     .iter()
                                //     .chain(b.iter())
                                //     .map(|decl| {
                                //         decl.deducted_from.iter().map(|span| (decl.deduction, span))
                                //     })
                                //     .flatten()
                                //     .map(|(shape, span)| {
                                //         Label::primary("AliasBlock", span)
                                //             .with_message(format!("{}", shape))
                                //     })
                                //     .collect();
                                return Err(ShellError::labeled_error_with_secondary(
                                    format!("Contrary types for variable {}", k.name),
                                    format!(
                                        "Deduction: {:?}",
                                        a.iter()
                                            .map(|deduction| deduction.deduction)
                                            .collect::<Vec<_>>()
                                    ),
                                    a[0].deducted_from[0],
                                    format!(
                                        "Deduction: {:?}",
                                        b.iter()
                                            .map(|deduction| deduction.deduction)
                                            .collect::<Vec<_>>()
                                    ),
                                    b[0].deducted_from[0],
                                ));
                            } else {
                                intersection
                            }
                        }
                    };
                (k.clone(), Deduction::VarShapeDeduction(combined_deductions))
            }
            None => (
                var_usage.clone(),
                Deduction::VarShapeDeduction(new_deductions),
            ),
        };

        self.inferences.insert(insert_k, insert_v);
        Ok(())
    }
}
