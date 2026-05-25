use anyhow::{Context, Result};

fn config_exists_in_parent(store: &std::path::Path, name: &str) -> bool {
    let chain = crate::store::resolve::resolve_chain(store);
    // Check all stores except the last one (which is the current store)
    chain.iter().rev().skip(1).any(|s| {
        crate::store::layout::active_path(s, name).exists()
    })
}

pub fn run(name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let store = crate::store::layout::store_path(&cwd);

    if !store.exists() {
        anyhow::bail!("no store found for {}. Run `dc init` first.", cwd.display());
    }

    match name {
        Some(n) => {
            let config_dir = crate::store::layout::config_dir(&store, n);
            let exists_locally = config_dir.exists();
            let exists_in_parent = config_exists_in_parent(&store, n);

            if !exists_locally && !exists_in_parent {
                anyhow::bail!("config '{}' does not exist in this store", n);
            }

            if exists_locally {
                std::fs::remove_dir_all(&config_dir)
                    .with_context(|| format!("failed to remove config dir: {}", config_dir.display()))?;
            }

            if exists_in_parent {
                // Write tombstone as a layer file so resolve_active produces it in .active
                crate::store::layout::ensure_config(&store, n)?;
                let layer = crate::store::layout::layer_path(&store, n, "base");
                std::fs::write(&layer, "_dc_pruned: true\n")
                    .with_context(|| format!("failed to write tombstone: {}", layer.display()))?;
                crate::store::resolve::resolve_active(&store, n)?;
            }

            crate::store::meta::update_configs_list(&store)?;
            eprintln!("purged config '{}'", n);
        }
        None => {
            std::fs::remove_dir_all(&store)
                .with_context(|| format!("failed to remove store: {}", store.display()))?;
            eprintln!("purged store for {}", cwd.display());
        }
    }

    Ok(())
}

pub fn completions() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let store = crate::store::layout::store_path(&cwd);
    if !store.exists() {
        return Ok(());
    }
    let meta = crate::store::meta::read_meta(&store)?;
    for name in &meta.configs {
        println!("{}", name);
    }
    Ok(())
}
