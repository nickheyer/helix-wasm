use std::io;
use std::path::{Path, PathBuf};

use crate::types::{DirEntry, Metadata, Permissions};
use crate::walk::{Walk, WalkConfig, WalkState, WalkVisitor};

pub trait FsDriver: Send + Sync {
    /// Short name shown in `:fsdriver` completions (e.g. "native", "memory").
    fn name(&self) -> &'static str;

    /// Load runtime files (configs, themes, etc.) into the driver so helix can
    /// find them at the paths it expects.  For native this is a no-op because
    /// the files are already on disk.
    fn bootstrap(&self) -> io::Result<()> {
        Ok(())
    }

    /// Recursively copy a directory from the real filesystem into this driver.
    /// No-op for drivers backed by the real filesystem.
    fn mirror_from_disk(&self, _path: &Path) {}

    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()>;
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;
    fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool {
        self.metadata(path).map(|m| m.is_file()).unwrap_or(false)
    }
    fn is_dir(&self, path: &Path) -> bool {
        self.metadata(path).map(|m| m.is_dir()).unwrap_or(false)
    }
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn remove_dir(&self, path: &Path) -> io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64>;
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
    fn set_permissions(&self, path: &Path, perm: Permissions) -> io::Result<()>;
    fn walk(&self, root: &Path, config: &WalkConfig) -> Walk;

    /// Walk a directory tree in parallel. The default implementation falls
    /// back to sequential iteration.
    fn walk_parallel<'s>(
        &self,
        root: &Path,
        config: &WalkConfig,
        builder: &mut (dyn FnMut() -> WalkVisitor<'s> + 's),
    ) {
        let mut visitor = builder();
        for entry in self.walk(root, config) {
            match visitor(entry) {
                WalkState::Continue | WalkState::Skip => {}
                WalkState::Quit => break,
            }
        }
    }
}
