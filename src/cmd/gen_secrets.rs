use anyhow::Result;
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::io::Read;

/// Marker: 🎲 handle [🔒] [encoding] [length]
/// Examples:
///   "🎲 db_pass"                    → base64, 64 chars, unencrypted
///   "🎲 db_pass 🔒"                 → base64, 64 chars, encrypted
///   "🎲 db_pass 🔒 hex 32"          → hex, 32 chars, encrypted
///   "🎲 api_key 🔒 base128 128"     → base128 (alphanumeric), 128 chars, encrypted
///   "🎲 counter int 6"              → decimal digits, 6 chars, unencrypted

pub fn run(file: Option<&str>, stdin_flag: bool, inline: bool, tier: u8) -> Result<()> {
    let input = read_input(file, stdin_flag)?;

    let re = Regex::new(r#""🎲\s+([^\s"]+)(\s+[^"]*)?""#)?;
    let mut generated: HashMap<String, String> = HashMap::new();

    let result = re.replace_all(&input, |caps: &regex::Captures| {
        let handle = caps[1].to_string();
        let rest = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

        if let Some(existing) = generated.get(&handle) {
            return existing.clone();
        }

        let (encrypt, encoding, length) = parse_opts(rest);
        let plaintext = generate(&encoding, length);

        let output = if encrypt {
            match crate::settings::key() {
                Ok(key) => match crate::crypto::encode_token(&plaintext, tier, &key) {
                    Ok(token) => format!("\"{}\"", token),
                    Err(e) => {
                        eprintln!("dc: failed to encrypt handle {}: {}", handle, e);
                        format!("\"{}\"", plaintext)
                    }
                },
                Err(e) => {
                    eprintln!("dc: no encryption key available for handle {}: {}", handle, e);
                    format!("\"{}\"", plaintext)
                }
            }
        } else {
            format!("\"{}\"", plaintext)
        };

        generated.insert(handle, output.clone());
        output
    });

    if inline {
        let path = file.ok_or_else(|| anyhow::anyhow!("--inline requires --file"))?;
        std::fs::write(path, result.as_ref())?;
        eprintln!("dc: generated {} secret(s) in {}", generated.len(), path);
    } else {
        print!("{}", result);
    }

    Ok(())
}

fn read_input(file: Option<&str>, stdin_flag: bool) -> Result<String> {
    let piped = !std::io::IsTerminal::is_terminal(&std::io::stdin());
    let use_stdin = stdin_flag || (piped && file.is_none());

    if let Some(f) = file {
        Ok(std::fs::read_to_string(f)?)
    } else if use_stdin {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf.trim_end_matches('\n').to_string();
        Ok(buf)
    } else {
        anyhow::bail!("provide input with --file <PATH> or pipe via stdin")
    }
}

fn parse_opts(rest: &str) -> (bool, String, usize) {
    let mut encrypt = false;
    let mut encoding = String::from("base64");
    let mut length: usize = 64;

    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "🔒" => encrypt = true,
            "base64" | "base128" | "hex" | "int" => encoding = tokens[i].to_string(),
            s => {
                if let Ok(n) = s.parse::<usize>() {
                    length = n;
                }
            }
        }
        i += 1;
    }

    (encrypt, encoding, length)
}

fn generate(encoding: &str, length: usize) -> String {
    let mut rng = rand::thread_rng();

    match encoding {
        "hex" => {
            let byte_count = (length + 1) / 2;
            let bytes: Vec<u8> = (0..byte_count).map(|_| rng.gen()).collect();
            let s: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            s[..length].to_string()
        }
        "base64" => {
            let byte_count = (length * 3 + 3) / 4;
            let bytes: Vec<u8> = (0..byte_count).map(|_| rng.gen()).collect();
            use base64::Engine;
            let s = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes);
            s[..length].to_string()
        }
        "int" => {
            (0..length)
                .map(|_| rng.gen_range(0..10).to_string())
                .collect()
        }
        "base128" | _ => {
            const CHARSET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            (0..length)
                .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
                .collect()
        }
    }
}
