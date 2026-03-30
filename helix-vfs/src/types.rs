use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use helix_stdx::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}

impl FileType {
    pub fn is_file(&self) -> bool {
        matches!(self, FileType::File)
    }
    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }
    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub(crate) len: u64,
    pub(crate) file_type: FileType,
    pub(crate) readonly: bool,
    pub(crate) modified: Option<SystemTime>,
    #[cfg(unix)]
    pub(crate) mode: u32,
    #[cfg(unix)]
    pub(crate) nlink: u64,
    #[cfg(unix)]
    pub(crate) gid: u32,
}

impl Metadata {
    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }
    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn file_type(&self) -> FileType {
        self.file_type
    }
    pub fn modified(&self) -> io::Result<SystemTime> {
        self.modified.ok_or_else(|| {
            io::Error::new(io::ErrorKind::Unsupported, "modified time not available")
        })
    }
    pub fn permissions(&self) -> Permissions {
        Permissions {
            readonly: self.readonly,
            #[cfg(unix)]
            mode: self.mode,
        }
    }
    #[cfg(unix)]
    pub fn nlink(&self) -> u64 {
        self.nlink
    }
    #[cfg(unix)]
    pub fn gid(&self) -> u32 {
        self.gid
    }
}

#[derive(Debug, Clone)]
pub struct Permissions {
    pub(crate) readonly: bool,
    #[cfg(unix)]
    pub(crate) mode: u32,
}

impl Permissions {
    pub fn readonly(&self) -> bool {
        self.readonly
    }
    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }
    #[cfg(unix)]
    pub fn mode(&self) -> u32 {
        self.mode
    }
    #[cfg(unix)]
    pub fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub(crate) path: PathBuf,
    pub(crate) file_name: OsString,
    pub(crate) file_type: FileType,
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_name(&self) -> OsString {
        self.file_name.clone()
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(self.file_type)
    }
}

pub struct ReadDir {
    entries: Vec<DirEntry>,
    pos: usize,
}

impl ReadDir {
    pub fn new(entries: Vec<DirEntry>) -> Self {
        ReadDir { entries, pos: 0 }
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.entries.len() {
            let entry = self.entries[self.pos].clone();
            self.pos += 1;
            Some(Ok(entry))
        } else {
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<std::fs::FileType> for FileType {
    fn from(ft: std::fs::FileType) -> Self {
        if ft.is_symlink() {
            FileType::Symlink
        } else if ft.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Metadata {
    pub fn from_std(m: std::fs::Metadata) -> Self {
        let file_type = if m.is_symlink() {
            FileType::Symlink
        } else if m.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };
        Metadata {
            len: m.len(),
            file_type,
            readonly: m.permissions().readonly(),
            modified: m.modified().ok(),
            #[cfg(unix)]
            mode: {
                use std::os::unix::fs::PermissionsExt;
                m.permissions().mode()
            },
            #[cfg(unix)]
            nlink: {
                use std::os::unix::fs::MetadataExt;
                m.nlink()
            },
            #[cfg(unix)]
            gid: {
                use std::os::unix::fs::MetadataExt;
                m.gid()
            },
        }
    }

    pub fn from_std_symlink(m: std::fs::Metadata) -> Self {
        let file_type = if m.file_type().is_symlink() {
            FileType::Symlink
        } else if m.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        };
        Metadata {
            len: m.len(),
            file_type,
            readonly: m.permissions().readonly(),
            modified: m.modified().ok(),
            #[cfg(unix)]
            mode: {
                use std::os::unix::fs::PermissionsExt;
                m.permissions().mode()
            },
            #[cfg(unix)]
            nlink: {
                use std::os::unix::fs::MetadataExt;
                m.nlink()
            },
            #[cfg(unix)]
            gid: {
                use std::os::unix::fs::MetadataExt;
                m.gid()
            },
        }
    }
}
