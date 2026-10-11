# direnv-config — Landing Copy Deck

Page: single route `/` at **dirconfig.therobotlives.com** · implemented in `web/` (Elixir Hologram)
Sources of truth: repo `README.md`, `docs/PROJ-ARCH.summary.md`, `docs/PROJ-SCHEMA.summary.md`,
`docs/THREAT-MODEL.summary.md`, `docs/TEST-HEALTH.md`, `demo/README.md`, `src/main.rs` (Commands
enum), `Makefile`, `CHANGELOG.md`, `Cargo.toml`, `LICENSE` (MIT) — every claim below is
repo-verifiable as of commit `3190560` (develop, 2026-10-12).

**Design note (bypass):** no hero art / media-generation mockup loop — mirrors the run-claude
landing: example-led typography IS the design, dark-first, monospace-forward. The hero's visual is
a real install sequence and the README's own Why table, not an illustration.

**Scrub note:** page examples use neutral `acme`/`example.com`-style names. The README's Getting
Started example (`name: noizu`, `~/.kube/noizu/config`) and demo fixtures (internal hostnames) are
never lifted verbatim. No secret values appear anywhere.

**Dev note:** dev/test builds need `HOLOGRAM_START=1` — without it the Hologram compiler
no-ops (no `priv/static/hologram` bundles) and `/` 404s. Use `HOLOGRAM_START=1 mix compile`
and `HOLOGRAM_START=1 mix test`. Prod pins `HOLOGRAM_ENV=prod` in the Dockerfile (same gate
as every other Hologram site in the fleet).

---

## 1. JTBD summary

| Question | Answer |
|---|---|
| Trigger | A dev's `.envrc` has grown past 50 export lines, mixes secrets with plain config, has no dev/prod layering — and child processes (scripts, agents, other tabs) can't feed state back to the shell that spawned them. |
| Job | Keep direnv's automatic per-directory env, but declare config as structured, layered, versioned YAML — and let any process read or write it. |
| Today's alternative | Hand-managed `export VAR=value` `.envrc` files; dotenv plus a secrets manager glued together by hand; or a hosted vault that only holds secrets while the rest of project config stays in shell strings. |
| What's wrong with that | Flat strings with no grouping or hierarchy; secrets sitting next to plain config; no history of what changed; and a child process cannot update its parent's environment — env vars are write-once from the parent's view. |
| Switch trigger | The third time a deploy script has to print "now run `export FOO=bar` and restart" — or the first secret committed to git because it lived in `.envrc`. |
| Objections | "Is this just `.env` with extra steps?" · "Where does my state actually live?" · "Do secrets ever leave my machine?" · "Can two shells write at once?" · "What if I lose the key?" |

## 2. Positioning statement (Dunford form)

> For developers who live in direnv but have outgrown flat export lines,
> direnv-config (`dc`) is a YAML-backed configuration layer that replaces sprawling
> `.envrc` files with named, layered configs — deep-merged Elixir-style and flattened
> into env vars automatically. Unlike dotenv files and hosted secrets vaults, dc's
> state lives in versioned files that any process can read or write, with encrypted
> secrets, timestamped history, and remote sync only when you ask for it.

## 3. Category POV (for launch posts, not the page)

- **Enemy:** the flat-string config tax — the assumption that per-project config must be export lines in `.envrc`, unversioned and invisible to every other process.
- **Shift:** config is state, and state belongs in files. Files are the one IPC channel a child process and its parent shell both already have.
- **New way:** declare config as data (YAML), let a resolver handle layering and flattening, and let every process on the box participate.
- **Proof:** `dc_set tab status "deploying"` from a deploy script; the parent shell — and its terminal tab title — pick it up on the next prompt.

## 4. Message hierarchy

| Level | Copy |
|---|---|
| **Tagline** (≤7 words) | YAML in. Env vars out. |
| **One-liner** | direnv-config replaces sprawling `export VAR=value` `.envrc` files with named, layered YAML configs — deep-merged and flattened into environment variables. |
| **Paragraph** | `dc` is a Rust CLI plus a direnv stdlib extension. Projects declare named configs in a one-liner `.envrc`; values live as layered YAML (`base.yaml` → env overlay → `local.yaml` → `secrets.yaml`) under `~/.local/state/direnv-config/`, deep-merged and flattened into env vars. Because state lives in files, any process — parent, child, sibling, a coding agent — can read or mutate shared config; every write bumps a version counter and snapshots to `history/`. Secrets are first-class: gitignored `secrets.yaml`, XChaCha20-Poly1305-encrypted values, generated placeholders, bidirectional Infisical sync. |
| **Benefit pillars** | One-liner `.envrc`, structured YAML · File-based state is IPC · Secrets, first-class |
| **Proof points** | 22 subcommands · 148 CLI tests plus five SDK suites · published THREAT-MODEL.md and TEST-HEALTH.md · runnable `demo/` walkthrough |

## 5. Landing-page copy block (final copy, implemented verbatim)

### SEO head

- **title:** `direnv-config — YAML in. Env vars out.`
- **meta description:** `direnv-config (dc) replaces sprawling export VAR=value .envrc files with named, layered YAML configs — deep-merged and flattened into env vars, with versioned history, file-based IPC, and encrypted-at-rest secrets. Rust CLI, MIT licensed.`

### Hero

- **Eyebrow:** `YAML-backed config layer for direnv · Rust · MIT`
- **Headline:** `YAML in. Env vars out.`
- **Subhead:** `direnv-config replaces sprawling export VAR=value .envrc files with named, layered
  YAML configs — deep-merged and flattened into environment variables. Because state lives in
  files, any process can read or write it: parent shell, child script, sibling tab, a coding agent.`
- **Primary CTA:** button `Install from source` → `#install`; adjacent copy-paste block:

  ```bash
  # Prerequisites: direnv hooked into your shell, plus a Rust toolchain
  git clone https://github.com/the-robot-lives/direnv-config && cd direnv-config

  make compile            # cargo build --release
  make test
  make install            # dc binary + direnv stdlib + shell hook
  make install-completions
  make doctor             # verify the installation
  ```

- **Install honesty line (directly under the block):** `dc is not on crates.io or Homebrew yet —
  it installs from source. Packaged installs are planned.`
- **CTA fine print:** `Local-first state · secrets encrypted at rest · MIT licensed`
- **Secondary CTA:** `How it works` → `#how`
- **Then-in-a-project block (README Getting Started, neutralized):**

  ```bash
  dc_yaml cluster <<'YAML'
  name: acme
  kubeconfig: ~/.kube/acme/config
  YAML
  dc_export
  ```

### Benefit blocks ×3

1. **One-liner `.envrc`, structured YAML.** `.envrc` files grow 50+ export lines. direnv-config
   replaces them with a single `dc_yaml` heredoc per named config. Values are nested keys in
   layered YAML — `base.yaml` → env overlay → `local.yaml` → `secrets.yaml` — deep-merged
   Elixir-style, then flattened into env vars by `_dc` rules.
2. **File-based state is IPC.** Env vars are write-once from a parent process's view — a child
   can't update them. dc's state lives in files, so any process (parent, child, sibling, a coding
   agent, a terminal-tab script) can read or mutate shared config. Every write bumps a monotonic
   `.version` counter and snapshots to `history/`; a `precmd` hook reloads the shell on change.
3. **Secrets, first-class.** A separate gitignored `secrets.yaml`; values encrypted at rest
   (XChaCha20-Poly1305, key in a 0600 lockable file); generated placeholder secrets; masked
   output by default with audited reveal; bidirectional Infisical sync that compares digests,
   never plaintext.

### Why table (the README's own problem/fix table)

Section heading: **The `.envrc` you have vs. the one you could have**

| `.envrc` today | direnv-config fix |
|---|---|
| `.envrc` files grow 50+ export lines | One-liner `.envrc` loading structured YAML |
| No hierarchy or grouping | Named configs with nested keys |
| Child processes can't update parent config | Edit the YAML; parent reads next access |
| No layering (dev vs prod vs local) | Elixir-style deep merge across layer files |
| Secrets mixed with plain config | Separate `secrets.yaml` (gitignored) |
| No history | Timestamped snapshots on every write |
| No sharing with external state (tab titles, etc.) | `dc_set tab status "deploying"` — file-based IPC |

### How it works (3 steps)

1. **Declare configs in `.envrc`** — one `dc_yaml <name> <<'YAML'` heredoc per named config, then
   `dc_export`. Stores form an inheritance chain down the directory tree: child directories
   deep-merge their ancestors' configs, ancestor-first.
2. **dc resolves layers into state** — each config lives under
   `~/.local/state/direnv-config/<path-hash>/` with its layer files, a resolved `.active` merge,
   a `.version` counter, and `history/`. Every mutating call regenerates `.active` and writes a
   timestamped snapshot; `_dc_prune` tombstones cut inheritance where you don't want it.
3. **Every shell sees the change** — a `precmd` hook watches `.version` and reloads the
   environment on the next prompt. Read-modify-write commands take an advisory `flock`, so
   concurrent shells can't clobber each other's writes. `dc set` from a deploy script updates
   the parent shell's config — and its terminal tab title.

### CLI surface + trust

Section heading: **22 subcommands, one small binary** · sub-line: `Rust, built with clap; zsh and
bash completions ship in the box.`

```bash
dc yaml <name>                # deep-merge YAML from stdin (--replace, --replace-key, --if-missing, --layer)
dc get / set / unset / bump   # read values; mutate values and the version counter
dc list / status / env        # configs, merge state, resolved environment
dc gen / gen-secrets          # generate values and placeholder secrets
dc encrypt / decrypt / secrets  # secret material; masked by default, reveal is audited
dc bat                        # browse configs with secrets masked, regex filters
dc infisical / compare / push # sync against Infisical — digests, never plaintext
dc keys / config / init       # key file, tool configuration, scaffold a store
dc prune / purge              # hygiene: tombstone inheritance, remove state
```

Trust cards ×3:

1. **A published threat model.** `docs/THREAT-MODEL.md` walks a STRIDE register — store tampering,
   key exfiltration, log disclosure — and states the residual risk plainly: dc trusts same-user
   processes; the 0600 key file raises the bar, it is not a security boundary.
2. **A published test-health report.** 148 CLI tests plus five SDK suites (Rust, TypeScript,
   Python, Elixir, PHP) kept aligned by shared contract fixtures — measured 2026-10-07 in
   `docs/TEST-HEALTH.md`.
3. **Honest defaults.** Secret values are masked in output by default; reveals are written to a
   0600 audit log; remote sync compares digests, not plaintext; nothing leaves the machine
   without an explicit sync command.

### FAQ (objection row)

1. **Is this just `.env` files with extra steps?** No. Named configs give you grouping and nested
   keys; layers give you dev/prod/local overrides without duplication; the parent chain gives you
   inheritance down the directory tree; every write is versioned and snapshotted. Plain exports
   give you none of that.
2. **Where does my config actually live?** Your `.envrc` stays the source of truth — the
   `dc_yaml` heredocs are it. Resolved state lives under `~/.local/state/direnv-config/`
   (one directory per project path hash), with the merged `.active` snapshot, the `.version`
   counter, and `history/`.
3. **Can two shells write at the same time?** Yes. Every read-modify-write command takes an
   advisory `flock` on the store, so simultaneous invocations can't lose writes or version bumps.
   A layer file that exists but fails to parse is refused, not silently replaced.
4. **Do secrets ever leave my machine?** Only when you run an explicit sync command
   (`dc infisical`, `dc compare`, `dc push`). Comparison uses digests — plaintext never crosses
   the wire — and the default output path masks secret values everywhere.
5. **What if I lose the machine holding the key?** Encrypted values are unrecoverable without
   the key file — back it up. `dc keys status | lock | unlock | migrate` manages the key file;
   legacy keys embedded in `settings.yaml` are still read transparently.
6. **How do I install it?** Clone the repo and `make install` — see the box above. You need
   direnv hooked into your shell and a Rust toolchain. It is not on crates.io or Homebrew yet;
   packaged installs are planned.
7. **What's the license?** MIT. Fork it, read the threat model, run the demo.

### Final CTA

`YAML in. Env vars out.` · button `Install from source` → `#install` (same code block).

### Footer

`direnv-config — a noizu.com project` · GitHub: `github.com/the-robot-lives/direnv-config` ·
`MIT licensed` · `This site is Elixir Hologram.`

---

## 6. Repo-verifiable claims (all against commit `3190560`)

| # | Claim on page | Verified in |
|---|---|---|
| 1 | One-liner `.envrc`; layered YAML deep-merged into env vars | `README.md`, `docs/PROJ-ARCH.summary.md` |
| 2 | Layer order `base.yaml → {DC_ENV}.yaml → local.yaml → secrets.yaml → .active`; parent-chain merge ancestor-first | `docs/PROJ-SCHEMA.summary.md` |
| 3 | State at `~/.local/state/direnv-config/<path-hash>/` with `.active`, `.version`, `history/` | `README.md`, `docs/PROJ-SCHEMA.summary.md` |
| 4 | Every write bumps a monotonic version counter + timestamped snapshot | `README.md`, `docs/PROJ-SCHEMA.summary.md` |
| 5 | File-based IPC via `precmd` hook watching `.version` | `docs/PROJ-ARCH.summary.md`, `bin/dc-init` |
| 6 | Secrets: `secrets.yaml` gitignored layer, `🔒:v1:` tokens, XChaCha20-Poly1305, 0600 lockable key file | `docs/PROJ-SCHEMA.summary.md`, `Cargo.toml` (chacha20poly1305) |
| 7 | Masked output by default; audited reveal (0600 audit log); digest-based remote comparison | `docs/PROJ-ARCH.summary.md`, `docs/PROJ-SCHEMA.summary.md`, `docs/THREAT-MODEL.summary.md` |
| 8 | Infisical sync `get|set|compare`; `k8://` address scheme also exists | `README.md`, `docs/PROJ-SCHEMA.summary.md` |
| 9 | `_dc_prune`/tombstone inheritance control | `docs/PROJ-ARCH.summary.md` |
| 10 | 22 visible subcommands | `src/main.rs` Commands enum (23 variants; `__complete-purge` hidden) |
| 11 | `flock` around every read-modify-write; corrupt layers refused not replaced | `CHANGELOG.md` [m8-session-safety-hardening] |
| 12 | Five SDKs (TS, Python, Elixir, PHP read-only; Rust read+write) with shared contract tests | `docs/PROJ-ARCH.summary.md` |
| 13 | 148 CLI tests; Rust SDK 71+1; Python 114; Elixir 89 (measured 2026-10-07) | `docs/TEST-HEALTH.md` |
| 14 | Published STRIDE threat model with stated residual risk (same-user trust) | `docs/THREAT-MODEL.summary.md` |
| 15 | Runnable demo tree with expected-state fixtures | `demo/README.md` |
| 16 | Install = clone + `make compile/test/install/install-completions/doctor`; prereqs direnv + Rust | `README.md`, `Makefile` |
| 17 | Not on crates.io or Homebrew; no one-liner installer exists | `Makefile` (install = local copy), `docs/TEST-HEALTH.md` (no CLI publish job) |
| 18 | MIT | `Cargo.toml`, `LICENSE` |
| 19 | Generated placeholder secrets (`🎲`) | `docs/PROJ-SCHEMA.summary.md`, `src/main.rs` GenSecrets |
| 20 | zsh + bash completions ship in the box | `docs/PROJ-ARCH.summary.md`, `Makefile` install-completions |

## Voice rules

- Dev-tool, example-led, zero fluff. Every claim names a mechanism or a file.
- No invented metrics, no testimonials, no users logos. Test counts cite TEST-HEALTH.md's measured date.
- Never oversell install: it is clone + make, prerequisites stated up front, "planned" stays planned.
- Honesty edge: same-user trust is a real boundary — the page says so instead of implying the key
  file is a security boundary. dc also evaluates shell env (`dc env`); `direnv allow` remains the gate.
- README inaccuracies corrected in this PR (user-requested): the Sphinx/ReadTheDocs claim is gone
  (no conf.py/index.rst in repo) and the CLI list now shows the real 22 subcommands. The page never
  repeated either error (`secretsmap`/`secretcmp`/`audit`/`settings` are internal modules, not
  subcommands).
