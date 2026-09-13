use filesize::file_real_size_fast;
use nu_protocol::{ShellError, Signals, Span, Value, record, shell_error::io::IoError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A glob pattern used for excluding entries in `du`, supporting both the legacy
/// `nu_glob` backend and the experimental `dc_glob` backend.
#[derive(Debug, Clone)]
pub enum ExcludeGlob {
    Legacy(nu_glob::Pattern),
    DcGlob(nu_glob::dc_glob::DcPattern),
}

impl ExcludeGlob {
    pub fn matches_path(&self, path: &Path) -> bool {
        match self {
            ExcludeGlob::Legacy(p) => p.matches_path(path),
            ExcludeGlob::DcGlob(p) => p.matches_path(path),
        }
    }
}

/// A device-inode pair, which identifies a file so that one reached through
/// several hard links is counted only once.
///
/// POSIX states the rule directly: "A file identity is uniquely determined by
/// the combination of st_dev and st_ino." The device number belongs in the pair
/// because file serial numbers are unique only within one filesystem, so a walk
/// spanning two mounts could otherwise treat two different files as one.
///
/// GNU `du` keeps the same structure, as gnulib's `di_set` — "set operations
/// for device-inode pairs stored in a space-efficient manner" (lib/di-set.c).
#[cfg(unix)]
pub type FileId = (u64, u64);

/// Windows' equivalent of the POSIX `(st_dev, st_ino)` pair.
#[cfg(windows)]
pub type FileId = (u32, u64);

/// Platforms with no notion of hard links never need to track anything.
#[cfg(not(any(unix, windows)))]
pub type FileId = ();

#[cfg(unix)]
fn platform_file_id(md: &std::fs::Metadata) -> Option<FileId> {
    use std::os::unix::fs::MetadataExt;

    Some((md.dev(), md.ino()))
}

#[cfg(windows)]
fn platform_file_id(md: &std::fs::Metadata) -> Option<FileId> {
    use std::os::windows::fs::MetadataExt;

    // These accessors return `Some` for metadata produced by `fs::metadata` and
    // `File::metadata`, and `None` only for `DirEntry::metadata`. `FileInfo::new`
    // uses the former, so no extra syscall or file handle is needed here.
    Some((md.volume_serial_number()?, md.file_index()?))
}

#[cfg(not(any(unix, windows)))]
fn platform_file_id(_md: &std::fs::Metadata) -> Option<FileId> {
    None
}

/// The device-inode pair to record for a file, or `None` if no record is to be
/// kept of it.
///
/// Every file is recorded, not only those with several links. POSIX does permit
/// the narrower rule — "an implementation can optimize by only tracking files
/// with a link count greater than one, since in that scenario, those are the
/// only files that could be encountered more than once" — but that holds only
/// while a single operand is walked on its own. Overlapping operands reach a
/// one-link file twice, and `du` with no arguments already expands to many
/// top-level paths, so the narrower rule would be wrong in the most ordinary
/// invocation of all. GNU `du` arrives at the same place from the other
/// direction, switching its own shortcut off whenever "there are multiple
/// arguments, or if following non-command-line symlinks, because in either case
/// a file with just one hard link might be seen more than once".
///
/// `None` therefore means only that the caller asked for `--count-links`, or
/// that the platform has no way to identify a file.
fn file_id(md: &std::fs::Metadata, count_links: bool) -> Option<FileId> {
    if count_links {
        None
    } else {
        platform_file_id(md)
    }
}

#[derive(Debug, Clone)]
pub struct DirBuilder {
    pub tag: Span,
    pub min: Option<u64>,
    pub deref: bool,
    pub exclude: Option<ExcludeGlob>,
    pub long: bool,
    /// Count each hard link of files with multiple links, as GNU
    /// `du --count-links` and BSD `du -l` do.
    pub count_links: bool,
}

impl DirBuilder {
    pub fn new(
        tag: Span,
        min: Option<u64>,
        deref: bool,
        exclude: Option<ExcludeGlob>,
        long: bool,
        count_links: bool,
    ) -> DirBuilder {
        DirBuilder {
            tag,
            min,
            deref,
            exclude,
            long,
            count_links,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirInfo {
    dirs: Vec<DirInfo>,
    files: Vec<FileInfo>,
    errors: Vec<ShellError>,
    size: u64,
    blocks: u64,
    path: PathBuf,
    tag: Span,
    long: bool,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    path: PathBuf,
    size: u64,
    blocks: Option<u64>,
    /// This file's device-inode pair, or `None` if no record is kept of it.
    /// See [`file_id`].
    id: Option<FileId>,
    tag: Span,
    long: bool,
}

impl FileInfo {
    pub fn new(
        path: impl Into<PathBuf>,
        deref: bool,
        tag: Span,
        long: bool,
        count_links: bool,
    ) -> Result<Self, ShellError> {
        let path = path.into();
        let m = if deref {
            std::fs::metadata(&path)
        } else {
            std::fs::symlink_metadata(&path)
        };

        match m {
            Ok(d) => {
                let block_size = file_real_size_fast(&path, &d).ok();

                Ok(FileInfo {
                    path,
                    blocks: block_size,
                    size: d.len(),
                    id: file_id(&d, count_links),
                    tag,
                    long,
                })
            }
            Err(e) => Err(IoError::new(e, tag, path).into()),
        }
    }

    /// Try to insert this file's device-inode pair into the set of pairs whose
    /// sizes have already been counted.
    ///
    /// Returns true if the pair was inserted, false if it was already there —
    /// the same contract as gnulib's `di_set_insert` and the `hash_ins` helper
    /// in GNU `du`. A file for which no record is kept always returns true.
    pub fn insert_into(&self, counted: &mut HashSet<FileId>) -> bool {
        self.id.is_none_or(|id| counted.insert(id))
    }
}

impl DirInfo {
    pub fn new(
        path: impl Into<PathBuf>,
        params: &DirBuilder,
        depth: Option<u64>,
        span: Span,
        signals: &Signals,
        counted: &mut HashSet<FileId>,
    ) -> Result<Self, ShellError> {
        let path = path.into();
        let from_io_error = IoError::factory(span, path.as_path());

        let mut s = Self {
            dirs: Vec::new(),
            errors: Vec::new(),
            files: Vec::new(),
            size: 0,
            blocks: 0,
            tag: params.tag,
            path: path.clone(),
            long: params.long,
        };

        match std::fs::metadata(&s.path) {
            Ok(d) => {
                s.size = d.len(); // dir entry size
                s.blocks = file_real_size_fast(&s.path, &d).ok().unwrap_or(0);
            }
            Err(e) => s = s.add_error(from_io_error(e).into()),
        };

        match std::fs::read_dir(&s.path) {
            Ok(d) => {
                for f in d {
                    signals.check(&span)?;

                    match f {
                        Ok(i) => match i.file_type() {
                            Ok(t) if t.is_dir() => {
                                s = s.add_dir(i.path(), depth, params, span, signals, counted)?
                            }
                            Ok(_t) => s = s.add_file(i.path(), params, counted),
                            Err(e) => s = s.add_error(from_io_error(e).into()),
                        },
                        Err(e) => s = s.add_error(from_io_error(e).into()),
                    }
                }
            }
            Err(e) => s = s.add_error(from_io_error(e).into()),
        }
        Ok(s)
    }

    fn add_dir(
        mut self,
        path: impl Into<PathBuf>,
        mut depth: Option<u64>,
        params: &DirBuilder,
        span: Span,
        signals: &Signals,
        counted: &mut HashSet<FileId>,
    ) -> Result<Self, ShellError> {
        if let Some(current) = depth {
            if let Some(new) = current.checked_sub(1) {
                depth = Some(new);
            } else {
                return Ok(self);
            }
        }

        let d = DirInfo::new(path, params, depth, span, signals, counted)?;
        self.size += d.size;
        self.blocks += d.blocks;
        self.dirs.push(d);
        Ok(self)
    }

    fn add_file(
        mut self,
        f: impl Into<PathBuf>,
        params: &DirBuilder,
        counted: &mut HashSet<FileId>,
    ) -> Self {
        let f = f.into();
        let include = params.exclude.as_ref().is_none_or(|x| !x.matches_path(&f));
        if include {
            match FileInfo::new(f, params.deref, self.tag, self.long, params.count_links) {
                Ok(file) => {
                    let inc = params.min.is_none_or(|s| file.size >= s);
                    // `&&` short-circuits, so a file held back by --min-size
                    // is not recorded, and a later link to it is still free to
                    // be counted.
                    if inc && file.insert_into(counted) {
                        self.size += file.size;
                        self.blocks += file.blocks.unwrap_or(0);
                        if params.long {
                            self.files.push(file);
                        }
                    }
                }
                Err(e) => self = self.add_error(e),
            }
        }
        self
    }

    fn add_error(mut self, e: ShellError) -> Self {
        self.errors.push(e);
        self
    }

    pub fn get_size(&self) -> u64 {
        self.size
    }
}

impl From<DirInfo> for Value {
    fn from(d: DirInfo) -> Self {
        // if !d.errors.is_empty() {
        //     let v = d
        //         .errors
        //         .into_iter()
        //         .map(move |e| Value::Error { error: e })
        //         .collect::<Vec<Value>>();

        //     cols.push("errors".into());
        //     vals.push(Value::List {
        //         vals: v,
        //         span: d.tag,
        //     })
        // }

        if d.long {
            Value::record(
                record! {
                    "path" => Value::string(d.path.display().to_string(), d.tag),
                    "apparent" => Value::filesize(d.size as i64, d.tag),
                    "physical" => Value::filesize(d.blocks as i64, d.tag),
                    "directories" => value_from_vec(d.dirs, d.tag),
                    "files" => value_from_vec(d.files, d.tag)
                },
                d.tag,
            )
        } else {
            Value::record(
                record! {
                    "path" => Value::string(d.path.display().to_string(), d.tag),
                    "apparent" => Value::filesize(d.size as i64, d.tag),
                    "physical" => Value::filesize(d.blocks as i64, d.tag),
                },
                d.tag,
            )
        }
    }
}

impl From<FileInfo> for Value {
    fn from(f: FileInfo) -> Self {
        // cols.push("errors".into());
        // vals.push(Value::nothing(Span::unknown()));

        if f.long {
            Value::record(
                record! {
                    "path" => Value::string(f.path.display().to_string(), f.tag),
                    "apparent" => Value::filesize(f.size as i64, f.tag),
                    "physical" => Value::filesize(f.blocks.unwrap_or(0) as i64, f.tag),
                    "directories" => Value::nothing(f.tag),
                    "files" => Value::nothing(f.tag),
                },
                f.tag,
            )
        } else {
            Value::record(
                record! {
                    "path" => Value::string(f.path.display().to_string(), f.tag),
                    "apparent" => Value::filesize(f.size as i64, f.tag),
                    "physical" => Value::filesize(f.blocks.unwrap_or(0) as i64, f.tag),
                },
                f.tag,
            )
        }
    }
}

fn value_from_vec<V>(vec: Vec<V>, tag: Span) -> Value
where
    V: Into<Value>,
{
    if vec.is_empty() {
        Value::nothing(tag)
    } else {
        let values = vec.into_iter().map(Into::into).collect::<Vec<Value>>();
        Value::list(values, tag)
    }
}
