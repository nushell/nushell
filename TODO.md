This pattern is extremely repetitive and can be abstracted:

```rs
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            latest_tag = Some(value_tag.clone());
            let value_span = value.tag.span;

            match &value.value {
                UntaggedValue::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                    concat_string.push_str("\n");
                }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                )),

            }
        }

```

Mandatory and Optional in parse_command

trace_remaining?

select_fields and select_fields take unnecessary Tag

Value#value should be Value#untagged

Unify dictionary building, probably around a macro

sys plugin in own crate

textview in own crate

Combine atomic and atomic_parse in parser

at_end_possible_ws needs to be comment and separator sensitive
