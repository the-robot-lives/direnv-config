use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use anyhow::{Context, Result};

/// An advisory exclusive lock over a dc store, released on drop.
pub struct StoreLock {
    // Held only for its side effect: closing the fd releases the flock.
    _file: File,
}

/// Acquire an exclusive advisory lock for the given store directory.
///
/// Serializes read-modify-write mutations (`set`, `yaml`, `unset`, `prune`,
/// version bumps) so concurrent `dc` invocations from different shells cannot
/// interleave and lose each other's writes to a layer file or the `.version`
/// counter. The lock is advisory (`flock(2)`) on `<store>/.lock` and is
/// released automatically when the returned guard is dropped. Read-only paths
/// (`get`, `env`, `bat`) do not take it.
// ⟦𓈌𓉋𓅿𓏖⟧ lock_store :: Acquire an exclusive advisory lock for the given store directory.
pub fn lock_store(store: &Path) -> Result<StoreLock> {
    std::fs::create_dir_all(store)
        .with_context(|| format!("failed to create store directory: {}", store.display()))?;
    let lock_path = store.join(".lock");
    let file = OpenOptions::new()
        .create(true)
        // The lock file is a pure flock sentinel; never write to or truncate it.
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("failed to open lock file: {}", lock_path.display()))?;

    // Blocking exclusive lock; released when `file` is dropped/closed.
    let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if ret != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| {
            format!("failed to acquire store lock: {}", lock_path.display())
        });
    }
    Ok(StoreLock { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn lock_creates_lockfile_and_releases_on_drop() {
        let tmp = TempDir::new().unwrap();
        {
            let _guard = lock_store(tmp.path()).unwrap();
            assert!(tmp.path().join(".lock").exists());
        }
        // A second acquisition after the first is dropped must succeed.
        let _again = lock_store(tmp.path()).unwrap();
    }
}
