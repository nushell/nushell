use super::trim_cstyle_null;
use nu_engine::command_prelude::*;
use sysinfo::Disks;

#[derive(Clone)]
pub struct SysDisks;

impl Command for SysDisks {
    fn name(&self) -> &str {
        "sys disks"
    }

    fn signature(&self) -> Signature {
        Signature::build("sys disks")
            .filter()
            .category(Category::System)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn description(&self) -> &str {
        "View information about the system disks."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(disks(call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Show info about the system disks",
            example: "sys disks",
            result: None,
        }]
    }
}

fn disks(span: Span) -> Value {
    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| {
            let device = trim_cstyle_null(disk.name().to_string_lossy());
            let typ = trim_cstyle_null(disk.file_system().to_string_lossy());

            let record = record! {
                "device" => Value::string(device, span),
                "type" => Value::string(typ, span),
                "mount" => Value::string(disk.mount_point().to_string_lossy(), span),
                "total" => Value::filesize(disk.total_space() as i64, span),
                "free" => Value::filesize(disk.available_space() as i64, span),
                "removable" => Value::bool(disk.is_removable(), span),
                "kind" => Value::string(disk.kind().to_string(), span),
            };

            Value::record(record, span)
        })
        .collect();

    Value::list(disks, span)
}
