use super::trim_cstyle_null;
use nu_engine::command_prelude::*;
use sysinfo::Users;

#[derive(Clone)]
pub struct SysUsers;

impl Command for SysUsers {
    fn name(&self) -> &str {
        "sys users"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys users")
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .switch("nogroup", "Don't compute groups", Some('n'))
    }

    fn description(&self) -> &str {
        "View information about the users on the system."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let nogroup = call.has_flag(engine_state, stack, "nogroup")?;
        Ok(users(nogroup, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show info about the system users",
            example: "sys users",
            result: None,
        }]
    }
}

fn users(nogroup: bool, span: Span) -> Value {
    let users = Users::new_with_refreshed_list()
        .iter()
        .map(|user| {
            let groups = if !nogroup {
                user.groups()
                    .iter()
                    .map(|group| {
                        Value::record(
                            record! {
                                "name" => Value::string(trim_cstyle_null(group.name()), span),
                                "id" => Value::int(group.id().to_le() as i64, span),
                            },
                            span,
                        )
                    })
                    .collect()
            } else {
                vec![]
            };

            let record = record! {
                "name" => Value::string(trim_cstyle_null(user.name()), span),
                "id" => Value::int(user.id().to_le() as i64, span),
                "groups" => Value::list(groups, span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(users, span)
}
