use anyhow::{Context, Result};

pub fn run(name: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let store = crate::store::layout::store_path(&cwd);

    if !store.exists() {
        anyhow::bail!("no store found for {}. Run `dc init` first.", cwd.display());
    }

    match name {
        Some(n) => {
            let config_dir = crate::store::layout::config_dir(&store, n);
            if !config_dir.exists() {
                anyhow::bail!("config '{}' does not exist in this store", n);
            }
            std::fs::remove_dir_all(&config_dir)
                .with_context(|| format!("failed to remove config dir: {}", config_dir.display()))?;
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
