use crate::{
    engine::{StateWorkingSet, VirtualPath},
    FileId,
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// An abstraction over a PathBuf that can have virtual paths (files and directories). Virtual
/// paths always exist and represent a way to ship Nushell code inside the binary without requiring
/// paths to be present in the file system.
///
/// Created from VirtualPath found in the engine state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParserPath {
    RealPath(PathBuf),
    VirtualFile(PathBuf, usize),
    VirtualDir(PathBuf, Vec<ParserPath>),
}

impl ParserPath {
    pub fn is_dir(&self) -> bool {
        match self {
            ParserPath::RealPath(p) => p.is_dir(),
            ParserPath::VirtualFile(..) => false,
            ParserPath::VirtualDir(..) => true,
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            ParserPath::RealPath(p) => p.is_file(),
            ParserPath::VirtualFile(..) => true,
            ParserPath::VirtualDir(..) => false,
        }
    }

    pub fn exists(&self) -> bool {
        match self {
            ParserPath::RealPath(p) => p.exists(),
            ParserPath::VirtualFile(..) => true,
            ParserPath::VirtualDir(..) => true,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            ParserPath::RealPath(p) => p,
            ParserPath::VirtualFile(p, _) => p,
            ParserPath::VirtualDir(p, _) => p,
        }
    }

    pub fn path_buf(self) -> PathBuf {
        match self {
            ParserPath::RealPath(p) => p,
            ParserPath::VirtualFile(p, _) => p,
            ParserPath::VirtualDir(p, _) => p,
        }
    }

    pub fn parent(&self) -> Option<&Path> {
        match self {
            ParserPath::RealPath(p) => p.parent(),
            ParserPath::VirtualFile(p, _) => p.parent(),
            ParserPath::VirtualDir(p, _) => p.parent(),
        }
    }

    pub fn read_dir(&self) -> Option<Vec<ParserPath>> {
        match self {
            ParserPath::RealPath(p) => p.read_dir().ok().map(|read_dir| {
                read_dir
                    .flatten()
                    .map(|dir_entry| ParserPath::RealPath(dir_entry.path()))
                    .collect()
            }),
            ParserPath::VirtualFile(..) => None,
            ParserPath::VirtualDir(_, files) => Some(files.clone()),
        }
    }

    pub fn file_stem(&self) -> Option<&OsStr> {
        self.path().file_stem()
    }

    pub fn extension(&self) -> Option<&OsStr> {
        self.path().extension()
    }

    pub fn join(self, path: impl AsRef<Path>) -> ParserPath {
        match self {
            ParserPath::RealPath(p) => ParserPath::RealPath(p.join(path)),
            ParserPath::VirtualFile(p, file_id) => ParserPath::VirtualFile(p.join(path), file_id),
            ParserPath::VirtualDir(p, entries) => {
                let new_p = p.join(path);
                let mut pp = ParserPath::RealPath(new_p.clone());
                for entry in entries {
                    if new_p == entry.path() {
                        pp = entry.clone();
                    }
                }
                pp
            }
        }
    }

    pub fn open<'a>(
        &'a self,
        working_set: &'a StateWorkingSet,
    ) -> std::io::Result<Box<dyn std::io::Read + 'a>> {
        match self {
            ParserPath::RealPath(p) => {
                std::fs::File::open(p).map(|f| Box::new(f) as Box<dyn std::io::Read>)
            }
            ParserPath::VirtualFile(_, file_id) => working_set
                .get_contents_of_file(FileId::new(*file_id))
                .map(|bytes| Box::new(bytes) as Box<dyn std::io::Read>)
                .ok_or(std::io::ErrorKind::NotFound.into()),

            ParserPath::VirtualDir(..) => Err(std::io::ErrorKind::NotFound.into()),
        }
    }

    pub fn read<'a>(&'a self, working_set: &'a StateWorkingSet) -> Option<Vec<u8>> {
        self.open(working_set)
            .and_then(|mut reader| {
                let mut vec = vec![];
                reader.read_to_end(&mut vec)?;
                Ok(vec)
            })
            .ok()
    }

    pub fn from_virtual_path(
        working_set: &StateWorkingSet,
        name: &str,
        virtual_path: &VirtualPath,
    ) -> Self {
        match virtual_path {
            VirtualPath::File(file_id) => {
                ParserPath::VirtualFile(PathBuf::from(name), file_id.get())
            }
            VirtualPath::Dir(entries) => ParserPath::VirtualDir(
                PathBuf::from(name),
                entries
                    .iter()
                    .map(|virtual_path_id| {
                        let (virt_name, virt_path) = working_set.get_virtual_path(*virtual_path_id);
                        ParserPath::from_virtual_path(working_set, virt_name, virt_path)
                    })
                    .collect(),
            ),
        }
    }
}
