use std::collections::HashMap;
use std::io;
use std::path::{Component, Path, PathBuf};

use helix_stdx::time::SystemTime;
use parking_lot::RwLock;

use crate::driver::FsDriver;
use crate::types::{DirEntry, FileType, Metadata, Permissions};
use crate::walk::{Walk, WalkConfig, WalkEntry};

#[cfg(feature = "embed-runtime")]
include!(concat!(env!("OUT_DIR"), "/embedded_runtime.rs"));

#[derive(Debug, Clone)]
enum FsNode {
    File {
        contents: Vec<u8>,
        modified: SystemTime,
        readonly: bool,
    },
    Directory,
    Symlink {
        target: PathBuf,
    },
}

pub struct InMemoryFs {
    nodes: RwLock<HashMap<PathBuf, FsNode>>,
}

impl InMemoryFs {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(PathBuf::from("/"), FsNode::Directory);
        InMemoryFs {
            nodes: RwLock::new(nodes),
        }
    }

    pub fn load_file(&self, path: impl Into<PathBuf>, contents: impl Into<Vec<u8>>) {
        let path = path.into();
        if let Some(parent) = path.parent() {
            self.ensure_dirs(parent);
        }
        self.nodes.write().insert(
            path,
            FsNode::File {
                contents: contents.into(),
                modified: SystemTime::now(),
                readonly: false,
            },
        );
    }

    pub fn load_dir(&self, path: impl Into<PathBuf>) {
        self.ensure_dirs(&path.into());
    }

    /// Recursively copy an entire directory from the real filesystem into the
    /// in-memory VFS, preserving the same paths.
    pub fn mirror_from_real_fs(&self, path: &Path) {
        fn walk(fs: &InMemoryFs, dir: &Path) {
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => return,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    fs.load_dir(&path);
                    walk(fs, &path);
                } else if path.is_file() {
                    if let Ok(contents) = std::fs::read(&path) {
                        fs.load_file(&path, contents);
                    }
                }
            }
        }

        self.load_dir(path);
        walk(self, path);
    }

    fn ensure_dirs(&self, path: &Path) {
        let mut nodes = self.nodes.write();
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            nodes.entry(current.clone()).or_insert(FsNode::Directory);
        }
    }

    fn normalize(path: &Path) -> PathBuf {
        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    result.pop();
                }
                Component::CurDir => {}
                other => result.push(other),
            }
        }
        result
    }

    fn resolve_symlinks(&self, path: &Path, depth: u8) -> io::Result<PathBuf> {
        if depth > 32 {
            return Err(io::Error::new(io::ErrorKind::Other, "too many symlink levels"));
        }
        let path = Self::normalize(path);
        let nodes = self.nodes.read();
        match nodes.get(&path) {
            Some(FsNode::Symlink { target }) => {
                let resolved = if target.is_relative() {
                    path.parent().unwrap_or(Path::new("/")).join(target)
                } else {
                    target.clone()
                };
                drop(nodes);
                self.resolve_symlinks(&resolved, depth + 1)
            }
            _ => Ok(path),
        }
    }
}

impl Default for InMemoryFs {
    fn default() -> Self {
        Self::new()
    }
}

impl FsDriver for InMemoryFs {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn bootstrap(&self) -> io::Result<()> {
        // User home directory and workspace marker so find_workspace()
        // identifies /home/user as the workspace root.
        self.load_dir("/home/user/.helix");
        self.load_dir("/home/user");
        self.load_dir("/tmp");

        // System directories expected by helix-loader.
        self.load_dir("/helix/config");
        self.load_dir("/helix/cache");
        self.load_dir("/helix/data");

        // Seed a welcome file so the file picker has something to show
        // and the user isn't dropped into a completely empty workspace.
        self.load_file(
            "/home/user/welcome.md",
            b"# Welcome to Helix (WASM)\n\
              \n\
              This is helix running in your browser.\n\
              \n\
              Use `:open <path>` or the file picker (`Space f`) to browse files.\n\
              Create new files with `:write <path>` (`:w path/to/file.txt`).\n\
              \n\
              The virtual filesystem lives entirely in memory - files are lost on page refresh.\n"
                .to_vec(),
        );

        #[cfg(feature = "embed-runtime")]
        {
            let base = std::path::Path::new("/helix/runtime");
            for (rel_path, contents) in EMBEDDED_RUNTIME {
                self.load_file(base.join(rel_path), *contents);
            }
            log::info!("vfs: loaded {} embedded runtime files", EMBEDDED_RUNTIME.len());
        }

        #[cfg(not(feature = "embed-runtime"))]
        {
            self.load_dir("/helix/runtime");
        }

        Ok(())
    }

    fn mirror_from_disk(&self, path: &Path) {
        self.mirror_from_real_fs(path);
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let path = self.resolve_symlinks(path, 0)?;
        let nodes = self.nodes.read();
        match nodes.get(&path) {
            Some(FsNode::File { contents, .. }) => Ok(contents.clone()),
            Some(FsNode::Directory) => Err(io::Error::new(io::ErrorKind::InvalidInput, "Is a directory")),
            _ => Err(io::Error::new(io::ErrorKind::NotFound, format!("not found: {}", path.display()))),
        }
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        let path = Self::normalize(path);
        if let Some(parent) = path.parent() {
            self.ensure_dirs(parent);
        }
        let mut nodes = self.nodes.write();
        if let Some(FsNode::File { readonly: true, .. }) = nodes.get(&path) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "file is readonly"));
        }
        nodes.insert(
            path,
            FsNode::File {
                contents: contents.to_vec(),
                modified: SystemTime::now(),
                readonly: false,
            },
        );
        Ok(())
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        let resolved = self.resolve_symlinks(path, 0)?;
        let nodes = self.nodes.read();
        match nodes.get(&resolved) {
            Some(FsNode::File { contents, modified, readonly }) => Ok(Metadata {
                len: contents.len() as u64,
                file_type: FileType::File,
                readonly: *readonly,
                modified: Some(*modified),
                #[cfg(unix)]
                mode: if *readonly { 0o444 } else { 0o644 },
                #[cfg(unix)]
                nlink: 1,
                #[cfg(unix)]
                gid: 0,
            }),
            Some(FsNode::Directory) => Ok(Metadata {
                len: 0,
                file_type: FileType::Directory,
                readonly: false,
                modified: None,
                #[cfg(unix)]
                mode: 0o755,
                #[cfg(unix)]
                nlink: 1,
                #[cfg(unix)]
                gid: 0,
            }),
            _ => Err(io::Error::new(io::ErrorKind::NotFound, format!("not found: {}", path.display()))),
        }
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata> {
        let path = Self::normalize(path);
        let nodes = self.nodes.read();
        match nodes.get(&path) {
            Some(FsNode::File { contents, modified, readonly }) => Ok(Metadata {
                len: contents.len() as u64,
                file_type: FileType::File,
                readonly: *readonly,
                modified: Some(*modified),
                #[cfg(unix)]
                mode: if *readonly { 0o444 } else { 0o644 },
                #[cfg(unix)]
                nlink: 1,
                #[cfg(unix)]
                gid: 0,
            }),
            Some(FsNode::Directory) => Ok(Metadata {
                len: 0,
                file_type: FileType::Directory,
                readonly: false,
                modified: None,
                #[cfg(unix)]
                mode: 0o755,
                #[cfg(unix)]
                nlink: 1,
                #[cfg(unix)]
                gid: 0,
            }),
            Some(FsNode::Symlink { .. }) => Ok(Metadata {
                len: 0,
                file_type: FileType::Symlink,
                readonly: false,
                modified: None,
                #[cfg(unix)]
                mode: 0o777,
                #[cfg(unix)]
                nlink: 1,
                #[cfg(unix)]
                gid: 0,
            }),
            None => Err(io::Error::new(io::ErrorKind::NotFound, format!("not found: {}", path.display()))),
        }
    }

    fn exists(&self, path: &Path) -> bool {
        let path = Self::normalize(path);
        self.nodes.read().contains_key(&path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let path = self.resolve_symlinks(path, 0)?;
        let nodes = self.nodes.read();
        match nodes.get(&path) {
            Some(FsNode::Directory) => {}
            Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a directory")),
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "directory not found")),
        }
        let mut entries = Vec::new();
        for (entry_path, node) in nodes.iter() {
            if let Some(parent) = entry_path.parent() {
                if parent == path && entry_path != &path {
                    let file_type = match node {
                        FsNode::File { .. } => FileType::File,
                        FsNode::Directory => FileType::Directory,
                        FsNode::Symlink { .. } => FileType::Symlink,
                    };
                    entries.push(DirEntry {
                        path: entry_path.clone(),
                        file_name: entry_path.file_name().unwrap_or_default().to_os_string(),
                        file_type,
                    });
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.ensure_dirs(&Self::normalize(path));
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.write();
        match nodes.get(&path) {
            Some(FsNode::File { .. } | FsNode::Symlink { .. }) => {
                nodes.remove(&path);
                Ok(())
            }
            Some(FsNode::Directory) => Err(io::Error::new(io::ErrorKind::InvalidInput, "is a directory")),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "file not found")),
        }
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.write();
        let has_children = nodes.keys().any(|k| k != &path && k.parent() == Some(&path));
        if has_children {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "directory not empty"));
        }
        match nodes.get(&path) {
            Some(FsNode::Directory) => {
                nodes.remove(&path);
                Ok(())
            }
            Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a directory")),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "directory not found")),
        }
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.write();
        match nodes.get(&path) {
            Some(FsNode::Directory) => {
                let to_remove: Vec<PathBuf> = nodes
                    .keys()
                    .filter(|k| k.starts_with(&path))
                    .cloned()
                    .collect();
                for k in to_remove {
                    nodes.remove(&k);
                }
                Ok(())
            }
            Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a directory")),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "directory not found")),
        }
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let from = Self::normalize(from);
        let to = Self::normalize(to);
        let mut nodes = self.nodes.write();
        match nodes.remove(&from) {
            Some(node) => {
                nodes.insert(to, node);
                Ok(())
            }
            None => Err(io::Error::new(io::ErrorKind::NotFound, "source not found")),
        }
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        let contents = self.read(from)?;
        let len = contents.len() as u64;
        self.write(to, &contents)?;
        Ok(len)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.resolve_symlinks(path, 0)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        let path = Self::normalize(path);
        let nodes = self.nodes.read();
        match nodes.get(&path) {
            Some(FsNode::Symlink { target }) => Ok(target.clone()),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a symbolic link")),
        }
    }

    fn set_permissions(&self, path: &Path, perm: Permissions) -> io::Result<()> {
        let path = Self::normalize(path);
        let mut nodes = self.nodes.write();
        match nodes.get_mut(&path) {
            Some(FsNode::File { readonly, .. }) => {
                *readonly = perm.readonly();
                Ok(())
            }
            Some(_) => Ok(()),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "not found")),
        }
    }

    fn walk(&self, root: &Path, config: &WalkConfig) -> Walk {
        let root = Self::normalize(root);
        let nodes = self.nodes.read();

        match nodes.get(&root) {
            None => Walk::new(std::iter::empty()),
            Some(FsNode::File { .. }) => {
                // Walking a single file: yield just that entry (matches ignore crate behavior).
                let entry = WalkEntry::new(root, FileType::File, 0, false);
                Walk::new(std::iter::once(Ok(entry)))
            }
            Some(FsNode::Symlink { .. }) => {
                let entry = WalkEntry::new(root, FileType::Symlink, 0, true);
                Walk::new(std::iter::once(Ok(entry)))
            }
            Some(FsNode::Directory) => {
                let mut entries = Vec::new();
                walk_dir(&nodes, &root, 0, config, &mut entries);
                Walk::new(entries.into_iter().map(Ok))
            }
        }
    }
}

/// Recursive depth-first walk of the in-memory filesystem.
fn walk_dir(
    nodes: &HashMap<PathBuf, FsNode>,
    dir: &Path,
    depth: usize,
    config: &WalkConfig,
    out: &mut Vec<WalkEntry>,
) {
    // Emit the directory entry itself.
    let is_sym = nodes
        .get(dir)
        .map_or(false, |n| matches!(n, FsNode::Symlink { .. }));
    out.push(WalkEntry::new(
        dir.to_path_buf(),
        FileType::Directory,
        depth,
        is_sym,
    ));

    // If we've reached max_depth, don't descend further.
    if let Some(max) = config.max_depth {
        if depth >= max {
            return;
        }
    }

    // Collect direct children.
    let mut children: Vec<_> = nodes
        .iter()
        .filter(|(path, _)| path.parent() == Some(dir) && *path != dir)
        .collect();

    // Sort by filename if requested.
    if let Some(ref cmp) = config.sort_by_file_name {
        children.sort_by(|(a, _), (b, _)| {
            let a_name = a.file_name().unwrap_or_default();
            let b_name = b.file_name().unwrap_or_default();
            cmp(a_name, b_name)
        });
    }

    for (path, node) in children {
        // Skip hidden entries when configured.
        if config.hidden {
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with('.'))
            {
                continue;
            }
        }

        let (ft, is_sym) = match node {
            FsNode::File { .. } => (FileType::File, false),
            FsNode::Directory => (FileType::Directory, false),
            FsNode::Symlink { target } => {
                // Resolve symlinks: check what the target is so we can
                // report the right FileType and optionally follow it.
                let resolved_ft = if config.follow_links {
                    let resolved = if target.is_relative() {
                        dir.join(target)
                    } else {
                        target.clone()
                    };
                    let resolved = InMemoryFs::normalize(&resolved);
                    match nodes.get(&resolved) {
                        Some(FsNode::Directory) => FileType::Directory,
                        Some(FsNode::File { .. }) => FileType::File,
                        _ => FileType::Symlink,
                    }
                } else {
                    FileType::Symlink
                };
                (resolved_ft, true)
            }
        };

        let entry = WalkEntry::new(path.clone(), ft, depth + 1, is_sym);

        // Apply user-provided filter (skips directories and their children).
        if let Some(ref filter) = config.filter {
            if !filter(&entry) {
                continue;
            }
        }

        if ft == FileType::Directory {
            walk_dir(nodes, path, depth + 1, config, out);
        } else {
            out.push(entry);
        }
    }
}
