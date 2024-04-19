use log::trace;
use nu_engine::{env::current_dir, eval_block};
use nu_parser::parse;
use nu_protocol::{
    debugger::WithoutDebug,
    engine::{FileStack, Stack, StateWorkingSet, VirtualPath},
    report_error, PipelineData,
};
use std::path::PathBuf;

// Virtual std directory unlikely to appear in user's file system
const NU_STDLIB_VIRTUAL_DIR: &str = "NU_STDLIB_VIRTUAL_DIR";

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    trace!("load_standard_library");
    let (block, delta) = {
        // Using full virtual path to avoid potential conflicts with user having 'std' directory
        // in their working directory.
        let std_dir = PathBuf::from(NU_STDLIB_VIRTUAL_DIR).join("std");

        let mut std_files = vec![
            ("mod.nu", include_str!("../std/mod.nu")),
            ("dirs.nu", include_str!("../std/dirs.nu")),
            ("dt.nu", include_str!("../std/dt.nu")),
            ("help.nu", include_str!("../std/help.nu")),
            ("iter.nu", include_str!("../std/iter.nu")),
            ("log.nu", include_str!("../std/log.nu")),
            ("assert.nu", include_str!("../std/assert.nu")),
            ("xml.nu", include_str!("../std/xml.nu")),
            ("input.nu", include_str!("../std/input.nu")),
            ("math.nu", include_str!("../std/math.nu")),
            ("formats.nu", include_str!("../std/formats.nu")),
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
# Define the `std` module
module std

# Prelude
use std dirs [
    enter
    shells
    g
    n
    p
    dexit
]
use std pwd
"#;

        let _ = working_set.add_virtual_path(std_dir, VirtualPath::Dir(std_virt_paths));

        // Add a placeholder file to the stack of files being evaluated.
        // The name of this file doesn't matter; it's only there to set the current working directory to NU_STDLIB_VIRTUAL_DIR.
        let placeholder = PathBuf::from(NU_STDLIB_VIRTUAL_DIR).join("loading stdlib");
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
            report_error(&working_set, err);
        }

        (block, working_set.render())
    };

    engine_state.merge_delta(delta)?;

    // We need to evaluate the module in order to run the `export-env` blocks.
    let mut stack = Stack::new();
    let pipeline_data = PipelineData::Empty;

    eval_block::<WithoutDebug>(engine_state, &mut stack, &block, pipeline_data)?;

    let cwd = current_dir(engine_state, &stack)?;
    engine_state.merge_env(&mut stack, cwd)?;

    Ok(())
}
