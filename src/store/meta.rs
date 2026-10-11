use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const META_FILE: &str = ".meta";

/// An external YAML file registered by `dc load`, tracked so `dc config *`
/// can find keys in it without re-parsing `.envrc`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalFile {
    /// Absolute path of the loaded file.
    pub path: PathBuf,
    /// Unix-time seconds of the file's mtime at registration (staleness hint).
    #[serde(default)]
    pub mtime: u64,
    /// File size in bytes at registration (staleness hint).
    #[serde(default)]
    pub size: u64,
}

/// Store metadata persisted as `.meta` YAML inside each store directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMeta {
    /// The original directory this store manages configs for.
    pub source: PathBuf,
    /// ISO 8601 timestamp when the store was created.
    pub created: String,
    /// Optional parent store path (for parent-chain resolution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PathBuf>,
    /// List of config names in this store.
    #[serde(default)]
    pub configs: Vec<String>,
    /// External files registered via `dc load`, in load order (last = last loaded).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external: Vec<ExternalFile>,
}

/// Read and parse the store's `.meta` YAML file.
// <REMOVED UUID HERE> read_meta :: Read and parse the store's `.meta` YAML file.
pub fn read_meta(store: &Path) -> Result<StoreMeta> {
    let path = store.join(META_FILE);
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read meta file: {}", path.display()))?;
    let meta: StoreMeta = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse meta file: {}", path.display()))?;
    Ok(meta)
}

/// Write the store's `.meta` YAML file.
// <REMOVED UUID HERE> write_meta :: Write the store's `.meta` YAML file.
pub fn write_meta(store: &Path, meta: &StoreMeta) -> Result<()> {
    let path = store.join(META_FILE);
    let yaml = serde_yaml::to_string(meta)
        .context("failed to serialize store meta")?;
    std::fs::write(&path, yaml)
        .with_context(|| format!("failed to write meta file: {}", path.display()))?;
    Ok(())
}

/// Scan the store directory for config subdirectories and update the
/// `configs` list in `.meta`.
///
/// Excludes dotfiles/dotdirs and `history/`.
// <REMOVED UUID HERE> update_configs_list :: Scan the store directory for config subdirectories and update the
pub fn update_configs_list(store: &Path) -> Result<()> {
    let mut meta = read_meta(store)?;

    let mut configs: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(store)
        .with_context(|| format!("failed to read store directory: {}", store.display()))?;

    for entry in entries {
        let entry = entry?;
        let ft = entry.file_type()?;
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip dotfiles/dotdirs and history/
        if name_str.starts_with('.') || name_str == "history" {
            continue;
        }
        configs.push(name_str.into_owned());
    }
    configs.sort();
    meta.configs = configs;
    write_meta(store, &meta)?;
    Ok(())
}

/// Register files loaded by `dc load` in the store meta, in load order.
///
/// Files already registered are moved to reflect this invocation's argument
/// order (so the list always ends with the most recently loaded batch, last
/// entry = last loaded). Files registered earlier and not re-loaded keep
/// their position before the new batch.
pub fn register_external_files(store: &Path, files: &[PathBuf]) -> Result<()> {
    let mut meta = read_meta(store)?;
    meta.external.retain(|e| !files.iter().any(|f| e.path == *f));
    for f in files {
        let (mtime, size) = match std::fs::metadata(f) {
            Ok(md) => (
                md.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                md.len(),
            ),
            Err(_) => (0, 0),
        };
        meta.external.push(ExternalFile { path: f.clone(), mtime, size });
    }
    write_meta(store, &meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (TempDir, StoreMeta) {
        let tmp = TempDir::new().unwrap();
        let meta = StoreMeta {
            source: PathBuf::from("/test/dir"),
            created: "2026-01-01T00:00:00+00:00".to_string(),
            parent: None,
            configs: vec!["aws".to_string()],
            external: Vec::new(),
        };
        (tmp, meta)
    }

    #[test]
    fn write_and_read_meta() {
        let (tmp, meta) = make_store();
        write_meta(tmp.path(), &meta).unwrap();
        let loaded = read_meta(tmp.path()).unwrap();
        assert_eq!(loaded.source, meta.source);
        assert_eq!(loaded.created, meta.created);
        assert_eq!(loaded.configs, meta.configs);
        assert!(loaded.parent.is_none());
    }

    #[test]
    fn update_configs_list_scans_subdirs() {
        let (tmp, meta) = make_store();
        write_meta(tmp.path(), &meta).unwrap();

        // Create some subdirectories
        std::fs::create_dir(tmp.path().join("aws")).unwrap();
        std::fs::create_dir(tmp.path().join("db")).unwrap();
        std::fs::create_dir(tmp.path().join(".hidden")).unwrap();
        std::fs::create_dir(tmp.path().join("history")).unwrap();

        update_configs_list(tmp.path()).unwrap();
        let loaded = read_meta(tmp.path()).unwrap();
        assert_eq!(loaded.configs, vec!["aws".to_string(), "db".to_string()]);
    }

    #[test]
    fn read_meta_missing_file_errors() {
        let tmp = TempDir::new().unwrap();
        assert!(read_meta(tmp.path()).is_err());
    }

    #[test]
    fn register_external_files_orders_and_reorders() {
        let (tmp, meta) = make_store();
        write_meta(tmp.path(), &meta).unwrap();

        let a = tmp.path().join("a.yaml");
        let b = tmp.path().join("b.yaml");
        std::fs::write(&a, "x: 1\n").unwrap();
        std::fs::write(&b, "y: 2\n").unwrap();

        register_external_files(tmp.path(), &[a.clone(), b.clone()]).unwrap();
        let loaded = read_meta(tmp.path()).unwrap();
        assert_eq!(
            loaded.external.iter().map(|e| e.path.clone()).collect::<Vec<_>>(),
            vec![a.clone(), b.clone()]
        );
        assert!(loaded.external[0].size > 0);
        assert!(loaded.external[0].mtime > 0);

        // Re-loading only `a` moves it after `b` (this invocation's order wins).
        register_external_files(tmp.path(), &[a.clone()]).unwrap();
        let loaded = read_meta(tmp.path()).unwrap();
        assert_eq!(
            loaded.external.iter().map(|e| e.path.clone()).collect::<Vec<_>>(),
            vec![b.clone(), a.clone()]
        );
    }

    #[test]
    fn legacy_meta_without_external_deserializes() {
        let (tmp, _meta) = make_store();
        std::fs::write(
            tmp.path().join(".meta"),
            "source: /test/dir\ncreated: '2026-01-01T00:00:00+00:00'\nconfigs: []\n",
        )
        .unwrap();
        let loaded = read_meta(tmp.path()).unwrap();
        assert!(loaded.external.is_empty());
    }
}
