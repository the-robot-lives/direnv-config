export type DcMode = 'native' | 'cli';

export interface DcClientOptions {
  mode?: DcMode;
  directory?: string;
  stateDir?: string;
  dcBinary?: string;
}

export interface Backend {
  get(config: string, path?: string): Promise<unknown>;
  listConfigs(): Promise<string[]>;
}
