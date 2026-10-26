// 〚📁:da03f5d0-0e8f-5475-9b75-dfffadcdc3d0〛 direnv-config CLI entry point :: dc binary entry point: clap command tree and dispatch
mod audit;
mod cmd;
mod crypto;
mod envrc;
mod generate;
mod infisical;
mod keys;
mod kube;
mod secret;
mod secretcmp;
mod secretsmap;
mod settings;
mod shadow;
mod store;
mod target;
mod yaml;

use clap::{ArgGroup, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dc", version, about = "YAML-backed configuration layer for direnv")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Merge YAML from stdin into a named config
    Yaml {
        name: String,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        replace: bool,
        #[arg(long, value_name = "KEY")]
        replace_key: Option<String>,
        #[arg(long)]
        if_missing: bool,
        #[arg(long)]
        no_bump: bool,
    },
    /// Load external YAML config file(s) into the store (root keys = subjects)
    Load {
        /// Files to load, in order (later files win on conflict)
        #[arg(required = true)]
        files: Vec<String>,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        replace: bool,
        #[arg(long, value_name = "KEY")]
        replace_key: Option<String>,
        #[arg(long)]
        if_missing: bool,
        #[arg(long)]
        no_bump: bool,
    },
    /// Read a config value by path
    Get {
        name: String,
        path: Option<String>,
        #[arg(long)]
        raw: bool,
        /// Env var that overrides dc (env wins)
        #[arg(long, value_name = "ENV_VAR")]
        r#override: Option<String>,
        /// Env var fallback if dc misses (dc wins)
        #[arg(long, value_name = "ENV_VAR")]
        fallback: Option<String>,
        /// Auto-generate if all sources miss: password|hex [length]
        #[arg(long, value_name = "TYPE", num_args = 1..=2)]
        auto: Vec<String>,
        /// Static default if all sources miss (lower priority than --auto)
        #[arg(long, value_name = "VALUE")]
        default: Option<String>,
        /// Decrypt and print an encrypted secret value (audited); on a
        /// SECTION path, decrypt every encrypted leaf in the subtree
        /// (audited per leaf; output is the subtree YAML — --raw is a
        /// no-op there; without --reveal a section is printed unchanged)
        #[arg(long)]
        reveal: bool,
        /// Decrypt and copy the value to the clipboard instead of stdout (audited, macOS)
        #[arg(long)]
        clippy: bool,
        /// Decrypt a ⛔ shadowed/restricted secret (stricter than --reveal, audited)
        #[arg(long)]
        reveal_restricted: bool,
        /// Print the YAML structure of the subject/section with every leaf
        /// VALUE replaced by a marker — 🔒 encrypted leaf, <set> non-empty
        /// plaintext, <empty> empty/null. No value or ciphertext is ever
        /// printed and no decryption happens (works with no key available);
        /// a scalar path prints `path: <marker>`
        #[arg(long, alias = "redact", conflicts_with_all = ["reveal", "raw", "clippy", "reveal_restricted"])]
        keys: bool,
        /// With --keys: print one dotted path per leaf
        /// (`subject.a.b[i]<TAB>marker`) instead of YAML
        #[arg(long, alias = "paths", requires = "keys")]
        flat: bool,
    },
    /// Generate a random secret (encrypted by default)
    Gen {
        /// Output hex characters instead of alphanumeric
        #[arg(long, short = 'x')]
        hex: bool,
        /// Output base64 characters instead of alphanumeric
        #[arg(long, short = 'b')]
        base64: bool,
        /// Length of the generated secret (default: 64)
        #[arg(default_value = "64")]
        length: usize,
        /// Output raw plaintext instead of encrypted token
        #[arg(long)]
        raw: bool,
        /// Sensitivity tier 0-3 (default: 1)
        #[arg(long, short = 't', default_value = "1", value_parser = clap::value_parser!(u8).range(0..=3))]
        tier: u8,
    },
    /// Encrypt a plaintext value into a 🔒:v1 token
    Encrypt {
        /// The plaintext value to encrypt
        #[arg(long)]
        value: Option<String>,
        /// Read plaintext from a file
        #[arg(long, value_name = "FILE")]
        from: Option<String>,
        /// Read plaintext from stdin
        #[arg(long)]
        stdin: bool,
        /// Sensitivity tier 0-1 (default: 0)
        #[arg(long, short = 't', default_value = "0", value_parser = clap::value_parser!(u8).range(0..=1))]
        tier: u8,
        /// Scan input for "🔒<plaintext>" placeholders and encrypt them in place
        #[arg(long)]
        parsed: bool,
        /// With --parsed --from, overwrite the file in place instead of printing to stdout
        #[arg(long, requires = "parsed")]
        inline: bool,
    },
    /// Decrypt a 🔒:v1 token and print its plaintext
    Decrypt {
        /// The encrypted token (🔒:v1:t<tier>:<payload>), or pipe via stdin
        token: Option<String>,
    },
    /// Set a config value (stored as an encrypted secret unless --plain)
    Set {
        name: String,
        key: String,
        /// Value to store — omit it and pass --stdin to read from stdin
        value: Option<String>,
        /// Read the value from stdin instead of the positional VALUE
        #[arg(long)]
        stdin: bool,
        /// Treat the value as a secret (the default): encrypt it for the
        /// store, or store an existing dc token verbatim
        #[arg(long, conflicts_with = "plain")]
        secret: bool,
        /// Store the value exactly as given — no secret handling
        #[arg(long)]
        plain: bool,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        no_bump: bool,
    },
    /// Remove a key from a named config
    Unset {
        name: String,
        keys: Vec<String>,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        no_bump: bool,
    },
    /// Remove named configs or branches within them
    Prune {
        name: String,
        keys: Vec<String>,
        #[arg(long)]
        layer: Option<String>,
        #[arg(long)]
        no_bump: bool,
    },
    /// Export resolved config as shell env vars
    Env {
        #[arg(long)]
        list: bool,
        #[arg(long)]
        diff: bool,
    },
    /// Bump the version counter
    Bump,
    /// Initialize a config store for the current directory
    Init {
        #[arg(long)]
        from_envrc: Option<String>,
    },
    /// Show current config state
    Status,
    /// List all known config stores
    List,
    /// Permanently delete a named config (or the entire store if no name given)
    Purge {
        /// Config name to purge (omit to purge entire store)
        name: Option<String>,
        /// Hard delete — remove files without writing tombstones
        #[arg(long)]
        hard: bool,
    },
    /// List every key name in scope for this project (merged across .envrc*
    /// dc_yaml blocks, registered external files, and store layers) — values
    /// are never printed
    Secrets {
        #[arg(long)]
        json: bool,
        /// Annotate each key with its source(s) (`file:line` or `store:<subject>/<layer>`)
        #[arg(long)]
        sources: bool,
    },
    /// Browse/search config entries with secrets masked (pipes to bat if available)
    Bat {
        /// Config subject (omit with --all)
        subject: Option<String>,
        /// Optional dot-path scope within the subject
        scope: Option<String>,
        /// Render every config in the resolved chain
        #[arg(long)]
        all: bool,
        /// Decrypt and show secret values (audited)
        #[arg(long)]
        reveal: bool,
        /// Flat output: one line per key with line number, no values
        #[arg(long)]
        flat: bool,
        /// Keep only entries whose full key path matches this regex
        #[arg(long, value_name = "RE")]
        filter_key: Option<String>,
        /// Keep only entries whose value matches this regex
        #[arg(long, value_name = "RE")]
        filter_value: Option<String>,
        /// Drop entries whose full key path matches this regex
        #[arg(long, value_name = "RE")]
        exclude_key: Option<String>,
        /// Drop entries whose value matches this regex
        #[arg(long, value_name = "RE")]
        exclude_value: Option<String>,
    },
    /// Output completion candidates for purge (hidden)
    #[command(name = "__complete-purge", hide = true)]
    CompletePurge,
    /// Locate and edit secrets in source .envrc* files
    Config {
        #[command(subcommand)]
        sub: ConfigCmd,
    },
    /// Compare a local secret against remote targets (no value output)
    Compare {
        layer: String,
        path: String,
        #[arg(long = "to", value_name = "TARGET")]
        to: Vec<String>,
    },
    /// Force-push local secret(s) to a remote (infisical:// or kubernetes://)
    Push {
        subject: String,
        path: String,
        #[arg(long = "to", value_name = "TARGET")]
        to: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Scan for 🎲 placeholders and generate secrets in place
    #[command(name = "gen-secrets")]
    GenSecrets {
        /// Input file containing 🎲 placeholders
        #[arg(long, value_name = "FILE")]
        file: Option<String>,
        /// Read from stdin
        #[arg(long)]
        stdin: bool,
        /// Overwrite --file in place instead of printing to stdout
        #[arg(long)]
        inline: bool,
        /// Sensitivity tier for encrypted secrets (default: 1)
        #[arg(long, short = 't', default_value = "1", value_parser = clap::value_parser!(u8).range(0..=3))]
        tier: u8,
    },
    /// Infisical roundtripping (secrets-map driven)
    Infisical {
        #[command(subcommand)]
        sub: InfisicalCmd,
    },
    /// Manage the external, lockable encryption-key file
    Keys {
        #[command(subcommand)]
        sub: KeysCmd,
    },
    /// Emit a loadable YAML secret file from the store's subjects (the inverse
    /// of `dc load`). Values are never printed; output paths must be covered
    /// by .gitignore unless --force. 🔒 tokens re-emit byte-identical.
    Emit {
        /// Output file (default secrets.dc.yaml); layer variants gain
        /// `.<layer>.yaml`, special subjects (`_dc`, `auto`) their own files
        #[arg(long, value_name = "FILE")]
        output: Option<String>,
        /// Only emit these subject(s) (repeatable)
        #[arg(long, value_name = "SUBJECT")]
        subject: Vec<String>,
        /// Print a summary (subject/key counts, target paths) without writing
        #[arg(long)]
        dry_run: bool,
        /// Write even when the target is git-tracked or not .gitignore-covered
        #[arg(long)]
        force: bool,
    },
    /// Encryption inventory + in-place (un)seal over project-scope sources.
    /// Interactive when stdin is a tty; store layer files are never rewritten.
    // Parser-level mutual exclusion: the four action flags are two groups
    // (encrypt / decrypt) that conflict with each other, so contradictory
    // combinations die at the parser with usage help (the cmd::seal check
    // stays as a runtime guard for direct construction).
    #[command(group(
        ArgGroup::new("seal_encrypt_action")
            .args(["encrypt", "encrypt_all_plain"])
            .conflicts_with("seal_decrypt_action")
    ))]
    #[command(group(
        ArgGroup::new("seal_decrypt_action")
            .args(["decrypt", "decrypt_all"])
    ))]
    Seal {
        /// Print the inventory (key | file:line | state) and exit
        #[arg(long)]
        list: bool,
        /// Machine-readable JSON (with --list)
        #[arg(long)]
        json: bool,
        /// Non-interactive mode: requires an --encrypt/--decrypt action
        #[arg(long)]
        headless: bool,
        /// Headless: seal these key(s) (`subject.path`, repeatable)
        #[arg(long = "encrypt", value_name = "KEY", num_args = 1..)]
        encrypt: Vec<String>,
        /// Headless: seal every plain entry in scope
        #[arg(long)]
        encrypt_all_plain: bool,
        /// Headless: unseal these key(s) (`subject.path`, repeatable)
        #[arg(long = "decrypt", value_name = "KEY", num_args = 1..)]
        decrypt: Vec<String>,
        /// Headless: unseal every sealed entry in scope
        #[arg(long)]
        decrypt_all: bool,
        /// Selector: keep only entries whose leaf key name matches this
        /// case-insensitive wildcard — `*` = any run of characters (incl.
        /// none), `?` = exactly one, all else literal; the whole leaf name
        /// (last dot-segment, e.g. `db_password` of `app.db.password`) must
        /// match. Repeatable (OR). Applies to every mode; --except wins.
        #[arg(long = "match", value_name = "PAT")]
        r#match: Vec<String>,
        /// Selector: drop entries whose leaf key name matches this wildcard
        /// (same syntax as --match). Repeatable (OR); always wins over --match.
        #[arg(long, value_name = "PAT")]
        except: Vec<String>,
        /// Restrict scope to entries in this file
        #[arg(long, value_name = "FILE")]
        file: Option<String>,
        /// Preview the changes without writing
        #[arg(long)]
        dry_run: bool,
        /// Apply the changes without prompting
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum KeysCmd {
    /// Report which key mode the configuration is in (external key file vs legacy settings.yaml)
    Status,
    /// Move the legacy `key:` from settings.yaml into the key file, verifying every
    /// store token decrypts via the new file before stripping key material
    Migrate {
        /// Store files to verify (default: ./.envrc.dc)
        stores: Vec<String>,
        /// Only strip the legacy `key:` from settings.yaml (key file already exists)
        #[arg(long)]
        strip_only: bool,
    },
    /// Print (and with --yes run) root-ownership hardening for the key file
    Lock {
        /// Execute the hardening commands via sudo
        #[arg(long)]
        yes: bool,
    },
    /// Reverse key-file hardening (back to 0600 user-owned)
    Unlock {
        /// Execute the unlock commands via sudo
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum InfisicalCmd {
    /// Compare a mapped Infisical secret's local value against targets
    Compare {
        /// Infisical path/NAME (e.g. /data/postgres/POSTGRES_PASSWORD)
        path: String,
        #[arg(long = "to", value_name = "TARGET")]
        to: Vec<String>,
    },
    /// Show live Infisical value next to its dc source (redacted unless --reveal)
    Get {
        name: String,
        #[arg(long)]
        reveal: bool,
    },
    /// Set the dc source for an Infisical secret NAME (edits .envrc in place)
    Set {
        name: String,
        #[arg(long)]
        value: Option<String>,
        #[arg(long, value_name = "FILE")]
        from: Option<String>,
        #[arg(long)]
        stdin: bool,
        #[arg(long)]
        encrypted: bool,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Find where a secret is set in .envrc* files
    Get { subject: String, path: String },
    /// Remove a leaf key or a whole branch from the SOURCE file that holds it
    /// (and from the store layers). Prints key paths and child key names only.
    #[command(alias = "delete")]
    Unset {
        subject: String,
        path: String,
        /// Force the source file (registered external file or .envrc*)
        #[arg(long, value_name = "FILE")]
        file: Option<String>,
        /// Apply without the confirmation prompt
        #[arg(long)]
        yes: bool,
        /// Show what would be removed; write nothing
        #[arg(long)]
        dry_run: bool,
        /// Also remove parents left empty by the removal
        #[arg(long)]
        prune_empty: bool,
    },
    /// Encrypt and write a secret value into its .envrc* source line
    Set {
        subject: String,
        path: String,
        #[arg(long)]
        value: Option<String>,
        #[arg(long, value_name = "FILE")]
        from: Option<String>,
        #[arg(long)]
        stdin: bool,
        /// The provided value is already an encrypted 🔒:v1 token
        #[arg(long)]
        encrypted: bool,
        /// Store the value as literal plaintext — no encryption, no 🔒 marker
        /// (overwriting an encrypted value also drops its tier marker)
        #[arg(long, conflicts_with = "encrypted")]
        plain: bool,
        /// Store the value as an unquoted YAML number (e.g. `bip: 4`) —
        /// implies plaintext, no marker (numbers are never encrypted;
        /// rejects non-finite/non-numeric input)
        #[arg(long, conflicts_with = "encrypted")]
        numeric: bool,
        /// Store the value as an unquoted YAML boolean (`flag: true`) —
        /// implies plaintext, no marker; accepts `true`/`false`
        /// case-insensitively, rejects everything else
        #[arg(long = "bool", conflicts_with_all = ["encrypted", "numeric"])]
        bool_flag: bool,
        #[arg(long)]
        yes: bool,
        /// Force the destination file (registered external file or .envrc*)
        #[arg(long, value_name = "FILE")]
        file: Option<String>,
        /// Create every missing section of PATH as nested YAML mappings —
        /// the full missing path is built under the deepest existing
        /// ancestor (existing sections are reused; a path segment that
        /// already holds a scalar value is refused)
        #[arg(short = 'p', long)]
        parents: bool,
        /// With -p: insert the new top-most section above this sibling path
        #[arg(long, value_name = "PATH")]
        above: Option<String>,
        /// With -p: insert the new top-most section below this sibling path
        #[arg(long, value_name = "PATH")]
        below: Option<String>,
        /// With -p: place the new top-most section alphabetically among its
        /// siblings (the default with -p)
        #[arg(long)]
        auto: bool,
        /// Generate a fresh secret of KIND and store it instead of reading a
        /// value (mutually exclusive with --value/--from/--stdin and with
        /// --encrypted/--numeric/--bool; encrypted at rest by default,
        /// respects --plain, -p and placement). KIND is one of:
        ///
        ///   password      — URL/shell-safe alphanumeric [A-Za-z0-9]; default 32 chars
        ///
        ///   hex           — lowercase hex; default 32 bytes → 64 chars; N counts bytes
        ///
        ///   base64        — standard base64, '='-padded; default 32 bytes; N counts bytes
        ///
        ///   base64url     — URL-safe base64, unpadded; default 32 bytes; N counts bytes
        ///
        ///   elixir        — `mix phx.gen.secret` equivalent: standard-base64 alphabet, no padding, truncated to exactly N chars; default 64, minimum 32 (alias: secret-key-base)
        ///
        ///   uuid          — random v4 UUID
        ///
        ///   rsa           — 2048-bit PKCS#1 PEM private key, base64-encoded onto one line (decode with `base64 -d`); no --length
        ///
        /// The generated value is never printed — the output line says
        /// `generated <kind> (<n> chars) → <subject> <path>`.
        #[arg(
            long,
            value_name = "KIND",
            conflicts_with_all = ["value", "from", "stdin", "encrypted", "numeric", "bool_flag"]
        )]
        generate: Option<String>,
        /// Length for --generate (defaults per kind: password/elixir count
        /// characters — elixir floors at 32; hex/base64/base64url count
        /// bytes; rejected for uuid/rsa)
        #[arg(long, value_name = "N", requires = "generate")]
        length: Option<usize>,
        /// With --generate: skip when the leaf already holds a non-empty
        /// value — output says `skipped` instead of `generated`
        #[arg(long, requires = "generate")]
        if_missing: bool,
    },
    /// Insert a YAML section (from stdin) above/below an anchor entry
    Setall {
        subject: String,
        /// Anchor path to insert above
        #[arg(long, value_name = "PATH")]
        above: Option<String>,
        /// Anchor path to insert below
        #[arg(long, value_name = "PATH")]
        below: Option<String>,
        #[arg(long)]
        yes: bool,
        /// Force the destination file (registered external file or .envrc*)
        #[arg(long, value_name = "FILE")]
        file: Option<String>,
    },
    /// Move a secret into the shadow store, leaving a ⛔ sentinel behind
    Secure {
        subject: String,
        path: String,
        #[arg(long)]
        note: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Yaml { name, layer, replace, replace_key, if_missing, no_bump } => {
            cmd::yaml::run(&name, layer.as_deref(), replace, replace_key.as_deref(), if_missing, no_bump)
        }
        Commands::Load { files, layer, replace, replace_key, if_missing, no_bump } => {
            cmd::load::run(&files, layer.as_deref(), replace, replace_key.as_deref(), if_missing, no_bump)
        }
        Commands::Get { name, path, raw, r#override, fallback, auto, default, reveal, clippy, reveal_restricted, keys, flat } => {
            cmd::get::run(&name, path.as_deref(), raw, r#override.as_deref(), fallback.as_deref(), &auto, default.as_deref(), reveal, clippy, reveal_restricted, keys, flat)
        }
        Commands::Gen { hex, base64, length, raw, tier } => {
            cmd::gen::run(hex, base64, length, raw, tier)
        }
        Commands::Encrypt { value, from, stdin, tier, parsed, inline } => {
            cmd::encrypt::run(value.as_deref(), from.as_deref(), stdin, tier, parsed, inline)
        }
        Commands::Decrypt { token } => {
            cmd::decrypt::run(token.as_deref())
        }
        Commands::Set { name, key, value, stdin, secret, plain, layer, no_bump } => {
            cmd::set::run(&name, &key, value.as_deref(), stdin, secret, plain, layer.as_deref(), no_bump)
        }
        Commands::Unset { name, keys, layer, no_bump } => {
            cmd::unset::run(&name, &keys, layer.as_deref(), no_bump)
        }
        Commands::Prune { name, keys, layer, no_bump } => {
            cmd::prune::run(&name, &keys, layer.as_deref(), no_bump)
        }
        Commands::Env { list, diff } => {
            cmd::env::run(list, diff)
        }
        Commands::Bump => {
            cmd::bump::run()
        }
        Commands::Init { from_envrc } => {
            cmd::init::run(from_envrc.as_deref())
        }
        Commands::Status => {
            cmd::status::run()
        }
        Commands::List => {
            cmd::list::run()
        }
        Commands::Secrets { json, sources } => {
            cmd::secrets::run(json, sources)
        }
        Commands::Bat { subject, scope, all, reveal, flat, filter_key, filter_value, exclude_key, exclude_value } => {
            cmd::bat::run(
                subject.as_deref(),
                scope.as_deref(),
                all,
                reveal,
                flat,
                filter_key.as_deref(),
                filter_value.as_deref(),
                exclude_key.as_deref(),
                exclude_value.as_deref(),
            )
        }
        Commands::Purge { name, hard } => {
            cmd::purge::run(name.as_deref(), hard)
        }
        Commands::CompletePurge => {
            cmd::purge::completions()
        }
        Commands::Config { sub } => match sub {
            ConfigCmd::Get { subject, path } => cmd::config::get(&subject, &path),
            ConfigCmd::Set { subject, path, value, from, stdin, encrypted, plain, numeric, bool_flag, yes, file, parents, above, below, auto, generate, length, if_missing } => {
                let spots = above.is_some() as u8 + below.is_some() as u8 + auto as u8;
                if spots > 1 {
                    anyhow::bail!("use at most one of --above <PATH>, --below <PATH>, --auto");
                }
                if !parents && spots == 1 {
                    anyhow::bail!(
                        "--above/--below/--auto require -p/--parents (when every parent already exists they are no-ops — drop the flag)"
                    );
                }
                let placement = if let Some(a) = above {
                    cmd::config::Placement::Above(a)
                } else if let Some(b) = below {
                    cmd::config::Placement::Below(b)
                } else {
                    cmd::config::Placement::Auto
                };
                cmd::config::set(
                    &subject,
                    &path,
                    value.as_deref(),
                    from.as_deref(),
                    stdin,
                    encrypted,
                    plain,
                    numeric,
                    bool_flag,
                    yes,
                    file.as_deref(),
                    parents,
                    &placement,
                    generate.as_deref(),
                    length,
                    if_missing,
                )
            }
            ConfigCmd::Unset { subject, path, file, yes, dry_run, prune_empty } => {
                cmd::config::unset(&subject, &path, file.as_deref(), yes, dry_run, prune_empty)
            }
            ConfigCmd::Setall { subject, above, below, yes, file } => {
                let (anchor, is_below) = match (above, below) {
                    (Some(a), None) => (a, false),
                    (None, Some(b)) => (b, true),
                    _ => anyhow::bail!("provide exactly one of --above <PATH> or --below <PATH>"),
                };
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                cmd::config::setall(&subject, &buf, &subject, &anchor, is_below, yes, file.as_deref())
            }
            ConfigCmd::Secure { subject, path, note } => {
                shadow::secure(&subject, &path, note.as_deref())
            }
        },
        Commands::Compare { layer, path, to } => {
            cmd::compare::run(&layer, &path, &to)
        }
        Commands::Push { subject, path, to, dry_run, yes } => {
            cmd::push::run(&subject, &path, &to, dry_run, yes)
        }
        Commands::GenSecrets { file, stdin, inline, tier } => {
            cmd::gen_secrets::run(file.as_deref(), stdin, inline, tier)
        }
        Commands::Infisical { sub } => match sub {
            InfisicalCmd::Compare { path, to } => cmd::infisical::compare(&path, &to),
            InfisicalCmd::Get { name, reveal } => cmd::infisical::get(&name, reveal),
            InfisicalCmd::Set { name, value, from, stdin, encrypted, yes } => {
                cmd::infisical::set(&name, value.as_deref(), from.as_deref(), stdin, encrypted, yes)
            }
        },
        Commands::Keys { sub } => match sub {
            KeysCmd::Status => cmd::keys::run_status(),
            KeysCmd::Migrate { stores, strip_only } => cmd::keys::run_migrate(&stores, strip_only),
            KeysCmd::Lock { yes } => cmd::keys::run_lock(yes),
            KeysCmd::Unlock { yes } => cmd::keys::run_unlock(yes),
        },
        Commands::Emit { output, subject, dry_run, force } => {
            cmd::emit::run(cmd::emit::EmitArgs {
                output: output.as_deref(),
                subjects: &subject,
                dry_run,
                force,
            })
        }
        Commands::Seal { list, json, headless, encrypt, encrypt_all_plain, decrypt, decrypt_all, r#match, except, file, dry_run, yes } => {
            cmd::seal::run(cmd::seal::SealArgs {
                list,
                json,
                headless,
                encrypt,
                encrypt_all_plain,
                decrypt,
                decrypt_all,
                r#match,
                except,
                file: file.as_deref(),
                dry_run,
                yes,
            })
        }
    }
}
