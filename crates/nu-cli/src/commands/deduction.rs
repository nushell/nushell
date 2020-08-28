use nu_protocol::{SyntaxShape, hir::{Variable, Operator, Expression, Block, ClassifiedCommand, Literal, SpannedExpression, Commands, NamedValue, NamedArguments}, Signature, PositionalType, NamedType};
use std::{collections::{HashMap}, hash::Hash};
use crate::CommandRegistry;
use serde::{Deserialize, Serialize};
use nu_source::Span;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;

use itertools::{merge_join_by, EitherOrBoth};
use log::trace;

//TODO where to move this?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDeclaration{
    pub name: String,
    // type_decl: Option<UntaggedValue>,
    pub is_var_arg: bool,
    // scope: ?
    // pub tag: Tag, ?
    pub span: Span,
}


impl VarDeclaration{
    // pub fn new(name: &str, span: Span) -> VarDeclaration{
    //     VarDeclaration{
    //         name: name.to_string(),
    //         is_var_arg: false,
    //         span,
    //     }
    // }
}


//TODO implement iterator for this to iterate on it like a list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarShapeDeduction{
    pub deduction: SyntaxShape,
    /// Spans pointing to the source of the deduction.
    /// The spans locate positions within the tag of var_decl
    pub deducted_from: Vec<Span>,
    /// For a command with a signature of:
    /// cmd [optional1] [optional2] <required>
    /// the resulting inference must be:
    /// optional1Shape or optional2Shape or requiredShape
    /// Thats a list of alternative shapes.
    /// This field stores a pointer to the possible next deduction
    // alternative: Option<Box<VarShapeDeduction>>,
    /// Whether the variable can be substituted with the SyntaxShape deduction
    /// multiple times.
    /// For example a Var-Arg-Variable must be substituted when used in a cmd with
    /// a signature of:
    /// cmd [optionalPaths...] [integers...]
    /// with 2 SpannedVarShapeDeductions, where each can substitute multiple arguments
    pub many_of_shapes: bool
}

impl VarShapeDeduction{
    //TODO better naming
    pub fn from_usage(usage: &Span, deduced_shape: &SyntaxShape) -> VarShapeDeduction{
        VarShapeDeduction{
            deduction: deduced_shape.clone(),
            deducted_from: vec![usage.clone()],
            many_of_shapes: false,
        }
    }

    pub fn from_usage_with_alternatives(usage: &Span, alternatives: &Vec<SyntaxShape>) -> Vec<VarShapeDeduction>{
        if alternatives.len() == 0{unreachable!("Calling this fn with 0 alternatives is probably invalid")}
        alternatives.iter().map(|shape| VarShapeDeduction::from_usage(usage, shape)).collect()
    }
}

pub struct VarSyntaxShapeDeductor{
    //Initial set of caller provided var declarations
    var_declarations: Vec<VarDeclaration>,
    inferences: HashMap<VarUsage, Vec<VarShapeDeduction>>,
    //var binary var
    dependencies: Vec<(VarUsage, SpannedExpression, VarUsage)>,
}

//TODO Where to put this
#[derive(Clone, Debug, Eq)]
pub struct VarUsage{
    pub name: String,
    /// Span describing where this var is used
    pub span: Span,
    //pub scope: ?
}
impl VarUsage{
    pub fn new(name: &str, span: &Span) -> VarUsage{
        VarUsage{
            name: name.to_string(),
            span: span.clone(),
        }
    }
}

impl PartialEq<VarUsage> for VarUsage{
    // When searching through the expressions, only the name of the
    // Variable is available. (TODO And their scope). Their full definition is not available.
    // Therefore the equals relationship is relaxed
    fn eq(&self, other: &VarUsage) -> bool {
        // TODO when scripting is available scope has to be respected
        self.name == other.name
        // && self.scope == other.scope
    }
}

impl Hash for VarUsage{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        return self.name.hash(state);
    }

}

impl From<VarDeclaration> for VarUsage{
    fn from(decl: VarDeclaration) -> Self {
        //Span unknown as multiple options are possible
        VarUsage::new(&decl.name, &Span::unknown())
    }
}
impl From<&VarDeclaration> for VarUsage{
    fn from(decl: &VarDeclaration) -> Self {
        //Span unknown as multiple options are possible
        VarUsage::new(&decl.name, &Span::unknown())
    }
}

//TODO Where to put these
fn get_shapes_allowed_in_path() -> Vec<SyntaxShape>{
    vec![SyntaxShape::Int, SyntaxShape::String]
}

fn get_shapes_decay_able_to_bool() -> Vec<SyntaxShape>{
    // todo!("What types are decay able to bool?");
    vec![SyntaxShape::Int]
}

fn get_shapes_allowed_in_range() -> Vec<SyntaxShape>{
    vec![SyntaxShape::Int]
}

impl VarSyntaxShapeDeductor{
    /// Deduce vars_to_find in block.
    /// Returns: Mapping from var_to_find -> Vec<shape_deduction>
    /// in which each shape_deduction is one possible deduction for the variable
    /// A mapping.get(var_to_find) == None means that no deduction
    /// has been found for var_to_find
    /// If a variable is used in at least 2 places with different
    /// required shapes, that do not coerce into each other,
    /// an error is returned
    pub fn infer_vars(
        vars_to_find: &Vec<VarDeclaration>,
        block: &Block,
        registry: &CommandRegistry,
    ) -> Result<Vec<(VarDeclaration, Option<Vec<VarShapeDeduction>>)>, ShellError> {
        trace!("Deducing shapes for vars: {:?}", vars_to_find);

        let mut deducer = VarSyntaxShapeDeductor{
            var_declarations: vars_to_find.clone(),
            inferences: HashMap::new(),
            dependencies: Vec::new(),
        };
        deducer.infer_shape(block, registry)?;
        //Solve dependencies
        trace!("Found shapes for vars: {:?}", deducer.inferences);


        //Remove unwanted vars
        Ok(deducer.var_declarations.iter().map(|decl| {
            let usage: VarUsage = decl.into();
            let deductions = match deducer.inferences.get(&usage){
                Some(vec) => {Some(vec.clone())}
                None => {None}
            };
            (decl.clone(), deductions)
        }).collect())
    }

    fn infer_shape(
        &mut self,
        block: &Block,
        registry: &CommandRegistry,
    )-> Result<(), ShellError>{
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
    )->Result<(), ShellError>{
        trace!("Infering vars in pipeline");
        for (cmd_pipeline_idx, classified) in pipeline.list.iter().enumerate() {
            match &classified {
                ClassifiedCommand::Internal(internal) => {
                    if let Some(signature) = registry.get(&internal.name){
                        if let Some(positional) = &internal.args.positional {
                            //Infer shapes in positional
                            self.infer_shapes_based_on_signature_positional(
                                positional,
                                &signature,
                            )?;
                        }
                        if let Some(named) = &internal.args.named {
                            //Infer shapes in named
                            self.infer_shapes_based_on_signature_named(
                                named,
                                &signature,
                            )?;
                        }
                    }
                    if let Some(positional) = &internal.args.positional {
                        trace!("Infering vars in positional exprs");
                        for (_pos_idx, pos_expr) in positional.iter().enumerate() {
                            if let Expression::Variable(Variable::Other(_, _ )) = &pos_expr.expr{
                                trace!("Skipping handled var");
                                //Should have been handled above!
                                continue;
                            }
                            self.infer_shapes_in_expr(
                                (cmd_pipeline_idx, pipeline),
                                pos_expr,
                                registry)?;
                        }
                    }
                    if let Some(named) = &internal.args.named {
                        trace!("Infering vars in named exprs");
                        for (_name, val) in named.iter() {
                            if let NamedValue::Value(_, named_expr) = val {
                                self.infer_shapes_in_expr(
                                    (cmd_pipeline_idx, pipeline),
                                    named_expr,
                                    registry)?;
                            }
                        }
                    }
                }
                ClassifiedCommand::Expr(_spanned_expr) => {
                    // let found = infer_shapes_in_expr(&var, &spanned_expr, registry)?;
                    // check_merge(&mut arg_shapes, &found)?
                    unimplemented!()
                }
                ClassifiedCommand::Dynamic(_) | ClassifiedCommand::Error(_) => {
                    unimplemented!()
                },
            }
        }
        Ok(())
    }

    fn infer_shapes_based_on_signature_positional(
        &mut self,
        positionals: &Vec<SpannedExpression>,
        signature: &Signature,
    )-> Result<(), ShellError>{
        trace!("Infering vars in positionals");
        // todo!("If current pos is optional check that all expr behind cur_pos are shiftable by 1");
        //Currently we assume var is fitting optional parameter
        trace!("Positionals len: {:?}", positionals.len());
        for (pos_idx, positional) in positionals.iter().enumerate().rev(){
            trace!("Handling pos_idx: {:?} of type: {:?}", pos_idx, positional);
            if let Expression::Variable(Variable::Other(var_name, _)) = &positional.expr{
                let deduced_shape = {
                    if pos_idx >= signature.positional.len(){
                        if let Some((shape, _)) = &signature.rest_positional{
                            Some(shape)
                        }else{
                            //TODO let this throw an error?
                            unreachable!("Should have failed at parsing stage!");
                        }
                    }else{
                        match &signature.positional[pos_idx].0{
                            PositionalType::Mandatory(_, shape) | PositionalType::Optional(_, shape) => Some(shape)
                        }
                    }
                };
                trace!("Found var: {:?} in positional_idx: {:?} of shape: {:?}", var_name, pos_idx, deduced_shape);
                if let Some(shape) = deduced_shape{
                    self.checked_insert(
                        &VarUsage::new(var_name, &positional.span),
                        vec![VarShapeDeduction::from_usage(&positional.span, shape)]
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
    )-> Result<(), ShellError>{
        trace!("Infering vars in named");
        for (name, val) in named.iter() {
            if let NamedValue::Value(span, spanned_expr) = &val {
                if let Expression::Variable(Variable::Other(var_name, _)) = &spanned_expr.expr{
                    if let Some((named_type, _)) = signature.named.get(name){
                        if let NamedType::Mandatory(_, shape) | NamedType::Optional(_, shape) = named_type{
                            trace!("Found var: {:?} in named: {:?} of shape: {:?}", var_name, name, shape);
                            self.checked_insert(&VarUsage::new(var_name, span),
                                vec![VarShapeDeduction::from_usage(span, shape)])?;
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
    ) -> Result<(), ShellError>{
        match &spanned_expr.expr {
            Expression::Binary(_) => {
                self.infer_shapes_in_binary_expr((pipeline_idx, pipeline), spanned_expr, registry)?;
            }
            Expression::Block(b) => self.infer_shape(&b, registry)?,
            Expression::Path(path) => {
                match &path.head.expr {
                    //TODO Iterate over path parts and find var
                    //Currently no vars in path allowed???
                    //TODO what is invocation and why is it allowed in path head?
                    Expression::Invocation(b) => self.infer_shape(&b, registry)?,
                    Expression::Variable(Variable::Other(var_name, span)) => {
                        self.checked_insert(
                            &VarUsage::new(var_name, span),
                            VarShapeDeduction::from_usage_with_alternatives(&span, &get_shapes_allowed_in_path())
                        )?;
                    }
                    _ => ()
                }
            }
            Expression::Range(range) => {
                if let Expression::Variable(Variable::Other(var_name, _)) = &range.left.expr{
                    self.checked_insert(
                        &VarUsage::new(var_name, &spanned_expr.span),
                        VarShapeDeduction::from_usage_with_alternatives(
                            &spanned_expr.span, &get_shapes_allowed_in_range())
                    )?;
                }else if let Expression::Variable(Variable::Other(var_name, span)) = &range.right.expr{
                    self.checked_insert(
                        &VarUsage::new(var_name, &spanned_expr.span),
                        VarShapeDeduction::from_usage_with_alternatives(
                            span, &get_shapes_allowed_in_range())
                    )?;
                }
            }
            Expression::List(inner_exprs) => {
                for expr in inner_exprs{
                    self.infer_shapes_in_expr((pipeline_idx, pipeline), expr, registry)?;
                }
            }
            Expression::Invocation(invoc) => {
                self.infer_shape(invoc , registry)?;
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

    fn get_shape_of_binary_arg(&mut self,
                               //var depending on shape of expr (arg)
                               (var, expr) : (&VarUsage, &SpannedExpression),
                               //source_bin is binary having var on one and expr on other side
                               source_bin: &SpannedExpression,
                               (pipeline_idx, pipeline): (usize, &Commands),
                               registry: &CommandRegistry) -> Option<Vec<SyntaxShape>>
    {
        // let bin = match source_bin.expr{
        //     Expression::Binary(bin) => bin,
        //     _ => unreachable!()
        // };
        match &expr.expr{
            //If both are variable
            Expression::Variable(_) => {unreachable!("case that both sides are var is handled on caller side")}
            Expression::Literal(literal) => {
                match literal{
                    nu_protocol::hir::Literal::Number(_) => {Some(vec![SyntaxShape::Number])}
                    nu_protocol::hir::Literal::Size(_, _) => {Some(vec![SyntaxShape::Unit])}
                    nu_protocol::hir::Literal::String(_) => {Some(vec![SyntaxShape::String])}
                    //Rest should have failed at parsing stage?
                    nu_protocol::hir::Literal::GlobPattern(_) => {Some(vec![SyntaxShape::String])}
                    nu_protocol::hir::Literal::Operator(_) => {Some(vec![SyntaxShape::Operator])}
                    nu_protocol::hir::Literal::ColumnPath(_) => {Some(vec![SyntaxShape::ColumnPath])}
                    nu_protocol::hir::Literal::Bare(_) => {Some(vec![SyntaxShape::String])}
                }
            }
            //What do these both mean?
            Expression::ExternalWord => {Some(vec![SyntaxShape::String])}
            Expression::Synthetic(_) => {Some(vec![SyntaxShape::String])}

            Expression::Binary(bin) => {
                //if both sides of bin are variables, no deduction will be possible, therefore
                // dependend_var depends on both these vars
                if let Expression::Variable(Variable::Other(lhs_var_name, l_span)) = &bin.left.expr{
                    if let Expression::Variable(Variable::Other(rhs_var_name, r_span)) = &bin.right.expr{
                        //Example of this case is: $foo * $bar + $baz
                        //The operator will be either logical or +/-/*... so keeping same operator here is fine
                        self.dependencies.push((var.clone(), source_bin.clone(), VarUsage::new(lhs_var_name, l_span)));
                        self.dependencies.push((var.clone(), source_bin.clone(), VarUsage::new(rhs_var_name, r_span)));
                        return None;
                    }
                }
                if let Expression::Variable(Variable::It(_l_span)) = &bin.left.expr{
                    if let Expression::Variable(Variable::It(_)) = &bin.right.expr{
                        //Example of this case is $foo * $it + $it
                        //The operator will be either logical or +/-/*... so keeping same operator here is fine
                        todo!("Figure out type of $it based on pipeline and return that")
                    }
                }
                //Shape will be either int, number, or unit
                let lhs_shape = self.get_shape_of_binary_arg((var, &bin.left),
                                                             source_bin,
                                                             (pipeline_idx, pipeline),
                                                             registry);
                let rhs_shape = self.get_shape_of_binary_arg((var, &bin.right),
                                                             source_bin,
                                                             (pipeline_idx, pipeline),
                                                             registry);
                //Is this correct? I think yes
                if let Some(lhs) = &lhs_shape{
                    if let Some(rhs) = &rhs_shape{
                        assert_eq!(lhs, rhs);
                    }
                }
                lhs_shape
            }
            Expression::Range(_) => {Some(vec![SyntaxShape::Range])}
            Expression::List(_) => {todo!("List of inner shapes")}
            Expression::Boolean(_) => {todo!("What to return here? Is $var + true valid?")}

            //Rest should have failed at parsing stage? Realy? Also invocation? Old code says so...
            Expression::Path(_) | Expression::FilePath(_) | Expression::Block(_) | Expression::ExternalCommand(_) | Expression::Command | Expression::Invocation(_) | Expression::Garbage => {unreachable!("Should have failed at parsing stage")}

        }
    }



    fn infer_shapes_between_var_and_expr(
        &mut self,
        (var, expr): (&VarUsage, &SpannedExpression),
        //Binary having expr on one side and var on other
        bin_spanned: &SpannedExpression,
        (pipeline_idx, pipeline): (usize, &Commands),
        registry: &CommandRegistry,
    )-> Result<(), ShellError>{
        let bin = match &bin_spanned.expr{
            Expression::Binary(bin) => bin,
            _ => unreachable!()
        };
        if let Expression::Literal(Literal::Operator(op)) = bin.op.expr{
            match &op{
                //For || and && we insert shapes decay able to bool
                Operator::And | Operator::Or => {
                    self.checked_insert(
                        &var,
                        VarShapeDeduction::from_usage_with_alternatives(
                            &var.span, &get_shapes_decay_able_to_bool())
                    )?;
                },
                Operator::In | Operator::NotIn | Operator::Contains | Operator::NotContains => {
                    todo!("Implement in, notin, contains... for type deduction");
                },
                Operator::Equal | Operator::NotEqual | Operator::LessThan | Operator::GreaterThan |
                Operator::LessThanOrEqual | Operator::GreaterThanOrEqual |
                Operator::Plus | Operator::Minus | Operator::Multiply | Operator::Divide => {
                    if let Some(possible_shapes) = self.get_shape_of_binary_arg(
                        (var, expr),
                        bin_spanned,
                        (pipeline_idx, pipeline),
                        registry){
                        self.checked_insert(
                            var,
                            VarShapeDeduction::from_usage_with_alternatives(
                                &var.span, &possible_shapes)
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
    ) -> Result<(), ShellError>{
        let bin = match &bin_spanned.expr{
            Expression::Binary(bin) => {bin}
            _ => unreachable!()
        };

        if let Expression::Variable(Variable::Other(left_var_name, l_span)) = &bin.left.expr{
            if let Expression::Variable(Variable::Other(right_var_name, r_span )) = &bin.right.expr{
                if left_var_name != right_var_name{
                    //type can't be deduced out of this, so add it to resolve it later
                    self.dependencies.push(
                        (VarUsage::new(left_var_name, l_span), bin_spanned.clone(), VarUsage::new(right_var_name, r_span)));
                }
                //No further inference possible
                return Ok(());
            }
        }
        if let Expression::Variable(Variable::It(_)) = bin.left.expr{
            if let Expression::Variable(Variable::Other(_right_var_name, _ )) = &bin.right.expr{
                todo!("Check return type of source (first command in pipeline), check that only data access (get row etc.) or data manipulation (not manipulating type), are between this cmd and source and if so, set right_var_name shape to return type of source.");
                //No further inference possible
                // return Ok(());
            }
        }
        if let Expression::Variable(Variable::It(_)) = bin.right.expr{
            if let Expression::Variable(Variable::Other(_left_var_name, _ )) = &bin.left.expr{
                todo!("Check return type of source (first command in pipeline), check that only data access (get row etc.) or data manipulation (not manipulating type), are between this cmd and source and if so, set right_var_name shape to return type of source.");
                //No further inference possible
                // return Ok(());
            }
        }
        if let Expression::Variable(Variable::Other(left_var_name, l_span )) = &bin.left.expr{
            self.infer_shapes_between_var_and_expr(
                (&VarUsage::new(left_var_name, l_span), &bin.right),
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
            //Descend deeper into bin tree if rhs is binary
            if let Expression::Binary(_) = &bin.right.expr{
                self.infer_shapes_in_binary_expr((pipeline_idx, pipeline),
                                                 &bin.right,
                                                 registry)?;
            }
        }
        if let Expression::Variable(Variable::Other(right_var_name, r_span )) = &bin.right.expr{
            self.infer_shapes_between_var_and_expr(
                (&VarUsage::new(right_var_name, r_span), &bin.right),
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
            //Descend deeper into bin tree if lhs is binary
            if let Expression::Binary(_) = &bin.left.expr{
                self.infer_shapes_in_binary_expr((pipeline_idx, pipeline),
                                                 &bin.left,
                                                 registry)?;
            }
        }

        Ok(())
    }

    // fn shape_cmp(a: &VarShapeDeduction, b: &VarShapeDeduction) -> Ordering{
    //     (a.deduction as i32).cmp(b.deduction as i32)
    // }

    /// Inserts the new deductions. Each VarShapeDeduction represents one alternative for
    /// the variable described by var_usage

    /// Each of the deductions is assumed to be for the same variable
    /// Each of the deductions is assumed to be unique of shape
    fn checked_insert(&mut self, var_usage: &VarUsage, new_deductions: Vec<VarShapeDeduction>) -> Result<(), ShellError> {
        trace!("Trying to insert for: {:?} possible shapes:{:?}", var_usage.name, new_deductions.iter().map(|d|d.deduction).collect::<Vec<_>>());

        //Every insertion is sorted by shape!
        //Everything within self.inferences is sorted by shape!
        // let cmp_fn = |a: &VarShapeDeduction, b: &VarShapeDeduction| -> Ordering ;
        let mut new_deductions = new_deductions;
        new_deductions.sort_unstable_by(|a,b| (a.deduction.clone() as i32).cmp(&(b.deduction.clone() as i32)));
        //TODO Check special for var arg
        let (insert_k, insert_v) =
            match self.inferences.get_key_value(&var_usage) {
                Some((k, existing_deductions)) => {
                    // If there is one any in one deduction, this deduction is capable of representing the other
                    // deduction and vice versa
                    let (any_in_new, new_vec) =
                        (new_deductions.iter().any(|deduc| deduc.deduction == SyntaxShape::Any), &new_deductions);
                    let (any_in_existing, existing_vec) =
                        (existing_deductions.iter().any(|deduc| deduc.deduction == SyntaxShape::Any), existing_deductions);

                    let combined_deductions = match ((any_in_new, new_vec), (any_in_existing, existing_vec)){
                        ((true, a), (true, b)) => {
                            //In each alternative there is any
                            //complete merge each set |
                            //TODO move closure into function. But the compiler sheds tears to much for me :F
                            merge_join_by(a, b, |a,b| (a.deduction.clone() as i32).cmp(&(b.deduction.clone() as i32)))
                                .map(|either_or|
                                        match either_or{
                                        EitherOrBoth::Left(deduc) | EitherOrBoth::Right(deduc) => deduc.clone(),
                                        EitherOrBoth::Both(a_elem, b_elem) => {
                                            let mut combination = a_elem.clone();
                                            combination.deducted_from.extend(&b_elem.deducted_from);
                                            combination.many_of_shapes = combination.many_of_shapes && b_elem.many_of_shapes;
                                            combination
                                        }
                                    }
                                ).collect()
                        }
                        ((false, a), (true, b)) | ((true, b), (false, a)) =>{
                            //B has an any. So A can be applied as a whole
                            // So result is intersection(b,a) + a
                            merge_join_by(a, b, |a,b| (a.deduction.clone() as i32).cmp(&(b.deduction.clone() as i32)))
                                .map(|either_or|
                                     match either_or{
                                         //Left is a, right is b
                                         //(a + none) + a is a
                                         EitherOrBoth::Left(deduc) => Some(deduc.clone()),
                                         //(none + b) + a is a
                                         EitherOrBoth::Right(_) => None,
                                         //(a + b) + a is (a + b)
                                         EitherOrBoth::Both(a_elem, b_elem) => {
                                             let mut combination = a_elem.clone();
                                             combination.deducted_from.extend(&b_elem.deducted_from);
                                             combination.many_of_shapes = combination.many_of_shapes && b_elem.many_of_shapes;
                                             Some(combination)
                                         }
                                     }
                                ).filter_map(|elem| elem)
                                .collect()
                        }
                        //No any's intersection of both is result
                        ((false, a), (false, b)) => {
                            let intersection: Vec<VarShapeDeduction> =
                                merge_join_by(a, b, |a,b|(a.deduction as i32).cmp(&(b.deduction as i32)))
                                .map(|either_or|
                                     match either_or{
                                         //Left is a, right is b
                                         EitherOrBoth::Left(_) => None,
                                         EitherOrBoth::Right(_) => None,
                                         EitherOrBoth::Both(a_elem, b_elem) => {
                                             let mut combination = a_elem.clone();
                                             combination.deducted_from.extend(b_elem.deducted_from.clone());
                                             combination.many_of_shapes = combination.many_of_shapes && b_elem.many_of_shapes;
                                             Some(combination)
                                         }
                                     }
                                ).filter_map(|elem|elem)
                                .collect();
                            if intersection.len() == 0{
                                //TODO obay coercion rules
                                todo!("Contrary needs for variable: What to return here?");
                                // return ShellError::argument_error(command, kind)

                            }else{
                                intersection
                            }
                        }
                    };
                    (k.clone(), combined_deductions)
                }
                None => {
                    (var_usage.clone(), new_deductions)
                }
            };

        self.inferences.insert(insert_k, insert_v);
        Ok(())
    }
}

        // let mut alias = Signature::build(&self.name);

        // for (arg, deducted_shape) in self.args.iter().take_while(|arg| !arg.0.is_var_arg) {
        //     let shape = match deducted_shape{
        //         //TODO allow signatures with multiple versions. For now we pick first
        //         VarSyntaxShapeInference::OneOf(options) => {
        //             let default = (SyntaxShape::Any, Span::default());
        //             options.first().unwrap_or(&default).0
        //         }
        //         VarSyntaxShapeInference::MultipleOneOf(_, _) => {unreachable!()}
        //     };
        //     //TODO add "deducted by span" as explanation?
        //     alias = alias.required(arg.name.clone(), shape, "");
        // }

        // //If we have an var arg
        // //TODO add var arg
        // // if let Some((arg, shape)) = self.args.last() {
        // //     if is_var_arg(arg){
        // //         //Added above to positionals, move it to rest
        // //         alias.positional.pop();
        // //         alias = alias.rest(*shape, arg);
        // //     }
        // // }

        // alias