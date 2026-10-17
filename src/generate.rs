//! Shared secret generator behind `dc gen`, `dc get --auto`, and
//! `dc config set --generate`. One CSPRNG implementation (`rand::thread_rng`,
//! ChaCha12 seeded from the OS entropy pool) — every kind below draws from it
//! and nothing else; no kind ever prints or logs its output.
//!
//! Kinds (`dc config set --generate <KIND>`, `--length N` where noted):
//!
//! | kind | default | `--length` meaning | output |
//! |---|---|---|---|
//! | `password` | 32 | characters | URL/shell-safe alphanumeric `[A-Za-z0-9]` |
//! | `hex` | 32 | bytes (output is 2×) | lowercase hex chars |
//! | `base64` | 32 | bytes | standard base64, `=`-padded |
//! | `base64url` | 32 | bytes | URL-safe base64, unpadded |
//! | `elixir` / `secret-key-base` | 64 | characters (min 32) | byte-for-byte `mix phx.gen.secret` equivalent |
//! | `uuid` | — | rejected | random v4 UUID |
//! | `rsa` | — | rejected | 2048-bit PKCS#1 PEM private key, base64-encoded (one line) |
//!
//! `elixir` matches `mix phx.gen.secret` exactly:
//! `:crypto.strong_rand_bytes(length) |> Base.encode64(padding: false) |> binary_part(0, length)`
//! — `length` random bytes, standard base64 alphabet (`+`/`/` included, no
//! `=` padding), truncated to exactly `length` characters; default 64, and
//! lengths under 32 are rejected with Phoenix's own error text.

use anyhow::{bail, Result};
use rand::Rng;

/// Default `--length` per canonical kind (`usize::MAX` = no length applies).
pub(crate) fn default_length(kind: &str) -> usize {
    match kind {
        // phx.gen.secret generates 64 characters by default.
        "elixir" => 64,
        // Everything else with a length defaults to 32 (bytes where the kind
        // counts bytes, characters where it counts characters).
        _ => 32,
    }
}

/// Whether `--length N` is meaningful for this kind.
fn takes_length(kind: &str) -> bool {
    matches!(kind, "password" | "hex" | "base64" | "base64url" | "elixir")
}

/// Map a user-supplied kind name (case-sensitive, like the rest of dc's flag
/// values) to its canonical form. `secret-key-base` / `secret_key_base` are
/// aliases of `elixir`. Unknown names return `None`.
pub(crate) fn normalize_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "password" => Some("password"),
        "hex" => Some("hex"),
        "base64" => Some("base64"),
        "base64url" => Some("base64url"),
        "elixir" | "secret-key-base" | "secret_key_base" => Some("elixir"),
        "uuid" => Some("uuid"),
        "rsa" => Some("rsa"),
        _ => None,
    }
}

/// The `--generate <KIND>` help text — kinds plus defaults, kept in sync with
/// [`generate`] by the unit tests.
pub(crate) const KINDS_HELP: &str =
    "password | hex | base64 | base64url | elixir (alias: secret-key-base) | uuid | rsa";

/// Generate a fresh secret of `kind` (canonical or aliased). `length` is the
/// raw `--length` value — `None` selects the kind's default. Returns the
/// value and its canonical kind name (for the `generated <kind> …` output
/// line). The value is drawn exclusively from a CSPRNG and is the caller's
/// responsibility to keep out of stdout/stderr/logs.
pub(crate) fn generate(kind: &str, length: Option<usize>) -> Result<(String, &'static str)> {
    let canonical = normalize_kind(kind).ok_or_else(|| {
        anyhow::anyhow!("unknown --generate kind `{kind}` — expected one of: {KINDS_HELP}")
    })?;
    if length.is_some() && !takes_length(canonical) {
        bail!("--length does not apply to --generate {canonical}");
    }
    let n = length.unwrap_or_else(|| default_length(canonical));
    if n == 0 {
        bail!("--length must be at least 1");
    }
    Ok((generate_canonical(canonical, n)?, canonical))
}

fn generate_canonical(kind: &str, n: usize) -> Result<String> {
    let mut rng = rand::thread_rng();
    match kind {
        "hex" => {
            // n counts BYTES; the output is 2n lowercase hex chars (matches
            // `dc get --auto hex <bytes>`).
            let bytes: Vec<u8> = (0..n).map(|_| rng.gen()).collect();
            Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
        }
        "password" => {
            const CHARSET: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            Ok((0..n)
                .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
                .collect())
        }
        "base64" => {
            let bytes: Vec<u8> = (0..n).map(|_| rng.gen()).collect();
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
        }
        "base64url" => {
            let bytes: Vec<u8> = (0..n).map(|_| rng.gen()).collect();
            use base64::Engine;
            Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
        }
        "elixir" => {
            // phx.gen.secret guard: "The secret should be at least 32
            // characters long" (`random_string/1` has `when length > 31`).
            if n < 32 {
                bail!("The secret should be at least 32 characters long (mix phx.gen.secret requires it; pass --length N with N >= 32)");
            }
            Ok(phx_secret(&mut rng, n))
        }
        "uuid" => Ok(uuid_v4(&mut rng)),
        "rsa" => {
            let key = rsa::RsaPrivateKey::new(&mut rng, 2048)
                .map_err(|e| anyhow::anyhow!("rsa keygen failed: {e}"))?;
            use rsa::pkcs1::EncodeRsaPrivateKey;
            let pem = key
                .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
                .map_err(|e| anyhow::anyhow!("rsa PEM encoding failed: {e}"))?;
            // The PEM body is multi-line; the stored value is its base64 so
            // it fits one YAML scalar (`base64 -d` it back into a PEM file).
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD.encode(pem.as_bytes()))
        }
        _ => unreachable!("normalize_kind gates every call"),
    }
}

/// `mix phx.gen.secret` byte-for-byte: `length` CSPRNG bytes → standard
/// base64 without padding → keep the first `length` characters.
fn phx_secret(rng: &mut impl Rng, length: usize) -> String {
    let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let no_pad = encoded.trim_end_matches('=');
    no_pad[..length].to_string()
}

/// Random UUID v4 (no `uuid` crate: 16 CSPRNG bytes, version/variant bits
/// set, hyphenated lowercase hex).
fn uuid_v4(rng: &mut impl Rng) -> String {
    let mut b = [0u8; 16];
    rng.fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // RFC 4122 variant
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen(kind: &str) -> String {
        generate(kind, None).unwrap().0
    }

    #[test]
    fn password_is_alphanumeric_and_respects_length() {
        for len in [None, Some(16), Some(64), Some(1)] {
            let v = generate("password", len).unwrap().0;
            assert_eq!(v.len(), len.unwrap_or(32));
            assert!(
                v.chars().all(|c| c.is_ascii_alphanumeric()),
                "password charset must be [A-Za-z0-9]: {v}"
            );
        }
    }

    #[test]
    fn hex_counts_bytes_output_is_double_lowercase() {
        for (bytes, want) in [(32, 64), (8, 16), (1, 2)] {
            let v = generate("hex", Some(bytes)).unwrap().0;
            assert_eq!(v.len(), want);
            assert!(v
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        }
        assert_eq!(gen("hex").len(), 64, "default 32 bytes → 64 hex chars");
    }

    #[test]
    fn base64_default_is_32_bytes_padded_standard() {
        let v = gen("base64");
        assert_eq!(
            v.len(),
            44,
            "32 bytes → ceil(32/3)*4 = 44 chars with padding"
        );
        assert!(v.ends_with('='));
        assert!(v[..43]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/'));
        let len16 = generate("base64", Some(16)).unwrap().0;
        assert_eq!(len16.len(), 24);
    }

    #[test]
    fn base64url_is_unpadded_url_safe() {
        let v = gen("base64url");
        assert_eq!(v.len(), 43, "32 bytes → ceil(128/6) = 43 unpadded chars");
        assert!(!v.contains('='));
        assert!(v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn elixir_matches_phx_gen_secret_shape() {
        // Default 64; alphabet = standard base64 minus padding; exact length.
        for len in [None, Some(32), Some(64), Some(100), Some(128)] {
            let v = generate("elixir", len).unwrap().0;
            assert_eq!(v.len(), len.unwrap_or(64));
            assert!(
                v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/'),
                "elixir alphabet is standard base64 (no padding survives the truncation edge): {v}"
            );
        }
        // 32 is the floor (phoenix's `when length > 31` guard).
        let err = generate("elixir", Some(31)).unwrap_err().to_string();
        assert!(err.contains("at least 32"), "{err}");
    }

    #[test]
    fn secret_key_base_alias_matches_elixir() {
        let (_, kind) = generate("secret-key-base", None).unwrap();
        assert_eq!(kind, "elixir");
        let (_, kind) = generate("secret_key_base", None).unwrap();
        assert_eq!(kind, "elixir");
        assert_eq!(gen("secret-key-base").len(), 64);
    }

    #[test]
    fn uuid_is_v4_shape() {
        for _ in 0..20 {
            let v = gen("uuid");
            let parts: Vec<&str> = v.split('-').collect();
            assert_eq!(
                parts.iter().map(|p| p.len()).collect::<Vec<_>>(),
                vec![8, 4, 4, 4, 12],
                "uuid hyphenation: {v}"
            );
            assert!(v.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
            assert_eq!(parts[2].chars().next().unwrap(), '4', "version nibble: {v}");
            let variant = parts[3].chars().next().unwrap();
            assert!(
                matches!(variant, '8' | '9' | 'a' | 'b'),
                "variant nibble: {v}"
            );
        }
        assert!(
            generate("uuid", Some(8)).is_err(),
            "--length must be rejected for uuid"
        );
    }

    #[test]
    fn rsa_is_base64_of_a_pkcs1_pem() {
        use base64::Engine;
        let v = gen("rsa");
        assert!(
            generate("rsa", Some(1024)).is_err(),
            "--length must be rejected for rsa"
        );
        let pem = base64::engine::general_purpose::STANDARD
            .decode(&v)
            .unwrap();
        let pem = String::from_utf8(pem).unwrap();
        assert!(
            pem.starts_with("-----BEGIN RSA PRIVATE KEY-----\n"),
            "{pem}"
        );
        assert!(pem.ends_with("-----END RSA PRIVATE KEY-----\n"), "{pem}");
    }

    #[test]
    fn unknown_kind_lists_the_valid_kinds() {
        let err = generate("passphrase", None).unwrap_err().to_string();
        for kind in [
            "password",
            "hex",
            "base64",
            "base64url",
            "elixir",
            "uuid",
            "rsa",
        ] {
            assert!(err.contains(kind), "error must list {kind}: {err}");
        }
    }

    #[test]
    fn draws_are_fresh_and_lengths_stable() {
        // Same kind twice → different values (32-char CSPRNG draw: a
        // collision is not a realistic outcome).
        assert_ne!(gen("password"), gen("password"));
        assert_ne!(gen("uuid"), gen("uuid"));
        for _ in 0..10 {
            assert_eq!(gen("password").len(), 32);
        }
    }
}
