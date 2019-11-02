use itertools::Itertools;
use nu::{
    serve_plugin, CallInfo, Plugin, ReturnSuccess, ReturnValue, ShellError, Signature, SyntaxShape,
    Tagged, TaggedItem, Value,
};

pub type ColumnPath = Vec<Tagged<String>>;

struct Insert {
    field: Option<ColumnPath>,
    value: Option<Value>,
}
impl Insert {
    fn new() -> Insert {
        Insert {
            field: None,
            value: None,
        }
    }

    fn insert(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        let value_tag = value.tag();
        match (value.item, self.value.clone()) {
            (obj @ Value::Row(_), Some(v)) => match &self.field {
                Some(f) => match obj.insert_data_at_column_path(value_tag.clone(), &f, v) {
                    Some(v) => return Ok(v),
                    None => {
                        return Err(ShellError::labeled_error(
                            format!(
                                "add could not find place to insert field {:?} {}",
                                obj,
                                f.iter().map(|i| &i.item).join(".")
                            ),
                            "column name",
                            &value_tag,
                        ))
                    }
                },
                None => Err(ShellError::labeled_error(
                    "add needs a column name when adding a value to a table",
                    "column name",
                    value_tag,
                )),
            },
            (value, _) => Err(ShellError::type_error(
                "row",
                value.type_name().tagged(value_tag),
            )),
        }
    }
}

impl Plugin for Insert {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("insert")
            .desc("Insert a new column to the table.")
            .required(
                "column",
                SyntaxShape::ColumnPath,
                "the column name to insert",
            )
            .required(
                "value",
                SyntaxShape::String,
                "the value to give the cell(s)",
            )
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                table @ Tagged {
                    item: Value::Table(_),
                    ..
                } => {
                    self.field = Some(table.as_column_path()?.item);
                }

                value => return Err(ShellError::type_error("table", value.tagged_type_name())),
            }
            match &args[1] {
                Tagged { item: v, .. } => {
                    self.value = Some(v.clone());
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.insert(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Insert::new());
}
