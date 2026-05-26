import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

export async function readVersion(storePath: string): Promise<number> {
  try {
    const content = await readFile(join(storePath, '.version'), 'utf-8');
    const parsed = parseInt(content.trim(), 10);
    return Number.isNaN(parsed) ? 0 : parsed;
  } catch {
    return 0;
  }
}

export function watchVersion(
  storePath: string,
  callback: (version: number) => void,
  intervalMs = 1000
): { dispose(): void } {
  let lastVersion = -1;

  const timer = setInterval(async () => {
    const version = await readVersion(storePath);
    if (version !== lastVersion) {
      lastVersion = version;
      callback(version);
    }
  }, intervalMs);

  return {
    dispose() {
      clearInterval(timer);
    },
  };
}
