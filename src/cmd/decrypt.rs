use anyhow::{anyhow, Result};
use std::io::Read;

// ⟦𓃤𓎈𓎏𓋢⟧ run :: auto-generated pointer for public function run
pub fn run(token: Option<&str>) -> Result<()> {
    let token = match token {
        Some(t) => t.to_string(),
        None => {
            if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                anyhow::bail!("provide a token argument or pipe one via stdin");
            }
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf.trim().to_string()
        }
    };
    let token = token.trim_matches('"').to_string();
    if !crate::crypto::is_encrypted(&token) {
        return Err(anyhow!("not a dc encrypted token (expected 🔒:v1:... prefix)"));
    }
    let key = crate::settings::key()?;
    let (_tier, plaintext) = crate::crypto::decode_token(&token, &key)?;
    print!("{}", plaintext);
    Ok(())
}
