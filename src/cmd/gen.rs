use anyhow::Result;
use rand::Rng;

// ⟦𓋒𓌷𓋋𓂴⟧ run :: auto-generated pointer for public function run
pub fn run(hex: bool, base64_mode: bool, length: usize, raw: bool, tier: u8) -> Result<()> {
    let plaintext = generate(hex, base64_mode, length);

    if raw {
        print!("{}", plaintext);
    } else {
        let key = crate::settings::key()?;
        let token = crate::crypto::encode_token(&plaintext, tier, &key)?;
        println!("{}", token);
    }

    Ok(())
}

fn generate(hex: bool, base64_mode: bool, length: usize) -> String {
    let mut rng = rand::thread_rng();

    if hex {
        let byte_count = (length + 1) / 2;
        let bytes: Vec<u8> = (0..byte_count).map(|_| rng.gen()).collect();
        let s: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        s[..length].to_string()
    } else if base64_mode {
        let byte_count = (length * 3 + 3) / 4;
        let bytes: Vec<u8> = (0..byte_count).map(|_| rng.gen()).collect();
        use base64::Engine;
        let s = base64::engine::general_purpose::STANDARD.encode(&bytes);
        s[..length].to_string()
    } else {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        (0..length)
            .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
            .collect()
    }
}
