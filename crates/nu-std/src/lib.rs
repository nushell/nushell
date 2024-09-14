#![doc = include_str!("../README.md")]
use log::trace;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{FileStack, Stack, StateWorkingSet, VirtualPath},
    report_parse_error, PipelineData,
};
use std::path::PathBuf;

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    trace!("load_standard_library");
    let (block, delta) = {
        let std_dir = PathBuf::from("std");

        let mut std_files = vec![
            ("mod.nu", include_str!("../std/mod.nu")),
            ("core", include_str!("../std/core/mod.nu")),
            ("assert", include_str!("../std/assert/mod.nu")),
            ("bench", include_str!("../std/bench/mod.nu")),
            ("dirs", include_str!("../std/dirs/mod.nu")),
            ("dt", include_str!("../std/dt/mod.nu")),
            ("formats", include_str!("../std/formats/mod.nu")),
            ("help", include_str!("../std/help/mod.nu")),
            ("input", include_str!("../std/input/mod.nu")),
            ("iter", include_str!("../std/iter/mod.nu")),
            ("log", include_str!("../std/log/mod.nu")),
            ("math", include_str!("../std/math/mod.nu")),
            ("util", include_str!("../std/util/mod.nu")),
            ("xml", include_str!("../std/xml/mod.nu")),
        ];

        let mut working_set = StateWorkingSet::new(engine_state);
        let mut std_virt_paths = vec![];

        for (name, content) in std_files.drain(..) {
            let name = std_dir.join(name);

            let file_id =
                working_set.add_file(name.to_string_lossy().to_string(), content.as_bytes());
            let virtual_file_id = working_set.add_virtual_path(
                name.to_string_lossy().to_string(),
                VirtualPath::File(file_id),
            );
            std_virt_paths.push(virtual_file_id);
        }

        let std_dir = std_dir.to_string_lossy().to_string();
        let source = r#"
# Prelude
use ([ std core ] | path join) *
"#;

        let _ = working_set.add_virtual_path(std_dir, VirtualPath::Dir(std_virt_paths));

        // Add a placeholder file to the stack of files being evaluated.
        // The name of this file doesn't matter; it's only there to set the current working directory to NU_STDLIB_VIRTUAL_DIR.
        let placeholder = PathBuf::from("load std/core");
        working_set.files = FileStack::with_file(placeholder);

        let block = parse(
            &mut working_set,
            Some("loading stdlib"),
            source.as_bytes(),
            false,
        );

        // Remove the placeholder file from the stack of files being evaluated.
        working_set.files.pop();

        if let Some(err) = working_set.parse_errors.first() {
            report_parse_error(&working_set, err);
        }

        (block, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    // We need to evaluate the module in order to run the `export-env` blocks.
    let mut stack = Stack::new();
    let pipeline_data = PipelineData::Empty;

    eval_block::<WithoutDebug>(engine_state, &mut stack, &block, pipeline_data)?;

    let cwd = engine_state.cwd(Some(&stack))?;
    engine_state.merge_env(&mut stack, cwd)?;

    Ok(())
}
