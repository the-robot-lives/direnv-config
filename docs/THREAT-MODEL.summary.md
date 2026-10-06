# Threat Model — Summary

Workstation-local config/secrets tooling. Crown jewels: the XChaCha20-Poly1305
key file (0600, root-lockable), encrypted `secrets.yaml`/shadow stores, and the
evaluated shell env (`dc env` is `eval`'d). Main boundary: no privilege
separation between `dc` and other same-user processes. Full reference:
[THREAT-MODEL.md](THREAT-MODEL.md). Structure only — never values.

## Register at a glance

| ID | Severity | STRIDE | Status |
|----|----------|--------|--------|
| T-001 | High | Tampering/EoP — store writes → env injection | Partial (same-user trust) |
| T-002 | Critical | Info disclosure — key exfiltration | Mitigated (key file 0600, `dc keys lock/migrate`) |
| T-003 | Medium | EoP — `.envrc` is shell by design | Accepted (`direnv allow` gate) |
| T-004 | Medium | Info disclosure — output/logs | Mitigated (redaction default, digest compare) |
| T-005 | Medium | Repudiation — reveal attribution | Partial (0600 audit log, user-deletable) |
| T-006 | Medium | Info disclosure — accidental commits | Partial (gitignored; shadow store in-tree) |
| T-007 | Low | Spoofing/tampering — remote backends | Mitigated (rustls TLS, digests) |
| T-008 | Low | Tampering — supply chain | Partial (lockfile; no signing) |

## Residual risk

Same-user compromise = game over (accepted for a single-operator tool);
root-owned key file raises the bar for non-root exfiltration only.
