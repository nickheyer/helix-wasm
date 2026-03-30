pub mod driver;
pub mod drivers;
pub mod types;
pub mod walk;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use driver::FsDriver;
use parking_lot::RwLock;
pub use types::{DirEntry, FileType, Metadata, Permissions, ReadDir};
pub use walk::{Walk, WalkBuilder, WalkConfig, WalkEntry, WalkParallel, WalkState, WalkVisitor};

struct Registry {
    drivers: Vec<Box<dyn FsDriver>>,
    active: RwLock<usize>,
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

fn registry() -> &'static Registry {
    REGISTRY.get().expect("helix_vfs: no drivers registered")
}

pub(crate) fn get() -> &'static dyn FsDriver {
    let reg = registry();
    let idx = *reg.active.read();
    reg.drivers[idx].as_ref()
}

/// Register one or more drivers. First registered becomes the default.
/// Each driver is bootstrapped as it is registered.
///
/// Call once at startup before any fs operations.
pub fn register(new_drivers: Vec<Box<dyn FsDriver>>) {
    assert!(!new_drivers.is_empty(), "must register at least one driver");
    for d in &new_drivers {
        if let Err(e) = d.bootstrap() {
            log::warn!("vfs: bootstrap for '{}' failed: {e}", d.name());
        }
    }
    if REGISTRY
        .set(Registry {
            drivers: new_drivers,
            active: RwLock::new(0),
        })
        .is_err()
    {
        panic!("helix_vfs already initialized");
    }
}

/// Convenience: register a single driver.
pub fn init(driver: impl FsDriver + 'static) {
    register(vec![Box::new(driver)]);
}

/// Switch the active driver by name. Returns an error if not found.
pub fn set_active(name: &str) -> Result<(), String> {
    let reg = registry();
    for (i, d) in reg.drivers.iter().enumerate() {
        if d.name() == name {
            *reg.active.write() = i;
            return Ok(());
        }
    }
    Err(format!("unknown fs driver: '{name}'"))
}

/// Get a registered driver by name.
pub fn get_driver(name: &str) -> Option<&'static dyn FsDriver> {
    let reg = registry();
    reg.drivers.iter().find(|d| d.name() == name).map(|d| d.as_ref())
}

/// Mirror a directory from the real filesystem into all non-active drivers
/// that support it (e.g. InMemoryFs). Call after helix_loader is initialized.
pub fn mirror_runtime(paths: &[&Path]) {
    let reg = registry();
    for driver in &reg.drivers {
        for path in paths {
            driver.mirror_from_disk(path);
        }
    }
}

/// Name of the currently active driver.
pub fn active_name() -> &'static str {
    get().name()
}

/// List names of all registered drivers.
pub fn driver_names() -> Vec<&'static str> {
    registry().drivers.iter().map(|d| d.name()).collect()
}

// ── Free functions mirroring std::fs ────────────────────────────────────────

pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    get().read(path.as_ref())
}

pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    get().read_to_string(path.as_ref())
}

pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    get().write(path.as_ref(), contents.as_ref())
}

pub fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    get().metadata(path.as_ref())
}

pub fn symlink_metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    get().symlink_metadata(path.as_ref())
}

pub fn exists(path: impl AsRef<Path>) -> bool {
    get().exists(path.as_ref())
}

pub fn is_file(path: impl AsRef<Path>) -> bool {
    get().is_file(path.as_ref())
}

pub fn is_dir(path: impl AsRef<Path>) -> bool {
    get().is_dir(path.as_ref())
}

pub fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    let entries = get().read_dir(path.as_ref())?;
    Ok(ReadDir::new(entries))
}

pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    get().create_dir_all(path.as_ref())
}

pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    get().remove_file(path.as_ref())
}

pub fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    get().remove_dir(path.as_ref())
}

pub fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    get().remove_dir_all(path.as_ref())
}

pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    get().rename(from.as_ref(), to.as_ref())
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    get().copy(from.as_ref(), to.as_ref())
}

pub fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    get().canonicalize(path.as_ref())
}

pub fn read_link(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    get().read_link(path.as_ref())
}

pub fn set_permissions(path: impl AsRef<Path>, perm: Permissions) -> io::Result<()> {
    get().set_permissions(path.as_ref(), perm)
}
