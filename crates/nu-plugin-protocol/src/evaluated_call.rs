use nu_protocol::{
    ast::{self, Expression},
    engine::{Call, CallImpl, EngineState, Stack},
    ir, FromValue, ShellError, Span, Spanned, Value,
};
use serde::{Deserialize, Serialize};

/// A representation of the plugin's invocation command including command line args
///
/// The `EvaluatedCall` contains information about the way a `Plugin` was invoked representing the
/// [`Span`] corresponding to the invocation as well as the arguments it was invoked with. It is
/// one of the items passed to `PluginCommand::run()`, along with the plugin reference, the engine
/// interface, and a [`Value`] that represents the input.
///
/// The evaluated call is used with the Plugins because the plugin doesn't have
/// access to the Stack and the EngineState the way a built in command might. For that
/// reason, before encoding the message to the plugin all the arguments to the original
/// call (which are expressions) are evaluated and passed to Values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedCall {
    /// Span of the command invocation
    pub head: Span,
    /// Values of positional arguments
    pub positional: Vec<Value>,
    /// Names and values of named arguments
    pub named: Vec<(Spanned<String>, Option<Value>)>,
}

impl EvaluatedCall {
    /// Create a new [`EvaluatedCall`] with the given head span.
    pub fn new(head: Span) -> EvaluatedCall {
        EvaluatedCall {
            head,
            positional: vec![],
            named: vec![],
        }
    }

    /// Add a positional argument to an [`EvaluatedCall`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::{Value, Span, IntoSpanned};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let head = Span::test_data();
    /// let mut call = EvaluatedCall::new(head);
    /// call.add_positional(Value::test_int(1337));
    /// ```
    pub fn add_positional(&mut self, value: Value) -> &mut Self {
        self.positional.push(value);
        self
    }

    /// Add a named argument to an [`EvaluatedCall`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::{Value, Span, IntoSpanned};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let head = Span::test_data();
    /// let mut call = EvaluatedCall::new(head);
    /// call.add_named("foo".into_spanned(head), Value::test_string("bar"));
    /// ```
    pub fn add_named(&mut self, name: Spanned<impl Into<String>>, value: Value) -> &mut Self {
        self.named.push((name.map(Into::into), Some(value)));
        self
    }

    /// Add a flag argument to an [`EvaluatedCall`]. A flag argument is a named argument with no
    /// value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nu_protocol::{Value, Span, IntoSpanned};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let head = Span::test_data();
    /// let mut call = EvaluatedCall::new(head);
    /// call.add_flag("pretty".into_spanned(head));
    /// ```
    pub fn add_flag(&mut self, name: Spanned<impl Into<String>>) -> &mut Self {
        self.named.push((name.map(Into::into), None));
        self
    }

    /// Builder variant of [`.add_positional()`](Self::add_positional).
    pub fn with_positional(mut self, value: Value) -> Self {
        self.add_positional(value);
        self
    }

    /// Builder variant of [`.add_named()`](Self::add_named).
    pub fn with_named(mut self, name: Spanned<impl Into<String>>, value: Value) -> Self {
        self.add_named(name, value);
        self
    }

    /// Builder variant of [`.add_flag()`](Self::add_flag).
    pub fn with_flag(mut self, name: Spanned<impl Into<String>>) -> Self {
        self.add_flag(name);
        self
    }

    /// Try to create an [`EvaluatedCall`] from a command `Call`.
    pub fn try_from_call(
        call: &Call,
        engine_state: &EngineState,
        stack: &mut Stack,
        eval_expression_fn: fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>,
    ) -> Result<Self, ShellError> {
        match &call.inner {
            CallImpl::AstRef(call) => {
                Self::try_from_ast_call(call, engine_state, stack, eval_expression_fn)
            }
            CallImpl::AstBox(call) => {
                Self::try_from_ast_call(call, engine_state, stack, eval_expression_fn)
            }
            CallImpl::IrRef(call) => Self::try_from_ir_call(call, stack),
            CallImpl::IrBox(call) => Self::try_from_ir_call(call, stack),
        }
    }

    fn try_from_ast_call(
        call: &ast::Call,
        engine_state: &EngineState,
        stack: &mut Stack,
        eval_expression_fn: fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>,
    ) -> Result<Self, ShellError> {
        let positional =
            call.rest_iter_flattened(0, |expr| eval_expression_fn(engine_state, stack, expr))?;

        let mut named = Vec::with_capacity(call.named_len());
        for (string, _, expr) in call.named_iter() {
            let value = match expr {
                None => None,
                Some(expr) => Some(eval_expression_fn(engine_state, stack, expr)?),
            };

            named.push((string.clone(), value))
        }

        Ok(Self {
            head: call.head,
            positional,
            named,
        })
    }

    fn try_from_ir_call(call: &ir::Call, stack: &Stack) -> Result<Self, ShellError> {
        let positional = call.rest_iter_flattened(stack, 0)?;

        let mut named = Vec::with_capacity(call.named_len(stack));
        named.extend(
            call.named_iter(stack)
                .map(|(name, value)| (name.map(|s| s.to_owned()), value.cloned())),
        );

        Ok(Self {
            head: call.head,
            positional,
            named,
        })
    }

    /// Check if a flag (named parameter that does not take a value) is set
    /// Returns Ok(true) if flag is set or passed true value
    /// Returns Ok(false) if flag is not set or passed false value
    /// Returns Err if passed value is not a boolean
    ///
    /// # Examples
    /// Invoked as `my_command --foo`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         None
    /// #     )],
    /// # };
    /// assert!(call.has_flag("foo").unwrap());
    /// ```
    ///
    /// Invoked as `my_command --bar`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "bar".to_owned(), span: null_span},
    /// #         None
    /// #     )],
    /// # };
    /// assert!(!call.has_flag("foo").unwrap());
    /// ```
    ///
    /// Invoked as `my_command --foo=true`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         Some(Value::bool(true, Span::unknown()))
    /// #     )],
    /// # };
    /// assert!(call.has_flag("foo").unwrap());
    /// ```
    ///
    /// Invoked as `my_command --foo=false`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         Some(Value::bool(false, Span::unknown()))
    /// #     )],
    /// # };
    /// assert!(!call.has_flag("foo").unwrap());
    /// ```
    ///
    /// Invoked with wrong type as `my_command --foo=1`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         Some(Value::int(1, Span::unknown()))
    /// #     )],
    /// # };
    /// assert!(call.has_flag("foo").is_err());
    /// ```
    pub fn has_flag(&self, flag_name: &str) -> Result<bool, ShellError> {
        for name in &self.named {
            if flag_name == name.0.item {
                return match &name.1 {
                    Some(Value::Bool { val, .. }) => Ok(*val),
                    None => Ok(true),
                    Some(result) => Err(ShellError::CantConvert {
                        to_type: "bool".into(),
                        from_type: result.get_type().to_string(),
                        span: result.span(),
                        help: Some("".into()),
                    }),
                };
            }
        }

        Ok(false)
    }

    /// Returns the [`Span`] of the name of an optional named argument.
    ///
    /// This can be used in errors for named arguments that don't take values.
    pub fn get_flag_span(&self, flag_name: &str) -> Option<Span> {
        self.named
            .iter()
            .find(|(name, _)| name.item == flag_name)
            .map(|(name, _)| name.span)
    }

    /// Returns the [`Value`] of an optional named argument
    ///
    /// # Examples
    /// Invoked as `my_command --foo 123`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         Some(Value::int(123, null_span))
    /// #     )],
    /// # };
    /// let opt_foo = match call.get_flag_value("foo") {
    ///     Some(Value::Int { val, .. }) => Some(val),
    ///     None => None,
    ///     _ => panic!(),
    /// };
    /// assert_eq!(opt_foo, Some(123));
    /// ```
    ///
    /// Invoked as `my_command`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![],
    /// # };
    /// let opt_foo = match call.get_flag_value("foo") {
    ///     Some(Value::Int { val, .. }) => Some(val),
    ///     None => None,
    ///     _ => panic!(),
    /// };
    /// assert_eq!(opt_foo, None);
    /// ```
    pub fn get_flag_value(&self, flag_name: &str) -> Option<Value> {
        for name in &self.named {
            if flag_name == name.0.item {
                return name.1.clone();
            }
        }

        None
    }

    /// Returns the [`Value`] of a given (zero indexed) positional argument, if present
    ///
    /// Examples:
    /// Invoked as `my_command a b c`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: vec![
    /// #         Value::string("a".to_owned(), null_span),
    /// #         Value::string("b".to_owned(), null_span),
    /// #         Value::string("c".to_owned(), null_span),
    /// #     ],
    /// #     named: vec![],
    /// # };
    /// let arg = match call.nth(1) {
    ///     Some(Value::String { val, .. }) => val,
    ///     _ => panic!(),
    /// };
    /// assert_eq!(arg, "b".to_owned());
    ///
    /// let arg = call.nth(7);
    /// assert!(arg.is_none());
    /// ```
    pub fn nth(&self, pos: usize) -> Option<Value> {
        self.positional.get(pos).cloned()
    }

    /// Returns the value of a named argument interpreted as type `T`
    ///
    /// # Examples
    /// Invoked as `my_command --foo 123`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         Some(Value::int(123, null_span))
    /// #     )],
    /// # };
    /// let foo = call.get_flag::<i64>("foo");
    /// assert_eq!(foo.unwrap(), Some(123));
    /// ```
    ///
    /// Invoked as `my_command --bar 123`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "bar".to_owned(), span: null_span},
    /// #         Some(Value::int(123, null_span))
    /// #     )],
    /// # };
    /// let foo = call.get_flag::<i64>("foo");
    /// assert_eq!(foo.unwrap(), None);
    /// ```
    ///
    /// Invoked as `my_command --foo abc`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: Vec::new(),
    /// #     named: vec![(
    /// #         Spanned { item: "foo".to_owned(), span: null_span},
    /// #         Some(Value::string("abc".to_owned(), null_span))
    /// #     )],
    /// # };
    /// let foo = call.get_flag::<i64>("foo");
    /// assert!(foo.is_err());
    /// ```
    pub fn get_flag<T: FromValue>(&self, name: &str) -> Result<Option<T>, ShellError> {
        if let Some(value) = self.get_flag_value(name) {
            FromValue::from_value(value).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Retrieve the Nth and all following positional arguments as type `T`
    ///
    /// # Example
    /// Invoked as `my_command zero one two three`:
    /// ```
    /// # use nu_protocol::{Spanned, Span, Value};
    /// # use nu_plugin_protocol::EvaluatedCall;
    /// # let null_span = Span::new(0, 0);
    /// # let call = EvaluatedCall {
    /// #     head: null_span,
    /// #     positional: vec![
    /// #         Value::string("zero".to_owned(), null_span),
    /// #         Value::string("one".to_owned(), null_span),
    /// #         Value::string("two".to_owned(), null_span),
    /// #         Value::string("three".to_owned(), null_span),
    /// #     ],
    /// #     named: Vec::new(),
    /// # };
    /// let args = call.rest::<String>(0);
    /// assert_eq!(args.unwrap(), vec!["zero", "one", "two", "three"]);
    ///
    /// let args = call.rest::<String>(2);
    /// assert_eq!(args.unwrap(), vec!["two", "three"]);
    /// ```
    pub fn rest<T: FromValue>(&self, starting_pos: usize) -> Result<Vec<T>, ShellError> {
        self.positional
            .iter()
            .skip(starting_pos)
            .map(|value| FromValue::from_value(value.clone()))
            .collect()
    }

    /// Retrieve the value of an optional positional argument interpreted as type `T`
    ///
    /// Returns the value of a (zero indexed) positional argument of type `T`.
    /// Alternatively returns [`None`] if the positional argument does not exist
    /// or an error that can be passed back to the shell on error.
    pub fn opt<T: FromValue>(&self, pos: usize) -> Result<Option<T>, ShellError> {
        if let Some(value) = self.nth(pos) {
            FromValue::from_value(value).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Retrieve the value of a mandatory positional argument as type `T`
    ///
    /// Expect a positional argument of type `T` and return its value or, if the
    /// argument does not exist or is of the wrong type, return an error that can
    /// be passed back to the shell.
    pub fn req<T: FromValue>(&self, pos: usize) -> Result<T, ShellError> {
        if let Some(value) = self.nth(pos) {
            FromValue::from_value(value)
        } else if self.positional.is_empty() {
            Err(ShellError::AccessEmptyContent { span: self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.positional.len() - 1,
                span: self.head,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::{Span, Spanned, Value};

    #[test]
    fn call_to_value() {
        let call = EvaluatedCall {
            head: Span::new(0, 10),
            positional: vec![
                Value::float(1.0, Span::new(0, 10)),
                Value::string("something", Span::new(0, 10)),
            ],
            named: vec![
                (
                    Spanned {
                        item: "name".to_string(),
                        span: Span::new(0, 10),
                    },
                    Some(Value::float(1.0, Span::new(0, 10))),
                ),
                (
                    Spanned {
                        item: "flag".to_string(),
                        span: Span::new(0, 10),
                    },
                    None,
                ),
            ],
        };

        let name: Option<f64> = call.get_flag("name").unwrap();
        assert_eq!(name, Some(1.0));

        assert!(call.has_flag("flag").unwrap());

        let required: f64 = call.req(0).unwrap();
        assert!((required - 1.0).abs() < f64::EPSILON);

        let optional: Option<String> = call.opt(1).unwrap();
        assert_eq!(optional, Some("something".to_string()));

        let rest: Vec<String> = call.rest(1).unwrap();
        assert_eq!(rest, vec!["something".to_string()]);
    }
}
