use anyhow::Result;
use serde_json::json;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn collect_leaf_keys(val: &Value, prefix: &str, keys: &mut Vec<String>) {
    match val {
        Value::Mapping(m) => {
            for (k, v) in m {
                if let Some(key_str) = k.as_str() {
                    let path = if prefix.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}.{}", prefix, key_str)
                    };
                    collect_leaf_keys(v, &path, keys);
                }
            }
        }
        _ => {
            keys.push(prefix.to_string());
        }
    }
}

/// Every key in one store, per config and per layer file: `(subject, layer,
/// keys)`. Reads every `*.yaml` layer (base/local/env overlays AND
/// `secrets.yaml`) — key names only, values never.
fn store_layer_keys(store: &Path) -> Result<Vec<(String, String, Vec<String>)>> {
    let mut result = Vec::new();
    let entries = std::fs::read_dir(store)?;

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || name_str == "history" {
            continue;
        }

        let mut layers: Vec<(String, Vec<String>)> = Vec::new();
        for layer_file in std::fs::read_dir(store.join(&*name_str))? {
            let layer_file = layer_file?;
            let lf_name = layer_file.file_name();
            let lf_name = lf_name.to_string_lossy();
            if !lf_name.ends_with(".yaml") {
                continue;
            }
            let contents = std::fs::read_to_string(layer_file.path())?;
            let val: Value = match serde_yaml::from_str(&contents) {
                Ok(v) => v,
                Err(_) => continue, // unreadable layer: skip, never fail the listing
            };
            let mut keys = Vec::new();
            collect_leaf_keys(&val, "", &mut keys);
            if !keys.is_empty() {
                let stem = lf_name.strip_suffix(".yaml").unwrap_or(&lf_name).to_string();
                layers.push((stem, keys));
            }
        }
        layers.sort();
        for (layer, keys) in layers {
            result.push((name_str.clone().into_owned(), layer, keys));
        }
    }
    result.sort();
    Ok(result)
}

/// Source annotations for one in-scope key.
type Sources = BTreeSet<String>;

/// Build the merged in-scope key view for the current project: the store's
/// layer files across the resolution chain, the `.envrc*` `dc_yaml` heredoc
/// blocks, and the registered external files. Values are never read into the
/// result — key names and `file:line` / `store:<subject>/<layer>` sources only.
fn collect_scoped_keys(
    chain: &[std::path::PathBuf],
    source_dir: &Path,
    external: &[crate::store::meta::ExternalFile],
) -> Result<BTreeMap<(String, String), Sources>> {
    let mut merged: BTreeMap<(String, String), Sources> = BTreeMap::new();

    // 1) store layers
    for s in chain {
        for (config, layer, keys) in store_layer_keys(s)? {
            for k in keys {
                merged
                    .entry((config.clone(), k))
                    .or_default()
                    .insert(format!("store:{config}/{layer}"));
            }
        }
    }

    // 2) .envrc* heredoc blocks
    for f in crate::envrc::locator::envrc_files(source_dir) {
        if let Ok(locs) = crate::envrc::locator::index_file(&f) {
            for l in locs {
                merged
                    .entry((l.subject.clone(), l.path.clone()))
                    .or_default()
                    .insert(format!("{}:{}", f.display(), l.key_line + 1));
            }
        }
    }

    // 3) registered external files
    for ef in external {
        if !ef.path.exists() {
            eprintln!(
                "warning: registered external file {} has vanished — skipping (re-run `dc load`; deregistration is not yet supported)",
                ef.path.display()
            );
            continue;
        }
        if let Ok(locs) = crate::envrc::locator::index_external(&ef.path) {
            for l in locs {
                merged
                    .entry((l.subject.clone(), l.path.clone()))
                    .or_default()
                    .insert(format!("{}:{}", ef.path.display(), l.key_line + 1));
            }
        }
    }

    Ok(merged)
}

// <REMOVED UUID HERE> run :: auto-generated pointer for public function run
pub fn run(json: bool, sources: bool) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let chain = crate::store::resolve::resolve_chain(&store);
    let meta = crate::store::meta::read_meta(&store)?;

    let merged = collect_scoped_keys(&chain, &meta.source, &meta.external)?;

    if merged.is_empty() {
        println!("No keys found in scope.");
        return Ok(());
    }

    if json && !sources {
        // Legacy shape: { subject: [keys...] }
        let mut all: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (s, k) in merged.keys() {
            all.entry(s.clone()).or_default().push(k.clone());
        }
        println!("{}", serde_json::to_string_pretty(&all)?);
        return Ok(());
    }
    if json {
        let arr: Vec<serde_json::Value> = merged
            .iter()
            .map(|((s, k), srcs)| {
                json!({
                    "subject": s,
                    "key": k,
                    "sources": srcs.iter().collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr)?);
        return Ok(());
    }

    let subjects: BTreeSet<String> = merged.keys().map(|(s, _)| s.clone()).collect();
    println!("{} key(s) across {} config(s)\n", merged.len(), subjects.len());

    let mut by_subject: BTreeMap<String, Vec<(&String, &Sources)>> = BTreeMap::new();
    for ((s, k), srcs) in &merged {
        by_subject.entry(s.clone()).or_default().push((k, srcs));
    }
    for (subject, keys) in &by_subject {
        println!("{subject}:");
        for (key, srcs) in keys {
            if sources {
                println!("  {key}  <- {}", srcs.iter().cloned().collect::<Vec<_>>().join(", "));
            } else {
                println!("  {key}");
            }
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn collect_leaf_keys_flat() {
        let val: Value = serde_yaml::from_str("a: 1\nb: hello\nc: true").unwrap();
        let mut keys = Vec::new();
        collect_leaf_keys(&val, "", &mut keys);
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    #[test]
    fn collect_leaf_keys_nested() {
        let val: Value = serde_yaml::from_str("db:\n  host: x\n  port: 5432\ntop: v").unwrap();
        let mut keys = Vec::new();
        collect_leaf_keys(&val, "", &mut keys);
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"db.host".to_string()));
        assert!(keys.contains(&"db.port".to_string()));
        assert!(keys.contains(&"top".to_string()));
    }

    #[test]
    fn store_layer_keys_covers_every_layer_not_just_secrets() {
        let tmp = TempDir::new().unwrap();
        let cfg = tmp.path().join("myapp");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(cfg.join("secrets.yaml"), "api_key: 🔒:v1:t0:tok\ndb:\n  password: 🔒:v1:t1:tok").unwrap();
        std::fs::write(cfg.join("base.yaml"), "host: localhost\nport: 5432").unwrap();
        std::fs::write(cfg.join(".active"), "ignored: not-a-yaml-layer-name").unwrap();

        let no_layers = tmp.path().join("other");
        std::fs::create_dir_all(&no_layers).unwrap();
        std::fs::write(no_layers.join("readme.txt"), "not yaml").unwrap();

        let result = store_layer_keys(tmp.path()).unwrap();
        assert_eq!(result.len(), 2, "secrets + base layers, config without yaml layers skipped");
        assert_eq!(result[0], ("myapp".to_string(), "base".to_string(), vec!["host".to_string(), "port".to_string()]));
        assert_eq!(result[1].0, "myapp".to_string(), "secrets layer second (sorted)");
        assert_eq!(result[1].1, "secrets");
        assert!(result[1].2.contains(&"api_key".to_string()));
        assert!(result[1].2.contains(&"db.password".to_string()));
    }

    #[test]
    fn collect_scoped_keys_merges_store_heredoc_and_external() {
        let tmp = TempDir::new().unwrap();
        let proj = std::fs::canonicalize({
            std::fs::create_dir_all(tmp.path().join("proj")).unwrap();
            tmp.path().join("proj")
        })
        .unwrap();

        // store: app/base + app/secrets
        let store = tmp.path().join("store");
        std::fs::create_dir_all(store.join("app")).unwrap();
        std::fs::write(store.join("app/base.yaml"), "host: h\n").unwrap();
        std::fs::write(store.join("app/secrets.yaml"), "api_key: 🔒:v1:t0:t\n").unwrap();

        // heredoc + external
        std::fs::write(
            proj.join(".envrc"),
            "dc_yaml --no-bump app <<'YAML'\nhost: heredoc\ninline_secret: \"🔒 s\"\nYAML\n",
        )
        .unwrap();
        let extfile = proj.join("team.yaml");
        std::fs::write(&extfile, "app:\n  db_password: \"🔒 d\"\n").unwrap();
        let external = vec![crate::store::meta::ExternalFile {
            path: extfile.clone(),
            mtime: 0,
            size: 0,
            layer: None,
            if_missing: false,
        }];

        let merged = collect_scoped_keys(std::slice::from_ref(&store), &proj, &external).unwrap();

        let srcs = |k: &str| {
            merged
                .iter()
                .find(|((_, key), _)| key == k)
                .map(|((_, _), s)| s.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_else(|| panic!("key {k} missing: {merged:?}"))
        };

        let host = srcs("host");
        assert!(host.contains(&"store:app/base".to_string()), "{host:?}");
        assert!(host.iter().any(|s| s.contains(".envrc:") && s.ends_with(":2")), "{host:?}");

        let dbp = srcs("db_password");
        assert_eq!(dbp, vec![format!("{}:2", extfile.display())], "{dbp:?}");

        let api = srcs("api_key");
        assert!(api.contains(&"store:app/secrets".to_string()), "{api:?}");

        // a vanished external warns but does not fail
        let gone = vec![crate::store::meta::ExternalFile {
            path: proj.join("gone.yaml"),
            mtime: 0,
            size: 0,
            layer: None,
            if_missing: false,
        }];
        let merged2 = collect_scoped_keys(std::slice::from_ref(&store), &proj, &gone).unwrap();
        assert!(merged2.iter().any(|((s, k), _)| s == "app" && k == "host"));
    }
}
