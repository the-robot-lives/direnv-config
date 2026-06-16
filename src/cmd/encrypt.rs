use anyhow::Result;
use regex::Regex;
use std::io::Read;

fn read_input(value: Option<&str>, from: Option<&str>, stdin_flag: bool) -> Result<String> {
    let piped = !std::io::IsTerminal::is_terminal(&std::io::stdin());
    let stdin_flag = stdin_flag || (piped && value.is_none() && from.is_none());

    let sources = [value.is_some(), from.is_some(), stdin_flag]
        .iter()
        .filter(|b| **b)
        .count();
    if sources == 0 {
        anyhow::bail!("provide input with --value, --from <FILE>, or --stdin (or pipe)");
    }
    if sources > 1 {
        anyhow::bail!("use only one of --value / --from / --stdin (or pipe)");
    }

    if let Some(v) = value {
        Ok(v.to_string())
    } else if let Some(f) = from {
        Ok(std::fs::read_to_string(f)?)
    } else {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    }
}

pub fn run(
    value: Option<&str>,
    from: Option<&str>,
    stdin_flag: bool,
    tier: u8,
    parsed: bool,
    inline: bool,
) -> Result<()> {
    let input = read_input(value, from, stdin_flag)?;

    if parsed {
        run_parsed(&input, from, tier, inline)
    } else {
        let plaintext = input.trim_end_matches('\n');
        let key = crate::settings::key()?;
        let token = crate::crypto::encode_token(plaintext, tier, &key)?;
        println!("\"{}\"", token);
        Ok(())
    }
}

fn run_parsed(input: &str, from: Option<&str>, tier: u8, inline: bool) -> Result<()> {
    let key = crate::settings::key()?;
    let re = Regex::new(r#""🔒([^"]*)""#)?;

    let result = re.replace_all(input, |caps: &regex::Captures| {
        let inner = &caps[1];
        if inner.starts_with(":v1:") || inner.starts_with(":dcenc:") {
            caps[0].to_string()
        } else if crate::crypto::is_encrypted(&format!("🔒{inner}")) {
            caps[0].to_string()
        } else {
            match crate::crypto::encode_token(inner, tier, &key) {
                Ok(token) => format!("\"{}\"", token),
                Err(_) => caps[0].to_string(),
            }
        }
    });

    if inline {
        let path = from.ok_or_else(|| anyhow::anyhow!("--inline requires --from <FILE>"))?;
        std::fs::write(path, result.as_ref())?;
    } else {
        print!("{}", result);
    }
    Ok(())
}
