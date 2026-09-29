use super::util::read_pickle;
use nu_engine::command_prelude::*;
use nu_parser::pickle;
use nu_protocol::{
    BlockId, DeclId, ModuleId, VarId, engine::StateWorkingSet, ir::Instruction, relocation::IdBases,
};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone)]
pub struct DebugPickle;

impl Command for DebugPickle {
    fn name(&self) -> &str {
        "debug pickle"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("pickle", SyntaxShape::Filepath, "The pickle to inspect.")
            .input_output_type(Type::Nothing, Type::record())
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Show what a pickle made by `pickle` contains and whether this nushell can run it."
    }

    fn extra_description(&self) -> &str {
        "`loads` says whether this nushell can run the pickle, and `problems` lists exactly what \
keeps it from loading. When it loads, the program is linked into a copy of the current engine \
and described: its files, commands, variables, blocks (with their IR, as `view ir` shows it), \
modules and the commands it uses from this nushell. The engine has this shell's experimental \
options, while `nu script.nupkl` uses the pickle's own. A pickle that calls a command only one of \
them registers can show a problem here and still run."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["nupkl", "inspect", "ir"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let (path, contents) = read_pickle(engine_state, stack, call)?;
        let info = pickle::info(&contents)?;

        let mut record = record! {
            "file" => Value::string(path.to_string_lossy(), head),
            "size" => Value::filesize(contents.len() as i64, head),
            "format" => Value::int(i64::from(info.format_version), head),
        };
        if let Some(header) = &info.header {
            record.push("version", Value::string(&header.nu_version, head));
            record.push("commit", Value::string(&header.build, head));
            let options: Vec<Record> = header
                .experimental_options
                .iter()
                .map(|(name, enabled)| {
                    record! {
                        "name" => Value::string(name, head),
                        "enabled" => Value::bool(*enabled, head),
                    }
                })
                .collect();
            record.push("experimental_options", options.into_value(head));
        }

        let mut problems = info.problems;
        if problems.is_empty() {
            match describe_program(engine_state, &path, &contents, head) {
                Ok((program, errors)) => {
                    problems.extend(errors);
                    record.extend(program);
                }
                Err(err) => problems.push(err.to_string()),
            }
        }
        record.insert("loads", Value::bool(problems.is_empty(), head));
        record.insert("problems", problems.into_value(head));
        Ok(Value::record(record, head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "See whether this nushell can run a pickle, and why not.",
                example: "debug pickle script.nupkl | select loads problems",
                result: None,
            },
            Example {
                description: "Show the IR of the pickle's custom commands.",
                example: "debug pickle script.nupkl | get blocks | where name != '' | select name ir",
                result: None,
            },
        ]
    }
}

/// Link the program of a pickle into a copy of `engine_state` and describe it as tables. Also
/// returns the errors of computing its parse-time values again.
fn describe_program(
    engine_state: &EngineState,
    path: &Path,
    contents: &[u8],
    head: Span,
) -> Result<(Record, Vec<String>), ShellError> {
    let mut scratch = engine_state.clone();
    let bases = IdBases::of(&scratch);
    let mut working_set = StateWorkingSet::new(&scratch);
    let dir = path.parent().unwrap_or(Path::new(""));
    let main = pickle::load(&mut working_set, contents, dir)?;
    let errors = working_set
        .parse_errors
        .iter()
        .map(|err| err.to_string())
        .collect();
    let delta = working_set.render();
    scratch.merge_delta(delta)?;
    let scratch = &scratch;

    let files: Vec<Record> = scratch
        .files()
        .skip(bases.files)
        .map(|file| {
            record! {
                "name" => Value::string(&*file.name, head),
                "size" => Value::filesize(file.content.len() as i64, head),
            }
        })
        .collect();

    let commands: Vec<Record> = (bases.decls..scratch.num_decls())
        .map(DeclId::new)
        .map(|decl_id| {
            let decl = scratch.get_decl(decl_id);
            record! {
                "id" => Value::int(decl_id.get() as i64, head),
                "name" => Value::string(decl.name(), head),
                "type" => Value::string(decl.command_type().to_string(), head),
                "description" => Value::string(decl.description(), head),
                "block" => decl.block_id().map_or(Value::nothing(head), |block_id| {
                    Value::int(block_id.get() as i64, head)
                }),
            }
        })
        .collect();

    let variables: Vec<Record> = (bases.vars..scratch.num_vars())
        .map(VarId::new)
        .map(|var_id| {
            let variable = scratch.get_var(var_id);
            record! {
                "id" => Value::int(var_id.get() as i64, head),
                "name" => variable.name.as_deref().map_or(Value::nothing(head), |name| {
                    Value::string(String::from_utf8_lossy(name), head)
                }),
                "type" => Value::string(variable.ty.to_string(), head),
                "mutable" => Value::bool(variable.mutable, head),
                "const" => variable.const_val.clone().unwrap_or(Value::nothing(head)),
            }
        })
        .collect();

    let mut used = BTreeSet::new();
    let blocks: Vec<Record> = (bases.blocks..scratch.num_blocks())
        .map(|id| {
            (
                Some(BlockId::new(id)),
                scratch.get_block(BlockId::new(id)).as_ref(),
            )
        })
        .chain([(None, main.as_ref())])
        .map(|(block_id, block)| {
            if let Some(ir_block) = &block.ir_block {
                used.extend(ir_block.instructions.iter().filter_map(
                    |instruction| match instruction {
                        Instruction::Call { decl_id, .. } if decl_id.get() < bases.decls => {
                            Some(scratch.get_decl(*decl_id).name().to_string())
                        }
                        _ => None,
                    },
                ));
            }
            let ir = block.ir_block.as_ref();
            record! {
                "id" => block_id.map_or(Value::nothing(head), |id| {
                    Value::int(id.get() as i64, head)
                }),
                "name" => Value::string(
                    if block_id.is_none() { "top level" } else { &block.signature.name },
                    head,
                ),
                "captures" => Value::int(block.captures.len() as i64, head),
                "registers" => ir.map_or(Value::nothing(head), |ir| {
                    Value::int(i64::from(ir.register_count), head)
                }),
                "instructions" => ir.map_or(Value::nothing(head), |ir| {
                    Value::int(ir.instructions.len() as i64, head)
                }),
                "ir" => ir.map_or(Value::nothing(head), |ir| {
                    Value::string(ir.display(scratch).to_string(), head)
                }),
            }
        })
        .collect();

    let modules: Vec<Record> = (bases.modules..scratch.num_modules())
        .map(ModuleId::new)
        .map(|module_id| {
            let module = scratch.get_module(module_id);
            let names = |names: Vec<Vec<u8>>| {
                Vec::from_iter(
                    names
                        .iter()
                        .map(|name| String::from_utf8_lossy(name).into_owned()),
                )
                .into_value(head)
            };
            record! {
                "id" => Value::int(module_id.get() as i64, head),
                "name" => Value::string(String::from_utf8_lossy(&module.name), head),
                "file" => module.real_file().map_or(Value::nothing(head), |file| {
                    Value::string(file.to_string_lossy(), head)
                }),
                "commands" => names(module.decls.keys().cloned().collect()),
                "constants" => names(module.constants.keys().cloned().collect()),
            }
        })
        .collect();

    let program = record! {
        "files" => files.into_value(head),
        "commands" => commands.into_value(head),
        "variables" => variables.into_value(head),
        "blocks" => blocks.into_value(head),
        "modules" => modules.into_value(head),
        "uses" => Vec::from_iter(used).into_value(head),
    };
    Ok((program, errors))
}
