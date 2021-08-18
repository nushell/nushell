use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::did_you_mean;
use nu_protocol::TaggedDictBuilder;
use nu_protocol::{
    hir::CapturedBlock, hir::ExternalRedirection, ColumnPath, PathMember, Signature, SyntaxShape,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_value_ext::get_data_by_column_path;

use nu_source::HasFallibleSpan;
pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "zip"
    }

    fn signature(&self) -> Signature {
        Signature::build("zip").required(
            "block",
            SyntaxShape::Block,
            "the block to run and zip into the table",
        )
    }

    fn usage(&self) -> &str {
        "Zip two tables."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Zip two lists",
            example: "[0 2 4 6 8] | zip { [1 3 5 7 9] } | each { $it }",
            result: None,
        },
        Example {
            description: "Zip two tables",
            example: "[[symbol]; ['('] ['['] ['{']] | zip { [[symbol]; [')'] [']'] ['}']] } | each { get symbol | $'($in.0)nushell($in.1)' }",
            result: Some(vec![
                Value::from("(nushell)"),
                Value::from("[nushell]"),
                Value::from("{nushell}")
            ])
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let context = &args.context;
    let name_tag = args.call_info.name_tag.clone();

    let block: CapturedBlock = args.req(0)?;
    let block_span = &block.block.span.clone();
    let input = args.input;

    context.scope.enter_scope();
    context.scope.add_vars(&block.captured.entries);
    let result = run_block(
        &block.block,
        context,
        InputStream::empty(),
        ExternalRedirection::Stdout,
    );
    context.scope.exit_scope();

    Ok(OutputStream::from_stream(zip(
        input,
        result,
        name_tag,
        *block_span,
    )?))
}

fn zip<'a>(
    l: impl Iterator<Item = Value> + 'a + Sync + Send,
    r: Result<InputStream, ShellError>,
    command_tag: Tag,
    secondary_command_span: Span,
) -> Result<Box<dyn Iterator<Item = Value> + 'a + Sync + Send>, ShellError> {
    Ok(Box::new(l.zip(r?).map(move |(s1, s2)| match (s1, s2) {
        (
            left_row
            @
            Value {
                value: UntaggedValue::Row(_),
                ..
            },
            mut
            right_row
            @
            Value {
                value: UntaggedValue::Row(_),
                ..
            },
        ) => {
            let mut zipped_row = TaggedDictBuilder::new(left_row.tag());

            right_row.tag = Tag::new(right_row.tag.anchor(), secondary_command_span);

            for column in left_row.data_descriptors() {
                let path = ColumnPath::build(&(column.to_string()).spanned(right_row.tag.span));
                zipped_row.insert_value(column, zip_row(&path, &left_row, &right_row));
            }

            zipped_row.into_value()
        }
        (s1, s2) => {
            let mut name_tag = command_tag.clone();
            name_tag.anchor = s1.tag.anchor();
            UntaggedValue::table(&vec![s1, s2]).into_value(&name_tag)
        }
    })))
}

fn zip_row(path: &ColumnPath, left: &Value, right: &Value) -> UntaggedValue {
    UntaggedValue::table(&vec![
        get_column(path, left)
            .unwrap_or_else(|err| UntaggedValue::Error(err).into_untagged_value()),
        get_column(path, right)
            .unwrap_or_else(|err| UntaggedValue::Error(err).into_untagged_value()),
    ])
}

pub fn get_column(path: &ColumnPath, value: &Value) -> Result<Value, ShellError> {
    get_data_by_column_path(value, path, move |obj_source, column_path_tried, error| {
        let path_members_span = path.maybe_span().unwrap_or_else(Span::unknown);

        if obj_source.is_row() {
            if let Some(error) = error_message(column_path_tried, &path_members_span, obj_source) {
                return error;
            }
        }

        error
    })
}

fn error_message(
    column_tried: &PathMember,
    path_members_span: &Span,
    obj_source: &Value,
) -> Option<ShellError> {
    match column_tried {
        PathMember {
            unspanned: UnspannedPathMember::String(column),
            ..
        } => {
            let primary_label = format!("There isn't a column named '{}' from this table", &column);

            did_you_mean(obj_source, column_tried.as_string()).map(|suggestions| {
                ShellError::labeled_error_with_secondary(
                    "Unknown column",
                    primary_label,
                    obj_source.tag.span,
                    format!(
                        "Perhaps you meant '{}'? Columns available: {}",
                        suggestions[0],
                        &obj_source.data_descriptors().join(", ")
                    ),
                    column_tried.span.since(path_members_span),
                )
            })
        }
        _ => None,
    }
}
