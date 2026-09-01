# How to: read (and write) `dc` config from app code without shelling out

**Goal:** call `dc`-managed config directly from Rust, TypeScript, Python, Elixir, or PHP application code — no subprocess, no parsing CLI output.
**Prereqs:** a store already initialized in the target directory ([first-hour.md](first-hour.md)); the language's package manager.

1. Install the SDK for your language:

   ```bash
   cargo add direnv-config                       # Rust
   npm install @noizu/direnv-config               # TypeScript
   pip install noizu-direnv-config                # Python
   composer require noizu/direnv-config           # PHP
   # Elixir: add {:direnv_config, "~> 0.1.0"} to mix.exs, then `mix deps.get`
   ```

2. Create a client and read a value (TypeScript shown; Rust/Python/Elixir/PHP share the same method names):

   ```typescript
   import { DcClient } from '@noizu/direnv-config';

   const dc = new DcClient({ directory: '/path/to/project' });
   const name = await dc.getString('cluster', 'name');
   const hosts = await dc.get('app', 'endpoints[*].host');
   ```

3. Write a value the same way `dc set` would (defaults to the `local` layer):

   ```typescript
   await dc.set('cluster', 'node_pool.min', '4');
   await dc.unset('cluster', ['deprecated_key']);
   ```

**Verify:** `dc get cluster node_pool.min` from a shell in the same directory reflects the SDK's write; `dc bat cluster` shows it in the resolved chain.

**Gotchas:**
- Two backends exist per SDK: **native** (default — reads/writes YAML files directly, no `dc` binary needed) and **cli** (shells out to `dc`, needed only if you rely on parent-chain resolution behavior identical to the CLI). Pick CLI mode explicitly if native gives unexpected results: `new DcClient({ mode: 'cli', dcBinary: '/usr/local/bin/dc' })`.
- All read-only SDKs pass encrypted-secret scalars (`🔒:v1:...`) through untouched — none of them decrypt; don't expect `dc.get()` to hand back plaintext for a tagged secret.
- Path expressions (`a.b.c`, `a[0]`, `a[*].b`, `a.length`) work identically across all five SDKs and match the CLI's `dc get` syntax — but wildcard/`length` reads are read-only, not writable.
