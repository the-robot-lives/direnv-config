import { execFile } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { parse as parseYaml } from 'yaml';
import type { Backend } from './types.js';

function exec(binary: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(binary, args, (error, stdout, stderr) => {
      if (error) {
        reject(new Error(`dc command failed: ${stderr || error.message}`));
        return;
      }
      resolve(stdout);
    });
  });
}

export class CliBackend implements Backend {
  constructor(
    private storePath: string,
    private dcBinary: string
  ) {}

  async get(config: string, path?: string): Promise<unknown> {
    const args = ['get', config];
    if (path) args.push(path);
    args.push('--raw');
    const output = await exec(this.dcBinary, args);
    return parseYaml(output);
  }

  async listConfigs(): Promise<string[]> {
    const metaPath = join(this.storePath, '.meta');
    const content = await readFile(metaPath, 'utf-8');
    const meta = parseYaml(content);
    return meta?.configs ?? [];
  }
}
