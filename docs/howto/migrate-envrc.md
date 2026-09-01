# How to: migrate a raw `export`-heavy `.envrc` to dc

**Goal:** replace 20-50 lines of `export VAR=value` with a structured, named YAML config that `dc` manages.
**Prereqs:** `direnv-config` installed ([first-hour.md](first-hour.md)); the project's existing `.envrc`.

1. From the project directory, import the current exports into a store without touching the file yet:
   ```bash
   dc init --from-envrc .envrc
   ```
   This parses every `export KEY=VALUE` line in `.envrc` into a single named config called `imported` (its `base.yaml`), keys kept as-is.
2. Inspect what was imported:
   ```bash
   dc bat --all
   ```
3. Group related keys into purpose-named configs (the importer dumps everything flat into `imported` — reorganize by concern, e.g. `cluster`, `cloudflare`, `build`). Re-merge with proper grouping, then prune the flat `imported` config once everything is regrouped:
   ```bash
   dc yaml cluster <<'YAML'
   name: noizu
   kubeconfig: ~/.kube/noizu/config
   YAML
   ```
4. Add flatten rules so the original env var names still resolve (only needed where the var name doesn't match the `{CONFIG}_{KEY}` auto-derive convention):
   ```bash
   dc yaml _dc <<'YAML'
   flatten:
     cluster.kubeconfig: KUBECONFIG
   YAML
   ```
5. Drop the now-redundant flat import:
   ```bash
   dc prune imported
   ```
6. Replace the old `.envrc` body with the new form and re-approve:
   ```bash
   cat > .envrc <<'EOF'
   dc_yaml cluster <<'YAML'
   name: noizu
   kubeconfig: ~/.kube/noizu/config
   YAML

   dc_yaml _dc <<'YAML'
   flatten:
     cluster.kubeconfig: KUBECONFIG
   YAML

   dc_export
   EOF
   direnv allow
   ```

**Verify:**
```bash
dc get cluster                 # confirm the new grouping resolved correctly
diff <(env | sort) <(env | sort)   # or just: echo $KUBECONFIG, confirm unchanged value
```

**Gotchas:**
- Secrets imported by `--from-envrc` land in `base.yaml` unencrypted — move sensitive keys to `secrets.yaml` (`dc set --layer secrets ...` or re-tag with `🔒` and re-merge) before committing `base.yaml` to git.
- If a var name doesn't match `{CONFIG_NAME}_{KEY}`, it silently won't export unless you add an explicit `flatten` rule (see [PROJ-ARCH.md](../PROJ-ARCH.md) Flatten Rules).
- Nested directories (subprojects) should use `source_up` + inherit, not re-import — see the Parent Chain section in the README rather than running `--from-envrc` again lower in the tree.
