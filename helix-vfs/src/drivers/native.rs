use std::io;
use std::path::{Path, PathBuf};

use crate::driver::FsDriver;
use crate::types::{DirEntry, FileType, Metadata, Permissions};
use crate::walk::{Walk, WalkConfig, WalkEntry, WalkState, WalkVisitor};

pub struct NativeFs;

fn convert_entry(entry: &ignore::DirEntry) -> WalkEntry {
    let is_symlink = entry.path_is_symlink();
    let file_type = entry
        .file_type()
        .map(|ft| {
            if ft.is_symlink() {
                FileType::Symlink
            } else if ft.is_dir() {
                FileType::Directory
            } else {
                FileType::File
            }
        })
        .unwrap_or(FileType::File);
    WalkEntry::new(entry.path().to_path_buf(), file_type, entry.depth(), is_symlink)
}

fn apply_walk_config(builder: &mut ignore::WalkBuilder, config: &WalkConfig) {
    builder
        .hidden(config.hidden)
        .parents(config.parents)
        .ignore(config.ignore)
        .follow_links(config.follow_links)
        .git_ignore(config.git_ignore)
        .git_global(config.git_global)
        .git_exclude(config.git_exclude)
        .max_depth(config.max_depth);

    if let Some(ref cmp) = config.sort_by_file_name {
        let cmp = cmp.clone();
        builder.sort_by_file_name(move |a, b| cmp(a, b));
    }

    for path in &config.custom_ignore_files {
        builder.add_custom_ignore_filename(path);
    }

    if let Some(ref filter) = config.filter {
        let filter = filter.clone();
        builder.filter_entry(move |entry| {
            let walk_entry = convert_entry(entry);
            filter(&walk_entry)
        });
    }
}

impl FsDriver for NativeFs {
    fn name(&self) -> &'static str {
        "native"
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        std::fs::write(path, contents)
    }
    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        std::fs::metadata(path).map(Metadata::from_std)
    }
    fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata> {
        std::fs::symlink_metadata(path).map(Metadata::from_std_symlink)
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let ft = entry.file_type()?;
            entries.push(DirEntry {
                path: entry.path(),
                file_name: entry.file_name(),
                file_type: FileType::from(ft),
            });
        }
        Ok(entries)
    }
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir(path)
    }
    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        std::fs::copy(from, to)
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }
    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::read_link(path)
    }
    fn set_permissions(&self, path: &Path, perm: Permissions) -> io::Result<()> {
        let mut std_perm = std::fs::metadata(path)?.permissions();
        std_perm.set_readonly(perm.readonly());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std_perm.set_mode(perm.mode());
        }
        std::fs::set_permissions(path, std_perm)
    }

    fn walk(&self, root: &Path, config: &WalkConfig) -> Walk {
        let mut builder = ignore::WalkBuilder::new(root);
        apply_walk_config(&mut builder, config);
        Walk::new(builder.build().map(|result| {
            result
                .map(|entry| {
                    let is_symlink = entry.path_is_symlink();
                    let file_type = entry
                        .file_type()
                        .map(|ft| {
                            if ft.is_symlink() {
                                FileType::Symlink
                            } else if ft.is_dir() {
                                FileType::Directory
                            } else {
                                FileType::File
                            }
                        })
                        .unwrap_or(FileType::File);
                    let depth = entry.depth();
                    WalkEntry::new(entry.into_path(), file_type, depth, is_symlink)
                })
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        }))
    }

    fn walk_parallel<'s>(
        &self,
        root: &Path,
        config: &WalkConfig,
        builder: &mut (dyn FnMut() -> WalkVisitor<'s> + 's),
    ) {
        let mut walk_builder = ignore::WalkBuilder::new(root);
        apply_walk_config(&mut walk_builder, config);
        walk_builder.build_parallel().run(|| {
            let mut visitor = builder();
            Box::new(
                move |result: Result<ignore::DirEntry, ignore::Error>| -> ignore::WalkState {
                    let mapped = result
                        .map(|entry| {
                            let is_symlink = entry.path_is_symlink();
                            let file_type = entry
                                .file_type()
                                .map(|ft| {
                                    if ft.is_symlink() {
                                        FileType::Symlink
                                    } else if ft.is_dir() {
                                        FileType::Directory
                                    } else {
                                        FileType::File
                                    }
                                })
                                .unwrap_or(FileType::File);
                            let depth = entry.depth();
                            WalkEntry::new(entry.into_path(), file_type, depth, is_symlink)
                        })
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
                    match visitor(mapped) {
                        WalkState::Continue => ignore::WalkState::Continue,
                        WalkState::Skip => ignore::WalkState::Skip,
                        WalkState::Quit => ignore::WalkState::Quit,
                    }
                },
            )
        });
    }
}
