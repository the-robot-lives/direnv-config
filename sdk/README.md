# direnv-config SDK Libraries

Read-only client libraries for [direnv-config (`dc`)](../README.md) in four languages. Each SDK reads YAML-backed configuration from `dc` stores, providing idiomatic access to config values from application code.

## SDKs

| Language | Package | Install |
|----------|---------|---------|
| TypeScript | `@noizu/direnv-config` | `npm install @noizu/direnv-config` |
| Python | `noizu-direnv-config` | `pip install noizu-direnv-config` |
| Elixir | `:direnv_config` | `{:direnv_config, "~> 0.1.0"}` in `mix.exs` |
| PHP | `noizu/direnv-config` | `composer require noizu/direnv-config` |

## Quick Start

All four SDKs share the same API surface:

### TypeScript

```typescript
import { DcClient } from '@noizu/direnv-config';

const dc = new DcClient({ directory: '/path/to/project' });
const name = await dc.getString('cluster', 'name');
const minNodes = await dc.getInt('cluster', 'node_pool.min');
const hosts = await dc.get('app', 'endpoints[*].host');
```

### Python

```python
from direnv_config import DcClient

dc = DcClient(directory='/path/to/project')
name = dc.get_string('cluster', 'name')
min_nodes = dc.get_int('cluster', 'node_pool.min')
hosts = dc.get('app', 'endpoints[*].host')
```

### Elixir

```elixir
client = DirenvConfig.Client.new(directory: "/path/to/project")
name = DirenvConfig.Client.get_string(client, "cluster", "name")
min_nodes = DirenvConfig.Client.get_int(client, "cluster", "node_pool.min")
hosts = DirenvConfig.Client.get(client, "app", "endpoints[*].host")
```

### PHP

```php
use Noizu\DirenvConfig\DcClient;

$dc = new DcClient(directory: '/path/to/project');
$name = $dc->getString('cluster', 'name');
$minNodes = $dc->getInt('cluster', 'node_pool.min');
$hosts = $dc->get('app', 'endpoints[*].host');
```

## Backends

Each SDK supports two backends:

- **Native** (default) -- reads `.active` YAML files directly from the dc store. Fast, no subprocess overhead.
- **CLI** -- shells out to the `dc` binary. Supports parent-chain resolution and stays in sync with CLI behavior.

```typescript
const dc = new DcClient({ mode: 'cli', dcBinary: '/usr/local/bin/dc' });
```

## Path Expressions

All SDKs support the same path expression syntax:

| Expression | Description | Example |
|-----------|-------------|---------|
| `key` | Map key | `name` |
| `a.b.c` | Nested keys | `node_pool.min` |
| `a[0]` | Array index | `endpoints[0].host` |
| `a[-1]` | Negative index | `endpoints[-1].host` |
| `a[*].b` | Wildcard (collect) | `endpoints[*].host` |
| `a.length` | Count | `endpoints.length` |
| `a[0][1]` | Chained brackets | `matrix[0][1]` |

## Version Watching

SDKs can poll the store's `.version` file to detect config changes (IPC):

```typescript
const watcher = dc.watch((version) => {
  console.log(`Config updated to version ${version}`);
}, 1000);

// Later: watcher.dispose();
```

## Contract Tests

The `contract-tests/` directory contains shared fixtures and `expectations.yaml` defining ~30 test cases that every SDK must pass, ensuring behavioral parity across all four implementations.

## Running Tests

```bash
# TypeScript
cd sdk/typescript && npm test

# Python
cd sdk/python && pip install -e ".[dev]" && pytest

# Elixir
cd sdk/elixir && mix deps.get && mix test

# PHP
cd sdk/php && composer install && vendor/bin/phpunit
```
