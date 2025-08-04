#![doc = include_str!("../README.md")]
use log::trace;
use nu_parser::parse;
use nu_protocol::{
    VirtualPathId,
    engine::{FileStack, StateWorkingSet, VirtualPath},
    report_parse_error,
};
use std::path::PathBuf;

fn create_virt_file(working_set: &mut StateWorkingSet, name: &str, content: &str) -> VirtualPathId {
    let sanitized_name = PathBuf::from(name).to_string_lossy().to_string();
    let file_id = working_set.add_file(sanitized_name.clone(), content.as_bytes());

    working_set.add_virtual_path(sanitized_name, VirtualPath::File(file_id))
}

pub fn load_standard_library(
    engine_state: &mut nu_protocol::engine::EngineState,
) -> Result<(), miette::ErrReport> {
    trace!("load_standard_library");

    let mut working_set = StateWorkingSet::new(engine_state);

    // Contents of the std virtual directory
    let mut std_virt_paths = vec![];

    // std/mod.nu
    let std_mod_virt_file_id = create_virt_file(
        &mut working_set,
        "std/mod.nu",
        include_str!("../std/mod.nu"),
    );
    std_virt_paths.push(std_mod_virt_file_id);

    // Submodules/subdirectories ... std/<module>/mod.nu
    let mut std_submodules = vec![
        // Loaded at startup - Not technically part of std
        (
            "mod.nu",
            "std/prelude",
            include_str!("../std/prelude/mod.nu"),
        ),
        // std submodules
        ("mod.nu", "std/assert", include_str!("../std/assert/mod.nu")),
        ("mod.nu", "std/bench", include_str!("../std/bench/mod.nu")),
        ("mod.nu", "std/dirs", include_str!("../std/dirs/mod.nu")),
        ("mod.nu", "std/dt", include_str!("../std/dt/mod.nu")),
        (
            "mod.nu",
            "std/formats",
            include_str!("../std/formats/mod.nu"),
        ),
        ("mod.nu", "std/help", include_str!("../std/help/mod.nu")),
        ("mod.nu", "std/input", include_str!("../std/input/mod.nu")),
        ("mod.nu", "std/iter", include_str!("../std/iter/mod.nu")),
        ("mod.nu", "std/log", include_str!("../std/log/mod.nu")),
        ("mod.nu", "std/math", include_str!("../std/math/mod.nu")),
        ("mod.nu", "std/util", include_str!("../std/util/mod.nu")),
        ("mod.nu", "std/xml", include_str!("../std/xml/mod.nu")),
        ("mod.nu", "std/config", include_str!("../std/config/mod.nu")),
        (
            "mod.nu",
            "std/testing",
            include_str!("../std/testing/mod.nu"),
        ),
        ("mod.nu", "std/clip", include_str!("../std/clip/mod.nu")),
    ];

    for (filename, std_subdir_name, content) in std_submodules.drain(..) {
        let mod_dir = PathBuf::from(std_subdir_name);
        let name = mod_dir.join(filename);
        let virt_file_id = create_virt_file(&mut working_set, &name.to_string_lossy(), content);

        // Place file in virtual subdir
        let mod_dir_filelist = vec![virt_file_id];

        let virt_dir_id = working_set.add_virtual_path(
            mod_dir.to_string_lossy().to_string(),
            VirtualPath::Dir(mod_dir_filelist),
        );
        // Add the subdir to the list of paths in std
        std_virt_paths.push(virt_dir_id);
    }

    // Create std virtual dir with all subdirs and files
    let std_dir = PathBuf::from("std").to_string_lossy().to_string();
    let _ = working_set.add_virtual_path(std_dir, VirtualPath::Dir(std_virt_paths));

    // Add std-rfc files
    let mut std_rfc_virt_paths = vec![];

    // std-rfc/mod.nu
    let std_rfc_mod_virt_file_id = create_virt_file(
        &mut working_set,
        "std-rfc/mod.nu",
        include_str!("../std-rfc/mod.nu"),
    );
    std_rfc_virt_paths.push(std_rfc_mod_virt_file_id);

    // Submodules/subdirectories ... std-rfc/<module>/mod.nu
    let mut std_rfc_submodules = vec![
        (
            "mod.nu",
            "std-rfc/clip",
            include_str!("../std-rfc/clip/mod.nu"),
        ),
        (
            "mod.nu",
            "std-rfc/conversions",
            include_str!("../std-rfc/conversions/mod.nu"),
        ),
        #[cfg(feature = "sqlite")]
        ("mod.nu", "std-rfc/kv", include_str!("../std-rfc/kv/mod.nu")),
        (
            "mod.nu",
            "std-rfc/path",
            include_str!("../std-rfc/path/mod.nu"),
        ),
        (
            "mod.nu",
            "std-rfc/str",
            include_str!("../std-rfc/str/mod.nu"),
        ),
        (
            "mod.nu",
            "std-rfc/tables",
            include_str!("../std-rfc/tables/mod.nu"),
        ),
        (
            "mod.nu",
            "std-rfc/iter",
            include_str!("../std-rfc/iter/mod.nu"),
        ),
        (
            "mod.nu",
            "std-rfc/random",
            include_str!("../std-rfc/random/mod.nu"),
        ),
    ];

    for (filename, std_rfc_subdir_name, content) in std_rfc_submodules.drain(..) {
        let mod_dir = PathBuf::from(std_rfc_subdir_name);
        let name = mod_dir.join(filename);
        let virt_file_id = create_virt_file(&mut working_set, &name.to_string_lossy(), content);

        // Place file in virtual subdir
        let mod_dir_filelist = vec![virt_file_id];

        let virt_dir_id = working_set.add_virtual_path(
            mod_dir.to_string_lossy().to_string(),
            VirtualPath::Dir(mod_dir_filelist),
        );
        // Add the subdir to the list of paths in std
        std_rfc_virt_paths.push(virt_dir_id);
    }

    // Create std virtual dir with all subdirs and files
    let std_rfc_dir = PathBuf::from("std-rfc").to_string_lossy().to_string();
    let _ = working_set.add_virtual_path(std_rfc_dir, VirtualPath::Dir(std_rfc_virt_paths));

    // Load prelude
    let (_, delta) = {
        let source = r#"
# Prelude
use std/prelude *
"#;

        // Add a placeholder file to the stack of files being evaluated.
        // The name of this file doesn't matter; it's only there to set the current working directory to NU_STDLIB_VIRTUAL_DIR.
        let placeholder = PathBuf::from("load std/prelude");
        working_set.files = FileStack::with_file(placeholder);

        let block = parse(
            &mut working_set,
            Some("loading stdlib prelude"),
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

    Ok(())
}
