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
    }

    fn description(&self) -> &str {
        "View information about the users on the system."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(users(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show info about the system users",
            example: "sys users",
            result: None,
        }]
    }
}

fn users(span: Span) -> Value {
    let users = Users::new_with_refreshed_list()
        .iter()
        .map(|user| {
            let groups = user
                .groups()
                .iter()
                .map(|group| Value::string(trim_cstyle_null(group.name()), span))
                .collect();

            // On Windows, Uid wraps a SID (Security Identifier) which is a string like "S-1-5-18"
            // On Unix-like systems (macOS, Linux, BSD), Uid wraps a numeric u32 user ID
            // We need conditional compilation to handle these platform differences
            #[cfg(windows)]
            let id_value = Value::string(user.id().to_string(), span);
            #[cfg(not(windows))]
            let id_value = {
                let id_ref: &u32 = user.id();
                Value::int((*id_ref) as i64, span)
            };

            let record = record! {
                "id" => id_value,
                "name" => Value::string(trim_cstyle_null(user.name()), span),
                "groups" => Value::list(groups, span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(users, span)
}
