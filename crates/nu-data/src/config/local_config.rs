use nu_errors::ShellError;
use std::path::{Path, PathBuf};

static LOCAL_CFG_FILE_NAME: &str = ".nu-env";

pub struct LocalConfigDiff {
    pub cfgs_to_load: Vec<PathBuf>,
    pub cfgs_to_unload: Vec<PathBuf>,
}

/// Finds all local configs between `from` up to `to`.
/// Every config seen while going up the filesystem (e.G. from `/foo` to `/foo/bar`) is returned
/// as a config to load
/// Every config seen while going down the filesystem (e.G. from `/foo/bar` to `/foo/bar`) is
/// returned as a config to unload
/// If both paths are unrelated to each other, (e.G. windows paths as: `C:/foo` and `D:/bar`)
/// this function first walks `from` completely down the filesystem and then it walks up until `to`.
///
/// Both paths are required to be absolute.
impl LocalConfigDiff {
    pub fn between(from: PathBuf, to: PathBuf) -> (LocalConfigDiff, Vec<ShellError>) {
        let common_prefix = common_path::common_path(&from, &to);
        let (cfgs_to_unload, err_down) = walk_down(&from, &common_prefix);
        let (cfgs_to_load, err_up) = walk_up(&common_prefix, &to);

        (
            LocalConfigDiff {
                cfgs_to_load,
                cfgs_to_unload,
            },
            err_down.into_iter().chain(err_up).collect(),
        )
    }
}

///Walks from the first parameter down the filesystem to the second parameter. Marking all
///configs found in directories on the way as to remove.
///If to is None, this method walks from the first parameter down to the beginning of the
///filesystem
///Returns tuple of (configs to remove, errors from io).
fn walk_down(
    from_inclusive: &Path,
    to_exclusive: &Option<PathBuf>,
) -> (Vec<PathBuf>, Vec<ShellError>) {
    let mut all_err = vec![];
    let mut all_cfgs_to_unload = vec![];
    for dir in from_inclusive.ancestors().take_while(|cur_path| {
        if let Some(until_path) = to_exclusive {
            //Stop before `to_exclusive`
            *cur_path != until_path
        } else {
            //No end, walk all the way down
            true
        }
    }) {
        match local_cfg_should_be_unloaded(dir.to_path_buf()) {
            Ok(Some(cfg)) => all_cfgs_to_unload.push(cfg),
            Err(e) => all_err.push(e),
            _ => {}
        }
    }

    (all_cfgs_to_unload, all_err)
}

///Walks from the first parameter up the filesystem to the second parameter, returns all configs
///found in directories on the way to load.
///Returns combined errors from checking directories on the way
///If from is None, this method walks from the beginning of the second parameter up to the
///second parameter
fn walk_up(
    from_exclusive: &Option<PathBuf>,
    to_inclusive: &Path,
) -> (Vec<PathBuf>, Vec<ShellError>) {
    let mut all_err = vec![];
    let mut all_cfgs_to_load = vec![];

    //skip all paths until (inclusive) from (or 0 if from is None)
    let skip_ahead = from_exclusive
        .as_ref()
        .map(|p| p.ancestors().count())
        .unwrap_or(0);
    //We have to traverse ancestors in reverse order (apply lower directories first)
    //ancestors() does not yield iter with .rev() method. So we store all ancestors
    //and then iterate over the vec
    let dirs: Vec<_> = to_inclusive.ancestors().map(Path::to_path_buf).collect();
    for dir in dirs.iter().rev().skip(skip_ahead) {
        match loadable_cfg_exists_in_dir(dir.clone()) {
            Ok(Some(cfg)) => all_cfgs_to_load.push(cfg),
            Err(e) => all_err.push(e),
            _ => {}
        }
    }

    (all_cfgs_to_load, all_err)
}

fn is_existent_local_cfg(cfg_file_path: &Path) -> Result<bool, ShellError> {
    if !cfg_file_path.exists() || cfg_file_path.parent() == super::default_path()?.parent() {
        //Don't treat global cfg as local one
        Ok(false)
    } else {
        Ok(true)
    }
}

fn is_trusted_local_cfg_content(cfg_file_path: &Path, content: &[u8]) -> Result<bool, ShellError> {
    //This checks whether user used `autoenv trust` to mark this cfg as secure
    if !super::is_file_trusted(&cfg_file_path, &content)? {
        //Notify user about present config, but not trusted
        Err(ShellError::untagged_runtime_error(
                format!("{:?} is untrusted. Run 'autoenv trust {:?}' to trust it.\nThis needs to be done after each change to the file.",
                    cfg_file_path, cfg_file_path.parent().unwrap_or_else(|| &Path::new("")))))
    } else {
        Ok(true)
    }
}

fn local_cfg_should_be_unloaded<P: AsRef<Path>>(cfg_dir: P) -> Result<Option<PathBuf>, ShellError> {
    let mut cfg = cfg_dir.as_ref().to_path_buf();
    cfg.push(LOCAL_CFG_FILE_NAME);
    if is_existent_local_cfg(&cfg)? {
        //No need to compute whether content is good. If it is not loaded before, unloading does
        //nothing
        Ok(Some(cfg))
    } else {
        Ok(None)
    }
}

/// Checks whether a local_cfg exists in cfg_dir and returns:
/// Ok(Some(cfg_path)) if cfg exists and is good to load
/// Ok(None) if no cfg exists
/// Err(error) if cfg exits, but is not good to load
pub fn loadable_cfg_exists_in_dir(mut cfg_dir: PathBuf) -> Result<Option<PathBuf>, ShellError> {
    cfg_dir.push(LOCAL_CFG_FILE_NAME);
    let cfg_path = cfg_dir;

    if !is_existent_local_cfg(&cfg_path)? {
        return Ok(None);
    }

    let content = std::fs::read(&cfg_path)?;

    if !is_trusted_local_cfg_content(&cfg_path, &content)? {
        return Ok(None);
    }

    Ok(Some(cfg_path))
}
