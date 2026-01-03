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
            .switch(
                "short",
                "Short representation, only shows users not the groups for each user",
                Some('s'),
            )
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
        let short = call.has_flag(engine_state, stack, "short")?;
        Ok(users(short, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Show info about the system users",
                example: "sys users",
                result: Some(
                    vec![record! {
                        "name" => Value::string("root", Span::default()),
                        "id" => Value::int(0, Span::default()),
                        "groups" => vec![
                            record! {
                                "name" => Value::string("root", Span::default()),
                                "id" => Value::int(0, Span::default()),
                            }
                        ]
                        .into_value(Span::default())
                    }]
                    .into_value(Span::default()),
                ),
            },
            Example {
                description: "Show users without groups, significantly faster.",
                example: "sys users --short",
                result: Some(
                    vec![record! {
                        "name" => Value::string("root", Span::default()),
                        "id" => Value::int(0, Span::default()),
                    }]
                    .into_value(Span::default()),
                ),
            },
        ]
    }
}

fn users(short: bool, span: Span) -> Value {
    let users = Users::new_with_refreshed_list()
        .iter()
        .map(|user| {
            #[allow(unused_mut)]
            let mut record = record! {
                "name" => Value::string(trim_cstyle_null(user.name()), span),
            };
            #[cfg(unix)]
            {
                use num_traits::ToPrimitive;
                record.insert("id", Value::int(user.id().to_i64().unwrap_or(-1), span));
            }
            if !short {
                let groups: Vec<Value> = user
                    .groups()
                    .iter()
                    .map(|group| {
                        #[allow(unused_mut)]
                        let mut rec = record! {
                            "name" => Value::string(trim_cstyle_null(group.name()), span),
                        };
                        #[cfg(unix)]
                        {
                            use num_traits::ToPrimitive;
                            rec.insert("id", Value::int(group.id().to_i64().unwrap_or(-1), span));
                        }
                        Value::record(rec, span)
                    })
                    .collect();
                record.insert("groups", groups.into_value(span));
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(users, span)
}
