import { describe, it, expect } from 'vitest';
import { resolve } from 'node:path';
import { NativeBackend } from '../src/native.js';

const FIXTURES = resolve(import.meta.dirname, '../../contract-tests/fixtures');

describe('NativeBackend', () => {
  describe('simple-store', () => {
    const backend = new NativeBackend(resolve(FIXTURES, 'simple-store'));

    it('reads a string value', async () => {
      expect(await backend.get('cluster', 'name')).toBe('noizu');
    });

    it('reads a nested string', async () => {
      expect(await backend.get('cluster', 'node_pool.instance_type')).toBe('m5.xlarge');
    });

    it('reads an integer', async () => {
      expect(await backend.get('cluster', 'port')).toBe(6443);
    });

    it('reads a boolean', async () => {
      expect(await backend.get('cluster', 'enabled')).toBe(true);
    });

    it('reads entire config as object', async () => {
      const result = await backend.get('cluster');
      expect(result).toBeTypeOf('object');
      expect(result).toHaveProperty('name');
      expect(result).toHaveProperty('node_pool');
    });

    it('returns null for missing path', async () => {
      expect(await backend.get('cluster', 'nonexistent')).toBeNull();
    });

    it('lists configs', async () => {
      const configs = await backend.listConfigs();
      expect(configs).toEqual(['cluster']);
    });
  });

  describe('nested-store', () => {
    const backend = new NativeBackend(resolve(FIXTURES, 'nested-store'));

    it('reads array element by index', async () => {
      expect(await backend.get('app', 'endpoints[0].host')).toBe('api.example.com');
    });

    it('reads with negative index', async () => {
      expect(await backend.get('app', 'endpoints[-1].host')).toBe('backup.example.com');
    });

    it('reads with wildcard', async () => {
      expect(await backend.get('app', 'endpoints[*].host')).toEqual([
        'api.example.com',
        'internal.example.com',
        'backup.example.com',
      ]);
    });

    it('reads length', async () => {
      expect(await backend.get('app', 'endpoints.length')).toBe(3);
    });

    it('reads chained brackets', async () => {
      expect(await backend.get('app', 'matrix[0][1]')).toBe(2);
    });

    it('lists configs', async () => {
      const configs = await backend.listConfigs();
      expect(configs).toEqual(['app']);
    });
  });
});
