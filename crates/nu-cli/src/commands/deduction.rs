use nu_protocol::{SyntaxShape, hir::{Variable, Binary, Operator, Expression, Block, ClassifiedCommand, Literal, SpannedExpression, Commands, NamedValue}, Signature, VarDeclaration, VarShapeDeduction};
use std::{collections::{HashSet, HashMap}};
use crate::CommandRegistry;
use nu_source::Span;
use nu_errors::ShellError;
use nu_parser::SignatureRegistry;

//TODO where to move this?
#[derive(Eq, Debug, Clone, Serialize, Deserialize, Hash)]
pub struct VarDeclaration{
    pub name: String,
    // type_decl: Option<UntaggedValue>,
    pub is_var_arg: bool,
    // scope: ?
    // pub tag: Tag, ?
    pub span: Span,
}

impl VarDeclaration{
    pub fn new(name: &str, span: Span) -> VarDeclaration{
        VarDeclaration{
            name: name.to_string(),
            is_var_arg: false,
            span,
        }
    }
}

impl PartialEq<VarDeclaration> for VarDeclaration{
    // When searching through the expressions, only the name of the
    // Variable is available. (TODO And their scope). Their full definition is not available.
    // Therefore the equals relationship is relaxed
    fn eq(&self, other: &VarDeclaration) -> bool {
        // TODO when scripting is available scope has to be respected
        self.name == other.name
            // && self.scope == other.scope
    }
}

//TODO implement iterator for this to iterate on it like a list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarShapeDeduction{
    var_decl: VarDeclaration,
    deduction: SyntaxShape,
    /// Spans pointing to the source of the deduction.
    /// The spans locate positions within the tag of var_decl
    deducted_from: Vec<Span>,
    /// For a command with a signature of:
    /// cmd [optional1] [optional2] <required>
    /// the resulting inference must be:
    /// optional1Shape or optional2Shape or requiredShape
    /// Thats a list of alternative shapes.
    /// This field stores a pointer to the possible next deduction
    alternative: Option<Box<VarShapeDeduction>>,
    /// Whether the variable can be substituted with the SyntaxShape deduction
    /// multiple times.
    /// For example a Var-Arg-Variable must be substituted when used in a cmd with
    /// a signature of:
    /// cmd [optionalPaths...] [integers...]
    /// with 2 SpannedVarShapeDeductions, where each can substitute multiple arguments
    many_of_shapes: bool
}
impl VarShapeDeduction{
    //TODO better naming
    pub fn from_usage(var_name: &str, deduced_from: &Span, deduced_shape: &SyntaxShape) -> VarShapeDeduction{
        VarShapeDeduction{
            var_decl: VarDeclaration{
                name: var_name.to_string(),
                is_var_arg: false,
                span: Span::unknown(),
            },
            deduction: deduced_shape.clone(),
            deducted_from: vec![deduced_from.clone()],
            alternative: None,
            many_of_shapes: false,
        }
    }
}


pub struct VarSyntaxShapeDeductor{
    //Initial set of caller provided var declarations
    var_declarations: Vec<VarDeclaration>,
    inferences: HashMap<VarDeclaration, VarShapeDeduction>,
    //var binary var
    dependencies: Vec<(VarUsage, SpannedExpression, VarUsage)>,
}

//TODO Where to put this
struct VarUsage{
    pub name: String,
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
    /// Returns: Mapping from var_to_find -> shape_deduction
    /// A mapping.get(var_to_find) == None means that no deduction
    /// has been found for var_to_find
    /// If a variable is used in at least 2 places with different
    /// required shapes, that do not coerce into each other,
    /// a error is returned
    pub fn infer_vars(
        vars_to_find: &Vec<VarDeclaration>,
        block: &Block,
        registry: &CommandRegistry,
    ) -> Result<HashMap<VarDeclaration, VarShapeDeduction>, ShellError> {

        let mut deducer = VarSyntaxShapeDeductor{
            var_declarations: vars_to_find.clone(),
            inferences: HashMap::new(),
            dependencies: Vec::new(),
        };
        deducer.infer_shape(block, registry)?;
        //Solve dependencies

        Ok(deducer.inferences)
    }

    fn infer_shape(
        &mut self,
        block: &Block,
        registry: &CommandRegistry,
    )-> Result<(), ShellError>{
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
        for (cmd_pipeline_idx, classified) in pipeline.list.iter().enumerate() {
            match &classified {
                ClassifiedCommand::Internal(internal) => {
                    if let Some(signature) = registry.get(&internal.name){
                        if let Some(positional) = &internal.args.positional {
                            //Infer shapes in positional
                            for (pos_idx, spanned_expr) in positional.iter().enumerate() {
                                self.infer_shapes_based_on_signature_positional(
                                    (pos_idx, &positional),
                                    spanned_expr,
                                    &signature,
                                )?;
                            }
                        }
                        if let Some(named) = &internal.args.named {
                            //Infer shapes in named
                            for (name, val) in named.iter() {
                                if let NamedValue::Value(_, spanned_expr) = val {
                                    self.infer_shapes_based_on_signature_named(
                                        name,
                                        spanned_expr,
                                        &signature,
                                    )?;
                                }
                            }
                        }
                    }
                    if let Some(positional) = &internal.args.positional {
                        //Infer shapes in positional
                        for (pos_idx, pos_expr) in positional.iter().enumerate() {
                            self.infer_shapes_in_expr(
                                (cmd_pipeline_idx, pipeline),
                                pos_expr,
                                registry)?;
                        }
                    }
                    if let Some(named) = &internal.args.named {
                        //Infer shapes in named
                        for (name, val) in named.iter() {
                            if let NamedValue::Value(_, named_expr) = val {
                                self.infer_shapes_in_expr(
                                    (cmd_pipeline_idx, pipeline),
                                    named_expr,
                                    registry)?;
                            }
                        }
                    }
                }
                ClassifiedCommand::Expr(spanned_expr) => {
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

    // pub fn infer_positional_var(
    //     &mut self,
    //     signature :Signature,
    //     positional: Vec<SpannedExpression>,
    //     i: usize){}


    fn infer_shapes_based_on_signature_positional(
        &mut self,
        (pos_idx, positionals): (usize, &Vec<SpannedExpression>),
        cur_pos: &SpannedExpression,
        signature: &Signature
    )-> Result<(), ShellError>{
        unimplemented!();
    }

    fn infer_shapes_based_on_signature_named(
        &mut self,
        name: &str,
        named_arg: &SpannedExpression,
        signature: &Signature,
    )-> Result<(), ShellError>{
        unimplemented!();
    }

    fn infer_shapes_in_expr(
        &mut self,
        (pipeline_idx, pipeline): (usize, &Commands),
        spanned_expr: &SpannedExpression,
        registry: &CommandRegistry,
    ) -> Result<(), ShellError>{
        match &spanned_expr.expr {
            Expression::Binary(bin) => {
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
                        self.checked_insert(&VarUsage::new(var_name, span),
                                            get_shapes_allowed_in_path().
                                            into_iter().map(|shape| VarShapeDeduction::new()).cloned().collect())?
                    }
                    _ => ()
                }
            }
            Expression::Range(range) => {
                if let Expression::Variable(Variable::Other(var_name, span)) = &range.left.expr{
                    self.checked_insert(&VarUsage::new(var_name, span),
                                        get_shapes_allowed_in_range().into_iter().map(|shape| (shape, spanned_expr.span)).collect())?;
                }else if let Expression::Variable(Variable::Other(var_name, span)) = &range.right.expr{
                    self.checked_insert(&VarUsage::new(var_name, span),
                                        get_shapes_allowed_in_range().into_iter().map(|shape| (shape, spanned_expr.span)).collect())?;
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
            Expression::Command(_) => {}
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
                if let Expression::Variable(Variable::It(l_span)) = &bin.left.expr{
                    if let Expression::Variable(Variable::It(r_span)) = &bin.right.expr{
                        //Example of this case is $foo * $it + $it
                        //The operator will be either logical or +/-/*... so keeping same operator here is fine
                        todo!("Figure out type of $it based on pipeline and return that")
                    }
                }
                //Shape will be either int, number, or unit
                let lhs_shape = self.get_shape_of_binary_arg(&bin.left,
                                                             (dependent_var, source_bin),
                                                             registry);
                let rhs_shape = self.get_shape_of_binary_arg(&bin.right,
                                                             (dependent_var, source_bin),
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
            Expression::List(content) => {todo!("List of inner shapes")}
            Expression::Boolean(_) => {todo!("What to return here? Is $var + true valid?")}

            //Rest should have failed at parsing stage? Realy? Also invocation? Old code says so...
            Expression::Path(_) | Expression::FilePath(_) | Expression::Block(_) | Expression::ExternalCommand(_) | Expression::Command(_) | Expression::Invocation(_) | Expression::Garbage => {unreachable!("Should have failed at parsing stage")}

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
        let bin = match bin_spanned.expr{
            Expression::Binary(bin) => bin,
            _ => unreachable!()
        };
        if let Expression::Literal(Literal::Operator(op)) = bin.op.expr{
            match &op{
                //For || and && we insert shapes decay able to bool
                Operator::And | Operator::Or => {
                    self.checked_insert(var.clone(),
                                        get_shapes_decay_able_to_bool().iter()
                                        .map(|shape| (shape.clone(), bin_spanned.span.clone())).collect())?;
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
                        self.checked_insert(&var_name,
                                            possible_shapes.into_iter().map(|shape| (shape, span(bin))).collect::<_>())?;
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
        let bin = match bin_spanned.expr{
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
            if let Expression::Variable(Variable::Other(right_var_name, _ )) = &bin.right.expr{
                todo!("Check return type of source (first command in pipeline), check that only data access (get row etc.) or data manipulation (not manipulating type), are between this cmd and source and if so, set right_var_name shape to return type of source.");
                //No further inference possible
                return Ok(());
            }
        }
        if let Expression::Variable(Variable::It(_)) = bin.right.expr{
            if let Expression::Variable(Variable::Other(left_var_name, _ )) = &bin.left.expr{
                todo!("Check return type of source (first command in pipeline), check that only data access (get row etc.) or data manipulation (not manipulating type), are between this cmd and source and if so, set right_var_name shape to return type of source.");
                //No further inference possible
                return Ok(());
            }
        }
        if let Expression::Variable(Variable::Other(left_var_name, l_span )) = &bin.left.expr{
            self.infer_shapes_between_var_and_expr(
                (VarUsage(left_var_name, l_span), &bin.right),
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
            //Descend deeper into bin tree if rhs is binary
            if let Expression::Binary(right_bin) = &bin.right.expr{
                self.infer_shapes_in_binary_expr((pipeline_idx, pipeline),
                                                 &bin.right,
                                                 registry)?;
            }
        }
        if let Expression::Variable(Variable::Other(right_var_name, r_span )) = &bin.right.expr{
            self.infer_shapes_between_var_and_expr(
                (VarUsage(right_var_name, r_span), &bin.right),
                bin_spanned,
                (pipeline_idx, pipeline),
                registry,
            )?;
            //Descend deeper into bin tree if lhs is binary
            if let Expression::Binary(left_bin) = &bin.left.expr{
                self.infer_shapes_in_binary_expr((pipeline_idx, pipeline),
                                                 &bin.left,
                                                 registry)?;
            }
        }

        Ok(())
    }


    fn checked_insert(&mut self, var: &VarUsage, new_deducts: Vec<VarShapeDeduction>) -> Result<(), ShellError> {
        if new_deducts.len() > 1{

        }
        //TODO Check special for var arg
        let (insert_k, insert_v) =
            match self.inferences.get_key_value(var) {
                Some((k, deduction)) => {
                    //TODO HANDLE SYNTAXSHAPE::ANY !!!!!
                    //Var has been infered already. Intersection of existing and new_infered_shapes is new possible shape
                    //TODO is there a way to have an hashset with custom hash fn and custom eq fn?
                    //If so we can preserve the spans here


                    let existing_shapes : HashSet<SyntaxShape> = existing_deducts.iter().map(|(shape, span)| shape.clone()).collect();
                    let new_shapes : HashSet<SyntaxShape> = new_deducts.iter().map(|(shape, span)| shape.clone()).collect();

                    let intersection: Vec<SyntaxShape> = existing_shapes.intersection(&new_shapes).cloned().collect();

                    //Find spans again
                    let intersection = existing_deducts.iter().filter(|(shape, span)| intersection.iter().any(|s| s == shape))
                        .chain(new_deducts.iter().filter(|(shape, span)| intersection.iter().any(|s| s == shape)))
                        .cloned().collect();

                    (k.clone(), ::OneOf(intersection))
                }
                Some((_k, VarSyntaxShapeInference::MultipleOneOf(_shapes, _rest))) => {
                    //TODO infer var arg
                    unimplemented!()

                }
                None => {
                    match self.var_declarations.iter().find(|decl| decl.name == var_name){
                        None => {
                            //Variable wasn't in parameter list
                            (VarDeclaration::new(var_name), VarSyntaxShapeInference::OneOf(new_deducts))
                        }
                        Some(var_decl) => {
                            //TODO different insert depending on is_var_arg flag
                            (var_decl.clone(), VarSyntaxShapeInference::OneOf(new_deducts))
                        }
                    }
                }
            };

        self.inferences.insert(insert_k, insert_v);
        Ok(())
    }


}