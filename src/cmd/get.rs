use anyhow::Result;

pub fn run(name: &str, path: Option<&str>, raw: bool) -> Result<()> {
    let store = crate::store::find_current_store()?;
    let chain = crate::store::resolve::resolve_chain(&store);
    let config = crate::store::resolve::resolve_config(&chain, name)?;

    match path {
        Some(p) => {
            match crate::yaml::path::get_path(&config, p) {
                Some(val) => {
                    if raw {
                        print!("{}", serde_yaml::to_string(&val)?);
                    } else {
                        match val {
                            serde_yaml::Value::String(s) => println!("{}", s),
                            serde_yaml::Value::Number(n) => println!("{}", n),
                            serde_yaml::Value::Bool(b) => println!("{}", b),
                            serde_yaml::Value::Null => println!(),
                            _ => print!("{}", serde_yaml::to_string(&val)?),
                        }
                    }
                }
                None => {
                    std::process::exit(1);
                }
            }
        }
        None => {
            print!("{}", serde_yaml::to_string(&config)?);
        }
    }

    Ok(())
}
