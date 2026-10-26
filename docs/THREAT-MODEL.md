# Threat Model

〚📁:f3d31648-73c7-5d34-b97d-be7278edb2de〛 direnv-config THREAT-MODEL :: Living threat model: trust boundaries, vulnerability register, mitigations

> Security counterpart to [PROJ-ARCH.md](PROJ-ARCH.md). Workstation-local config
> and secrets tooling: the assets are secret material and the shell environments
> `dc` populates; the adversary model is (a) malicious/compromised same-host
> processes, (b) accidental disclosure (output, commits, logs), (c) tampering in
> transit or at rest with remote secret backends. No secret values appear here.

## Overview

`direnv-config` holds the scalar config and credential layer for the fleet
(`.envrc.k8.dc`: AWS, Docker, Helm, Infisical). Crown jewels, in order: the
XChaCha20-Poly1305 **key file** (`~/.config/direnv-config/keys`, 0600, root-lockable
via `dc keys lock`) — it decrypts every `🔒:v1` token; the encrypted
`secrets.yaml` layers and shadow stores; and the **evaluated shell environment** —
`dc env` output is `eval`'d by `.envrc`, so anything that can write the store can
inject env vars (and thereby shell behavior) into every project directory.

Trust boundaries: (1) same-user processes ↔ store/settings/audit files (no
privilege separation — the key file's root-ownership lock is the one exception);
(2) shell ↔ `.envrc`/direnv stdlib (code execution by design, gated by `direnv
allow`); (3) CLI ↔ remote backends (Infisical API over rustls TLS, kubectl);
(4) repo ↔ local-only artifacts (`secrets.yaml`, `local.yaml`, shadow store,
`.secrets/` are gitignored — accidental commit is a live disclosure path).

## Attack Surface

```mermaid
graph LR
    PROC["local processes<br/>(any same-user)"] -->|writes local.yaml / .version| STORE[("state store<br/>~/.local/state/direnv-config/")]
    ENVRC[".envrc (eval'd)"] -->|dc_yaml / dc_export| CLI["dc CLI"]
    STORE -->|resolve| ACTIVE[".active"]
    CLI -->|dc env: export lines| SHELL["parent shell env"]
    CLI -->|--reveal / --clippy| AUDIT[("audit.log 0600")]
    KEYFILE[("keys 0600<br/>root-lockable")] -->|decrypt| CLI
    CLI -->|digest compare/push| INF[Infisical API]
    CLI -->|kubectl| K8S[k8s Secrets]
    SECMAP[".infisical-secrets.yaml<br/>(committed)"] --> CLI
    SDKS["SDKs (5 langs)"] -->|read .active/layers| STORE
```

## Vulnerability Register

| ID | Severity | STRIDE | Component | Status |
|----|----------|--------|-----------|--------|
| T-001 | High | Tampering / EoP | Store files writable by any same-user process → env-var injection into every shell that evals `dc env` (e.g. `LD_PRELOAD`, `PATH`) | Partial — single-user workstation trust assumption; no MAC/signature on store files |
| T-002 | Critical | Info disclosure | AEAD key exfiltration decrypts all tokens | Mitigated — dedicated key file, 0600 enforced, `dc keys lock` makes it root-owned read-only; loud failure on rotation. Legacy `settings.yaml` key is weaker → `dc keys migrate` |
| T-003 | Medium | EoP | `.envrc` is arbitrary shell by design (direnv model) | Accepted — `direnv allow` re-confirmation gate; `dc_yaml` heredocs keep most content data-only |
| T-004 | Medium | Info disclosure | Secret values reaching terminals, logs, CI output | Mitigated — redaction by default; reveal requires explicit `--reveal`/`--clippy`; remote compare uses SHA-256 digests (no plaintext/length leak); `dc get --keys` reads shape only (no decryption at all) |
| T-005 | Medium | Repudiation | Reveals must be attributable | Partial — append-only 0600 audit log records every reveal (sections reveal per leaf too), but the log is user-deletable (no external sink) |
| T-006 | Medium | Info disclosure | Accidental commit of `secrets.yaml` / `local.yaml` / `<project>/.secrets/restricted.config.yaml` | Partial — gitignored by convention; shadow store is 0600 but lives inside the project tree; tokens stay ciphertext without T-002 key; IPC writes (`dc set`/`dc config set`) now encrypt **by default**, so a write can no longer leave a plaintext value behind unless `--plain` is asked for |
| T-007 | Low | Spoofing / Tampering | Remote backends: Infisical API creds, kubectl context | Mitigated — rustls TLS (no native TLS CA issues), creds resolved from env/config; digest comparison prevents value overwrite guessing |
| T-008 | Low | Tampering | Supply chain: Cargo deps, SDK publishing (`publish-sdks.yml`) | Partial — lockfile pinned; standard CI trust; no artifact signing |
| T-009 | High | Info disclosure | Write-preview output (set/setall/unset and generated values) spilling sibling secrets into terminals and agent transcripts | Mitigated — previews render every non-target line structure-only and mask the target's old value (#47); `--generate` never echoes the minted value (#43); `--reveal`/`--keys`/`--raw`/`--clippy` flag conflicts are rejected rather than resolved silently (#44) |

## Mitigation Coverage

4 mitigated · 4 partial · 1 accepted. The partial items share one root cause:
**no privilege boundary between `dc` and other same-user processes**
(workstation trust assumption, T-001/T-005/T-006/T-008 all reduce to it).
T-009 is the one register entry whose attack path is purely a display surface —
`dc` printing more than it must — and it is closed in code rather than accepted:
every value-bearing output has a mask or a shape-only mode.

## Residual Risk

Same-user compromise = game over (key read or store write) — accepted for a
single-operator dev tool; the root-owned key file (`dc keys lock`) raises the bar
for non-root exfiltration only. Audit log is best-effort forensics, not evidence.
Preview masking lowers accidental exposure (terminals, transcripts, pasted
logs) but is not an access control: the underlying file is still readable by
the same user. `🎲` auto-generation never happens implicitly in `dc infisical`
paths, so secrets are never silently minted during sync.
