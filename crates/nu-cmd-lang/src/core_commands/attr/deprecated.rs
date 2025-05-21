use nu_cmd_base::WrapCall;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct AttrDeprecated;

impl Command for AttrDeprecated {
    fn name(&self) -> &str {
        "attr deprecated"
    }

    fn signature(&self) -> Signature {
        Signature::build("attr deprecated")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::Nothing, Type::String),
            ])
            .optional(
                "message",
                SyntaxShape::String,
                "Help message to include with deprecation warning.",
            )
            .named(
                "flag",
                SyntaxShape::String,
                "Mark a flag as deprecated rather than the command",
                None,
            )
            .named(
                "since",
                SyntaxShape::String,
                "Denote a version when this item was deprecated",
                Some('s'),
            )
            .named(
                "remove",
                SyntaxShape::String,
                "Denote a version when this item will be removed",
                Some('r'),
            )
            .named(
                "report",
                SyntaxShape::String,
                "How to warn about this item. One of: first (default), every",
                None,
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Attribute for marking a command or flag as deprecated."
    }

    fn extra_description(&self) -> &str {
        "Mark a command (default) or flag/switch (--flag) as deprecated. By default, only the first usage will trigger a deprecation warning.

A help message can be included to provide more context for the deprecation, such as what to use as a replacement.

Also consider setting the category to deprecated with @category deprecated"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call = WrapCall::Eval(engine_state, stack, call);
        Ok(deprecated_record(call)?.into_pipeline_data())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call = WrapCall::ConstEval(working_set, call);
        Ok(deprecated_record(call)?.into_pipeline_data())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Add a deprecation warning to a custom command",
                example: r###"@deprecated
    def outdated [] {}"###,
                result: Some(Value::nothing(Span::test_data())),
            },
            Example {
                description: "Add a deprecation warning with a custom message",
                example: r###"@deprecated "Use my-new-command instead."
    @category deprecated
    def my-old-command [] {}"###,
                result: Some(Value::string(
                    "Use my-new-command instead.",
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn deprecated_record(call: WrapCall) -> Result<Value, ShellError> {
    let (call, message): (_, Option<Spanned<String>>) = call.opt(0)?;
    let (call, flag): (_, Option<Spanned<String>>) = call.get_flag("flag")?;
    let (call, since): (_, Option<Spanned<String>>) = call.get_flag("since")?;
    let (call, remove): (_, Option<Spanned<String>>) = call.get_flag("remove")?;
    let (call, report): (_, Option<Spanned<String>>) = call.get_flag("report")?;

    let mut record = Record::new();
    if let Some(message) = message {
        record.push("help", Value::string(message.item, message.span))
    }
    if let Some(flag) = flag {
        record.push("flag", Value::string(flag.item, flag.span))
    }
    if let Some(since) = since {
        record.push("since", Value::string(since.item, since.span))
    }
    if let Some(remove) = remove {
        record.push("expected_removal", Value::string(remove.item, remove.span))
    }

    let report = if let Some(Spanned { item, span }) = report {
        match item.as_str() {
            "every" => Value::string(item, span),
            "first" => Value::string(item, span),
            _ => {
                return Err(ShellError::IncorrectValue {
                    msg: "The report mode must be one of: every, first".into(),
                    val_span: span,
                    call_span: call.head(),
                });
            }
        }
    } else {
        Value::string("first", call.head())
    };
    record.push("report", report);

    Ok(Value::record(record, call.head()))
}
