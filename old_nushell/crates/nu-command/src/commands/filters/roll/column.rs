use crate::prelude::*;
use nu_data::base::select_fields;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tagged;

use super::support::{rotate, Direction};

pub struct SubCommand;

pub struct Arguments {
    by: Option<Tagged<u64>>,
    opposite: bool,
    cells_only: bool,
}

impl Arguments {
    fn direction(&self) -> Direction {
        if self.opposite {
            return Direction::Left;
        }

        Direction::Right
    }

    fn move_headers(&self) -> bool {
        !self.cells_only
    }
}

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "roll column"
    }

    fn signature(&self) -> Signature {
        Signature::build("roll column")
            .optional("by", SyntaxShape::Int, "the number of times to roll")
            .switch("opposite", "roll in the opposite direction", Some('o'))
            .switch("cells-only", "only roll the cells", Some('c'))
    }

    fn usage(&self) -> &str {
        "Rolls the table columns"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        roll(args)
    }
}

pub fn roll(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let options = Arguments {
        by: args.opt(0)?,
        opposite: args.has_flag("opposite"),
        cells_only: args.has_flag("cells-only"),
    };

    Ok(args
        .input
        .flat_map(move |value| {
            let tag = value.tag();

            roll_by(value, &options)
                .unwrap_or_else(|| vec![UntaggedValue::nothing().into_value(tag)])
        })
        .into_output_stream())
}

fn roll_by(value: Value, options: &Arguments) -> Option<Vec<Value>> {
    let tag = value.tag();
    let direction = options.direction();

    if value.is_row() {
        let columns = value.data_descriptors();
        if options.move_headers() {
            if let Some(fields) = rotate(columns, &options.by, direction) {
                return Some(vec![select_fields(&value, &fields, &tag)]);
            }
        } else {
            let values_rotated = rotate(
                value
                    .row_entries()
                    .map(|(_, value)| value.clone())
                    .collect::<Vec<_>>(),
                &options.by,
                direction,
            );

            if let Some(ref values) = values_rotated {
                let mut out = TaggedDictBuilder::new(&tag);

                for (k, v) in columns.iter().zip(values) {
                    out.insert_value(k, v.clone());
                }

                return Some(vec![out.into_value()]);
            }
        }
        None
    } else if value.is_table() {
        rotate(
            value.table_entries().cloned().collect(),
            &options.by,
            direction,
        )
    } else {
        Some(vec![value])
    }
}
