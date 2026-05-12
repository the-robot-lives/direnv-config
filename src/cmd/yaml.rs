use anyhow::Result;
use std::io::Read;

pub fn run(
    name: &str,
    layer: Option<&str>,
    replace: bool,
    replace_key: Option<&str>,
    if_missing: bool,
    no_bump: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let store = crate::store::ensure_store(&cwd)?;
    crate::store::ensure_config(&store, name)?;

    let layer_name = layer.unwrap_or("base");
    let layer_file = crate::store::layout::layer_path(&store, name, layer_name);

    if if_missing && layer_file.exists() {
        return Ok(());
    }

    let mut stdin_buf = String::new();
    std::io::stdin().read_to_string(&mut stdin_buf)?;
    let input: serde_yaml::Value = serde_yaml::from_str(&stdin_buf)?;

    if replace {
        let yaml_str = serde_yaml::to_string(&input)?;
        std::fs::write(&layer_file, yaml_str)?;
    } else if let Some(rk) = replace_key {
        // Replace just one branch: load existing, replace that key, write back
        let mut existing = if layer_file.exists() {
            let content = std::fs::read_to_string(&layer_file)?;
            serde_yaml::from_str(&content).unwrap_or(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()))
        } else {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        };

        if let (serde_yaml::Value::Mapping(ref mut emap), serde_yaml::Value::Mapping(ref imap)) = (&mut existing, &input) {
            // Get the replacement value for the key from input
            let key_val = serde_yaml::Value::String(rk.to_string());
            if let Some(new_val) = imap.get(&key_val) {
                emap.insert(key_val, new_val.clone());
            }
        }
        let yaml_str = serde_yaml::to_string(&existing)?;
        std::fs::write(&layer_file, yaml_str)?;
    } else {
        // Deep merge
        let existing = if layer_file.exists() {
            let content = std::fs::read_to_string(&layer_file)?;
            serde_yaml::from_str(&content).unwrap_or(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()))
        } else {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        };

        let merged = crate::yaml::merge::deep_merge(&existing, &input);
        let yaml_str = serde_yaml::to_string(&merged)?;
        std::fs::write(&layer_file, yaml_str)?;
    }

    // Resolve active for this config
    crate::store::resolve::resolve_active(&store, name)?;
    crate::store::meta::update_configs_list(&store)?;

    if !no_bump {
        crate::store::bump_version(&store)?;
    }

    Ok(())
}
