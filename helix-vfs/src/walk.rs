use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::types::FileType;

/// An entry produced by walking a directory tree.
pub struct WalkEntry {
    path: PathBuf,
    file_type: FileType,
    depth: usize,
    is_symlink: bool,
}

impl WalkEntry {
    pub fn new(path: PathBuf, file_type: FileType, depth: usize, is_symlink: bool) -> Self {
        Self {
            path,
            file_type,
            depth,
            is_symlink,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }

    pub fn file_name(&self) -> &OsStr {
        self.path.file_name().unwrap_or_default()
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn path_is_symlink(&self) -> bool {
        self.is_symlink
    }

    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }

    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }
}

/// Walk state returned by parallel walk visitors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkState {
    Continue,
    Skip,
    Quit,
}

/// Visitor closure type for parallel walks.
pub type WalkVisitor<'a> = Box<dyn FnMut(io::Result<WalkEntry>) -> WalkState + Send + 'a>;

/// Configuration for a directory walk.
pub struct WalkConfig {
    pub hidden: bool,
    pub parents: bool,
    pub ignore: bool,
    pub follow_links: bool,
    pub git_ignore: bool,
    pub git_global: bool,
    pub git_exclude: bool,
    pub max_depth: Option<usize>,
    pub custom_ignore_files: Vec<PathBuf>,
    pub sort_by_file_name:
        Option<Arc<dyn Fn(&OsStr, &OsStr) -> std::cmp::Ordering + Send + Sync>>,
    pub filter: Option<Arc<dyn Fn(&WalkEntry) -> bool + Send + Sync>>,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            hidden: true,
            parents: true,
            ignore: true,
            follow_links: false,
            git_ignore: true,
            git_global: true,
            git_exclude: true,
            max_depth: None,
            custom_ignore_files: Vec::new(),
            sort_by_file_name: None,
            filter: None,
        }
    }
}

/// Sequential directory walk iterator.
pub struct Walk {
    inner: Box<dyn Iterator<Item = io::Result<WalkEntry>> + Send>,
}

impl Walk {
    pub fn new(inner: impl Iterator<Item = io::Result<WalkEntry>> + Send + 'static) -> Self {
        Walk {
            inner: Box::new(inner),
        }
    }
}

impl Iterator for Walk {
    type Item = io::Result<WalkEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Parallel directory walk handle. Call `run()` to execute.
pub struct WalkParallel {
    root: PathBuf,
    config: WalkConfig,
}

impl WalkParallel {
    pub(crate) fn new(root: PathBuf, config: WalkConfig) -> Self {
        WalkParallel { root, config }
    }

    /// Run the parallel walk. The `builder` factory is called once per worker
    /// thread; each returned visitor receives entries until it returns `Quit`
    /// or the walk completes. Visitors may borrow from the calling scope.
    pub fn run<'s>(self, mut builder: impl FnMut() -> WalkVisitor<'s> + 's) {
        crate::get().walk_parallel(&self.root, &self.config, &mut builder);
    }
}

/// Builder for directory walks, mirroring the `ignore::WalkBuilder` API.
///
/// Dispatches to the active VFS driver when `build()` or `build_parallel()`
/// is called.
pub struct WalkBuilder {
    root: PathBuf,
    config: WalkConfig,
}

impl std::fmt::Debug for WalkBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalkBuilder")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl WalkBuilder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        WalkBuilder {
            root: root.into(),
            config: WalkConfig::default(),
        }
    }

    pub fn hidden(&mut self, yes: bool) -> &mut Self {
        self.config.hidden = yes;
        self
    }

    pub fn parents(&mut self, yes: bool) -> &mut Self {
        self.config.parents = yes;
        self
    }

    pub fn ignore(&mut self, yes: bool) -> &mut Self {
        self.config.ignore = yes;
        self
    }

    pub fn follow_links(&mut self, yes: bool) -> &mut Self {
        self.config.follow_links = yes;
        self
    }

    pub fn git_ignore(&mut self, yes: bool) -> &mut Self {
        self.config.git_ignore = yes;
        self
    }

    pub fn git_global(&mut self, yes: bool) -> &mut Self {
        self.config.git_global = yes;
        self
    }

    pub fn git_exclude(&mut self, yes: bool) -> &mut Self {
        self.config.git_exclude = yes;
        self
    }

    pub fn max_depth(&mut self, depth: Option<usize>) -> &mut Self {
        self.config.max_depth = depth;
        self
    }

    pub fn sort_by_file_name(
        &mut self,
        cmp: impl Fn(&OsStr, &OsStr) -> std::cmp::Ordering + Send + Sync + 'static,
    ) -> &mut Self {
        self.config.sort_by_file_name = Some(Arc::new(cmp));
        self
    }

    pub fn filter_entry(
        &mut self,
        filter: impl Fn(&WalkEntry) -> bool + Send + Sync + 'static,
    ) -> &mut Self {
        self.config.filter = Some(Arc::new(filter));
        self
    }

    pub fn add_custom_ignore_filename(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.config.custom_ignore_files.push(path.into());
        self
    }

    pub fn build(&self) -> Walk {
        crate::get().walk(&self.root, &self.config)
    }

    pub fn build_parallel(&self) -> WalkParallel {
        WalkParallel::new(self.root.clone(), self.config_snapshot())
    }

    fn config_snapshot(&self) -> WalkConfig {
        WalkConfig {
            hidden: self.config.hidden,
            parents: self.config.parents,
            ignore: self.config.ignore,
            follow_links: self.config.follow_links,
            git_ignore: self.config.git_ignore,
            git_global: self.config.git_global,
            git_exclude: self.config.git_exclude,
            max_depth: self.config.max_depth,
            custom_ignore_files: self.config.custom_ignore_files.clone(),
            sort_by_file_name: self.config.sort_by_file_name.clone(),
            filter: self.config.filter.clone(),
        }
    }
}
