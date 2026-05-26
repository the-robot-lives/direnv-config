import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { parse as parseYaml } from 'yaml';
import type { Backend } from './types.js';
import { getPath } from './path.js';

export class NativeBackend implements Backend {
  constructor(private storePath: string) {}

  async get(config: string, path?: string): Promise<unknown> {
    const activePath = join(this.storePath, config, '.active');
    const content = await readFile(activePath, 'utf-8');
    const root = parseYaml(content);

    if (!path) return root;
    return getPath(root, path);
  }

  async listConfigs(): Promise<string[]> {
    const metaPath = join(this.storePath, '.meta');
    const content = await readFile(metaPath, 'utf-8');
    const meta = parseYaml(content);
    return meta?.configs ?? [];
  }
}
