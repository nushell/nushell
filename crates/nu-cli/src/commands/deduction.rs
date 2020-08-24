use crate::CommandRegistry;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;
use nu_protocol::{
    hir::{
        Block, ClassifiedCommand, Commands, Expression, Literal, NamedArguments, NamedValue,
        Operator, SpannedExpression, Variable,
    },
    NamedType, PositionalType, Signature, SyntaxShape,
};
use nu_source::Span;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use codespan_reporting::diagnostic::Diagnostic;
use itertools::{merge_join_by, EitherOrBoth};

//TODO move all of this to nu_cli/src/types/deduction.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDeclaration {
    pub name: String,
    // type_decl: Option<UntaggedValue>,
    pub is_var_arg: bool,
    // scope: ?
    pub span: Span,
}

impl VarDeclaration {
    // pub fn new(name: &str, span: Span) -> VarDeclaration{
    //     VarDeclaration{
    //         name: name.to_string(),
    //         is_var_arg: false,
    //         span,
    //     }
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarShapeDeduction {
    pub deduction: SyntaxShape,
    /// Spans pointing to the source of the deduction.
    /// The spans locate positions within the tag of var_decl
    pub deducted_from: Vec<Span>,
    /// Whether the variable can be substituted with the SyntaxShape deduction
    /// multiple times.
    /// For example a Var-Arg-Variable must be substituted when used in a cmd with
    /// a signature of:
    /// cmd [optionalPaths...] [integers...]
    /// with 2 SpannedVarShapeDeductions, where each can substitute multiple arguments
    pub many_of_shapes: bool,
}

impl VarShapeDeduction {
    //TODO better naming
    pub fn from_usage(usage: &Span, deduced_shape: &SyntaxShape) -> VarShapeDeduction {
        VarShapeDeduction {
            deduction: *deduced_shape,
            deducted_from: vec![*usage],
            many_of_shapes: false,
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

pub struct VarSyntaxShapeDeductor {
    //Initial set of caller provided var declarations
    var_declarations: Vec<VarDeclaration>,
    inferences: HashMap<VarUsage, Vec<VarShapeDeduction>>,
    //The block which is analysed.
    // block: &'a SpannedExpression,
    //var binary var
    dependencies: Vec<(VarUsage, SpannedExpression, VarUsage)>,
}

//TODO Where to put this
#[derive(Clone, Debug, Eq)]
pub struct VarUsage {
    pub name: String,
    /// Span describing where this var is used
    pub span: Span,
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
        //Span unknown as multiple options are possible
        VarUsage::new(&decl.name, &Span::unknown())
    }
}
impl From<&VarDeclaration> for VarUsage {
    fn from(decl: &VarDeclaration) -> Self {
        //Span unknown as multiple options are possible
        VarUsage::new(&decl.name, &Span::unknown())
    }
}

//TODO Where to put these
fn get_shapes_allowed_in_path() -> Vec<SyntaxShape> {
    vec![SyntaxShape::Int, SyntaxShape::String]
}

fn get_shapes_decay_able_to_bool() -> Vec<SyntaxShape> {
    // todo!("What types are decay able to bool?");
    vec![SyntaxShape::Int, SyntaxShape::Math]
}

fn get_shapes_allowed_in_range() -> Vec<SyntaxShape> {
    vec![SyntaxShape::Int]
}

type AlternativeDeductions = Vec<VarShapeDeduction>;
impl VarSyntaxShapeDeductor {
    /// Deduce vars_to_find in block.
    /// Returns: Mapping from var_to_find -> Vec<shape_deduction>
    /// in which each shape_deduction is one possible deduction for the variable
    /// A mapping.get(var_to_find) == None means that no deduction
    /// has been found for var_to_find
    /// If a variable is used in at least 2 places with different
    /// required shapes, that do not coerce into each other,
    /// an error is returned
    pub fn infer_vars(
        vars_to_find: &[VarDeclaration],
        block: &Block,
        registry: &CommandRegistry,
    ) -> Result<Vec<(VarDeclaration, Option<AlternativeDeductions>)>, ShellError> {
        trace!("Deducing shapes for vars: {:?}", vars_to_find);

        let mut deducer = VarSyntaxShapeDeductor {
            var_declarations: vars_to_find.to_owned(),
            inferences: HashMap::new(),
            // block,
            dependencies: Vec::new(),
        };
        deducer.infer_shape(block, registry)?;

        deducer.solve_dependencies();
        trace!("Found shapes for vars: {:?}", deducer.inferences);

        //Remove unwanted vars
        Ok(deducer
            .var_declarations
            .iter()
            .map(|decl| {
                let usage: VarUsage = decl.into();
                let deductions = match deducer.inferences.get(&usage) {
                    Some(vec) => Some(vec.clone()),
                    None => None,
                };
                (decl.clone(), deductions)
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
                        //Infer shapes in named
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
                ClassifiedCommand::Expr(_spanned_expr) => {
                    trace!(
                        "Infering shapes in ClassifiedCommand::Expr: {:?}",
                        _spanned_expr
                    );
                    self.infer_shapes_in_expr(
                        (cmd_pipeline_idx, pipeline),
                        _spanned_expr,
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
        // todo!("If current pos is optional check that all expr behind cur_pos are shiftable by 1");
        //Currently we assume var is fitting optional parameter
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
                    //PathMember can't be var?
                    //TODO Iterate over path parts and find var
                    //Currently no vars in path allowed???
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
                if let Expression::Variable(Variable::Other(var_name, _)) = &range.left.expr {
                    self.checked_insert(
                        &VarUsage::new(var_name, &spanned_expr.span),
                        VarShapeDeduction::from_usage_with_alternatives(
                            &spanned_expr.span,
                            &get_shapes_allowed_in_range(),
                        ),
                    )?;
                } else if let Expression::Variable(Variable::Other(var_name, span)) =
                    &range.right.expr
                {
                    self.checked_insert(
                        &VarUsage::new(var_name, &spanned_expr.span),
                        VarShapeDeduction::from_usage_with_alternatives(
                            span,
                            &get_shapes_allowed_in_range(),
                        ),
                    )?;
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
            Expression::Table(_header, rows) => {
                //TODO infer shapes in table header? But any can be allowed there?
                self.infer_shapes_in_rows(rows);
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

    fn infer_shapes_in_rows(&mut self, rows: &[Vec<SpannedExpression>]) {
        for (_row_idx, _row) in rows.iter().enumerate() {
            for (_col_idx, _cell) in _row.iter().enumerate() {
                todo!("deduce types in table")
            }
        }
    }

    fn get_shape_of_binary_arg(
        &mut self,
        //var depending on shape of expr (arg)
        (var, expr): (&VarUsage, &SpannedExpression),
        //source_bin is binary having var on one and expr on other side
        source_bin: &SpannedExpression,
        (pipeline_idx, pipeline): (usize, &Commands),
        registry: &CommandRegistry,
    ) -> Option<Vec<SyntaxShape>> {
        match &expr.expr {
            //If both are variable
            Expression::Variable(_) => {
                trace!("Expr is unexpected var: {:?}", expr);
                unreachable!("case that both sides are var is handled on caller side")
            }
            Expression::Literal(literal) => {
                match literal {
                    nu_protocol::hir::Literal::Number(_) => Some(vec![SyntaxShape::Number]),
                    nu_protocol::hir::Literal::Size(_, _) => Some(vec![SyntaxShape::Unit]),
                    nu_protocol::hir::Literal::String(_) => Some(vec![SyntaxShape::String]),
                    //Rest should have failed at parsing stage?
                    nu_protocol::hir::Literal::GlobPattern(_) => Some(vec![SyntaxShape::String]),
                    nu_protocol::hir::Literal::Operator(_) => Some(vec![SyntaxShape::Operator]),
                    nu_protocol::hir::Literal::ColumnPath(_) => Some(vec![SyntaxShape::ColumnPath]),
                    nu_protocol::hir::Literal::Bare(_) => Some(vec![SyntaxShape::String]),
                }
            }
            //What do these both mean?
            Expression::ExternalWord => Some(vec![SyntaxShape::String]),
            Expression::Synthetic(_) => Some(vec![SyntaxShape::String]),

            Expression::Binary(bin) => {
                //if both sides of bin are variables, no deduction will be possible, therefore
                // dependend_var depends on both these vars
                match (&bin.left.expr, &bin.right.expr) {
                    (
                        Expression::Variable(Variable::Other(l_name, l_span)),
                        Expression::Variable(Variable::Other(r_name, r_span)),
                    ) => {
                        //Example of this case is: $foo + $bar * $baz
                        //foo = var (depending of shape of arg (bar * baz))
                        //
                        //TODO depending on the operators between var and this binary
                        // there might or might not be dependencies.
                        // e.G. $var * ($baz < $bar) (As of time of this writing its disallowed)
                        // $var is not dependend on the types of baz or bar.
                        // For e.G. $var * $baz + $bar all types have to coerce into each other
                        //
                        // For now we build all dependencies
                        self.dependencies.push((
                            var.clone(),
                            source_bin.clone(),
                            VarUsage::new(l_name, l_span),
                        ));
                        self.dependencies.push((
                            var.clone(),
                            source_bin.clone(),
                            VarUsage::new(r_name, r_span),
                        ));
                        None
                    }
                    (
                        Expression::Variable(Variable::It(_it_span)),
                        Expression::Variable(Variable::Other(_var_name, _var_span)),
                    )
                    | (
                        Expression::Variable(Variable::Other(_var_name, _var_span)),
                        Expression::Variable(Variable::It(_it_span)),
                    ) => {
                        //TODO deduce type of $it and its usage based on operator and return it
                        None
                    }
                    (
                        Expression::Variable(Variable::It(_l_it)),
                        Expression::Variable(Variable::It(_r_it)),
                    ) => {
                        //TODO deduce type of $it and return it (based on operator)
                        None
                    }
                    _ => {
                        let lhs_shape = self.get_shape_of_binary_arg(
                            (var, &bin.left),
                            source_bin,
                            (pipeline_idx, pipeline),
                            registry,
                        );
                        let rhs_shape = self.get_shape_of_binary_arg(
                            (var, &bin.right),
                            source_bin,
                            (pipeline_idx, pipeline),
                            registry,
                        );
                        match (lhs_shape, rhs_shape) {
                            (None, None) => None,
                            (None, Some(shapes)) | (Some(shapes), None) => Some(shapes),
                            (Some(lhs_shapes), Some(rhs_shapes)) => {
                                let lhs_shapes: HashSet<_> = lhs_shapes.into_iter().collect();
                                let rhs_shapes: HashSet<_> = rhs_shapes.into_iter().collect();
                                //TODO obay coercion
                                let intersection: Vec<SyntaxShape> =
                                    lhs_shapes.intersection(&rhs_shapes).cloned().collect();
                                if intersection.is_empty() {
                                    //TODO throw nice error describing that both sides had
                                    //different types
                                    //e.G. $var * 1kb + $true
                                    unimplemented!("Intersection err");
                                // Err(ShellError::coerce_error)
                                } else {
                                    Some(intersection)
                                }
                            }
                        }
                    }
                }
            }
            Expression::Range(_) => Some(vec![SyntaxShape::Range]),
            Expression::List(_) => todo!("List of inner shapes"),
            Expression::Boolean(_) => todo!("What to return here? Is $var + true valid?"),

            //Rest should have failed at parsing stage? Realy? Also invocation? Old code says so...
            Expression::Path(_)
            | Expression::FilePath(_)
            | Expression::Block(_)
            | Expression::ExternalCommand(_)
            | Expression::Table(_, _)
            | Expression::Command
            | Expression::Invocation(_)
            | Expression::Garbage => unreachable!("Should have failed at parsing stage"),
        }
    }

    fn infer_shapes_between_var_and_expr(
        &mut self,
        (var, expr): (&VarUsage, &SpannedExpression),
        //Binary having expr on one side and var on other
        bin_spanned: &SpannedExpression,
        (pipeline_idx, pipeline): (usize, &Commands),
        registry: &CommandRegistry,
    ) -> Result<(), ShellError> {
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
                Operator::In | Operator::NotIn | Operator::Contains | Operator::NotContains => {
                    todo!("Implement in, notin, contains... for type deduction");
                }
                Operator::Equal
                | Operator::NotEqual
                | Operator::LessThan
                | Operator::GreaterThan
                | Operator::LessThanOrEqual
                | Operator::GreaterThanOrEqual
                | Operator::Plus
                | Operator::Minus
                | Operator::Multiply
                | Operator::Divide => {
                    if let Some(possible_shapes) = self.get_shape_of_binary_arg(
                        (var, expr),
                        bin_spanned,
                        (pipeline_idx, pipeline),
                        registry,
                    ) {
                        // possible_shapes.push(SyntaxShape::Math);
                        self.checked_insert(
                            var,
                            VarShapeDeduction::from_usage_with_alternatives(
                                &var.span,
                                &possible_shapes,
                            ),
                        )?;
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
                        bin_spanned.clone(),
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
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
        }
        if let Expression::Variable(Variable::Other(right_var_name, r_span)) = &bin.right.expr {
            self.infer_shapes_between_var_and_expr(
                (&VarUsage::new(right_var_name, r_span), &bin.left),
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
    }

    /// Inserts the new deductions.
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

        //TODO Check special for var arg
        let (insert_k, insert_v) = match self.inferences.get_key_value(&var_usage) {
            Some((k, existing_deductions)) => {
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

                let combined_deductions = match (
                    (any_in_new, new_vec),
                    (any_in_existing, existing_vec),
                ) {
                    ((true, a), (true, b)) => {
                        //In each alternative there is any
                        //complete merge each set |
                        //TODO move closure into function. But the compiler sheds tears to much for me :F
                        merge_join_by(a, b, |a, b| (a.deduction as i32).cmp(&(b.deduction as i32)))
                            .map(|either_or| match either_or {
                                EitherOrBoth::Left(deduc) | EitherOrBoth::Right(deduc) => {
                                    deduc.clone()
                                }
                                EitherOrBoth::Both(a_elem, b_elem) => {
                                    let mut combination = a_elem.clone();
                                    combination.deducted_from.extend(&b_elem.deducted_from);
                                    combination.many_of_shapes =
                                        combination.many_of_shapes && b_elem.many_of_shapes;
                                    combination
                                }
                            })
                            .collect()
                    }
                    ((false, a), (true, b)) | ((true, b), (false, a)) => {
                        //B has an any. So A can be applied as a whole
                        // So result is intersection(b,a) + a
                        merge_join_by(a, b, |a, b| (a.deduction as i32).cmp(&(b.deduction as i32)))
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
                                    combination.many_of_shapes =
                                        combination.many_of_shapes && b_elem.many_of_shapes;
                                    Some(combination)
                                }
                            })
                            .filter_map(|elem| elem)
                            .collect()
                    }
                    //No any's intersection of both is result
                    ((false, a), (false, b)) => {
                        let intersection: Vec<VarShapeDeduction> = merge_join_by(a, b, |a, b| {
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
                                combination.many_of_shapes =
                                    combination.many_of_shapes && b_elem.many_of_shapes;
                                Some(combination)
                            }
                        })
                        .filter_map(|elem| elem)
                        .collect();
                        if intersection.is_empty() {
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
                            //TODO obay coercion rules
                            return Err(ShellError::diagnostic(
                                    Diagnostic::error()
                                    //TODO pass block and spans
                                    // How can you make spanned_expr to code?
                                    // .with_code(self.block.clone())
                                    .with_message(format!("Contrary types for variable {}. Variable can't be one of {:#?} and one of {:#?}", k.name,
                                        a.iter().map(|deduction| deduction.deduction).collect::<Vec<_>>(),
                                        b.iter().map(|deduction| deduction.deduction).collect::<Vec<_>>()
                                    ))
                                    // .with_labels( labels)
                                    ));
                        } else {
                            intersection
                        }
                    }
                };
                (k.clone(), combined_deductions)
            }
            None => (var_usage.clone(), new_deductions),
        };

        self.inferences.insert(insert_k, insert_v);
        Ok(())
    }
}
